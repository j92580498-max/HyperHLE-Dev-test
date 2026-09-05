//! The OpenFeint dashboard: a touchHLE app-picker screen (in the spirit of
//! the original OpenFeint app) that lists every installed game with locally
//! tracked OpenFeint achievements and high scores, and shows a per-game
//! stats page.
//!
//! Data sources:
//! - per-game achievement/high-score stores written by the OpenFeint HLE
//!   layer (`touchHLE_sandbox/<bundle id>/AppSandbox/Library/OpenFeint/`),
//! - each app bundle's `openfeint_offline_config.xml` (when present) for the
//!   real achievement and leaderboard titles.

use crate::frameworks::foundation::NSInteger;
use crate::environment::app_picker::AppInfo;
use crate::frameworks::core_graphics::cg_image;
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string;
use crate::frameworks::uikit::ui_view::ui_control::ui_button::UIButtonTypeCustom;
use crate::frameworks::uikit::ui_view::ui_control::{
    UIControlEventTouchUpInside, UIControlStateNormal,
};
use crate::frameworks::uikit::ui_font::{UITextAlignment, UITextAlignmentCenter, UITextAlignmentLeft};
use crate::image::Image;
use crate::objc::{id, msg, msg_class, nil};
use crate::paths;
use crate::Environment;

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

const OF_DARK_BG: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.09, 0.09, 0.11, 1.0);
const OF_CARD_BG: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.15, 0.16, 0.20, 1.0);
const OF_ACCENT: (CGFloat, CGFloat, CGFloat, CGFloat) = (1.0, 0.62, 0.10, 1.0);
const OF_TEXT: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.93, 0.93, 0.90, 1.0);
const OF_DIM_TEXT: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.55, 0.57, 0.60, 1.0);
const LINES_PER_PAGE: usize = 14;

pub struct GameStats {
    pub app_idx: usize,
    pub app_path: PathBuf,
    pub display_name: String,
    /// `(id, title, gamerscore)` from the offline config, if available.
    pub ach_defs: Vec<(String, String, f32)>,
    /// `(id, percent)` of every locally tracked achievement.
    pub ach_state: Vec<(String, f32)>,
    /// `(leaderboard title or id, best formatted score)`.
    pub scores: Vec<(String, String)>,
}

pub struct OfDashboardStuff {
    pub of_main_view: id,
    pub detail_view: id,
    pub detail_text: id,
    pub page_label: id,
}

fn color(env: &mut Environment, rgba: (CGFloat, CGFloat, CGFloat, CGFloat)) -> id {
    let (r, g, b, a) = rgba;
    msg_class![env; UIColor colorWithRed:r green:g blue:b alpha:a]
}

fn make_label(
    env: &mut Environment,
    frame: CGRect,
    text: &str,
    font_size: CGFloat,
    rgba: (CGFloat, CGFloat, CGFloat, CGFloat),
    alignment: crate::frameworks::uikit::ui_font::UITextAlignment,
) -> id {
    let label: id = msg_class![env; UILabel alloc];
    let label: id = msg![env; label initWithFrame:frame];
    let ns_text = ns_string::from_rust_string(env, text.to_owned());
    () = msg![env; label setText:ns_text];
    () = msg![env; label setTextAlignment:alignment];
    let font: id = msg_class![env; UIFont boldSystemFontOfSize:font_size];
    () = msg![env; label setFont:font];
    let _of_c = color(env, rgba);
    () = msg![env; label setTextColor:_of_c];
    let _of_c = color(env, (0.0, 0.0, 0.0, 0.0));
    () = msg![env; label setBackgroundColor:_of_c];
    () = msg![env; label setNumberOfLines:0];
    label
}

fn make_button(
    env: &mut Environment,
    delegate: id,
    frame: CGRect,
    title: &str,
    action_sel: &str,
    tag: NSInteger,
    font_size: CGFloat,
) -> id {
    let button: id = msg_class![env; UIButton buttonWithType:UIButtonTypeCustom];
    () = msg![env; button setFrame:frame];
    () = msg![env; button setTag:tag];
    let ns_title = ns_string::from_rust_string(env, title.to_owned());
    () = msg![env; button setTitle:ns_title forState:UIControlStateNormal];
    () = msg![env; button layoutSubviews];
    let _of_c = color(env, OF_CARD_BG);
    () = msg![env; button setBackgroundColor:_of_c];
    let font: id = msg_class![env; UIFont boldSystemFontOfSize:font_size];
    () = msg![env; button setFont:font];
    let _of_c = color(env, OF_TEXT);
    () = msg![env; button setTitleColor:_of_c
                              forState:UIControlStateNormal];
    let sel = env.objc.lookup_selector(action_sel).unwrap();
    () = msg![env; button addTarget:delegate
                           action:sel
                 forControlEvents:UIControlEventTouchUpInside];
    button
}

fn tag_text<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(text[start..end].trim())
}

fn element_blocks<'a>(text: &'a str, element: &str) -> Vec<&'a str> {
    let open = format!("<{element}>");
    let close = format!("</{element}>");
    let mut blocks = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(&open) {
        let after_open = &rest[start + open.len()..];
        let Some(end) = after_open.find(&close) else {
            break;
        };
        blocks.push(&after_open[..end]);
        rest = &after_open[end + close.len()..];
    }
    blocks
}

fn read_offline_config(app_path: &PathBuf) -> Option<String> {
    if app_path.is_dir() {
        return std::fs::read_to_string(app_path.join("openfeint_offline_config.xml")).ok();
    }
    let file = std::fs::File::open(app_path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).ok()?;
        if entry.name().ends_with("openfeint_offline_config.xml") {
            let mut text = String::new();
            entry.read_to_string(&mut text).ok()?;
            return Some(text);
        }
    }
    None
}

fn read_tsv_lines(bundle_id: &str, file: &str) -> Vec<(String, String, String)> {
    let path = paths::user_data_base_path()
        .join(paths::SANDBOX_DIR)
        .join(bundle_id)
        .join("AppSandbox/Library/OpenFeint")
        .join(file);
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let a = parts.next()?.to_owned();
            let b = parts.next()?.to_owned();
            let c = parts.next().unwrap_or("").to_owned();
            Some((a, b, c))
        })
        .collect()
}

/// Gather OpenFeint stats for every installed app. Apps without any locally
/// tracked OpenFeint data are skipped.
pub fn collect_game_stats(apps: &mut [AppInfo]) -> Vec<GameStats> {
    let mut games = Vec::new();
    for (app_idx, app) in apps.iter_mut().enumerate() {
        let bundle_id = app.bundle_id.clone();
        let ach_state: Vec<(String, f32)> = read_tsv_lines(&bundle_id, "achievements.tsv")
            .into_iter()
            .filter_map(|(id, percent, _)| {
                percent.parse::<f32>().ok().map(|p| (id, p))
            })
            .collect();
        let scores: Vec<(String, String)> = read_tsv_lines(&bundle_id, "highscores.tsv")
            .into_iter()
            .map(|(id, score, display)| {
                let name = if display.is_empty() { id } else { display };
                (name, score)
            })
            .collect();
        if ach_state.is_empty() && scores.is_empty() {
            continue;
        }

        let mut ach_defs = Vec::new();
        if let Some(config) = read_offline_config(&app.path) {
            for block in element_blocks(&config, "achievement") {
                let Some(ach_id) = tag_text(block, "id") else {
                    continue;
                };
                let title = tag_text(block, "title").unwrap_or(ach_id).to_owned();
                let gamerscore = tag_text(block, "gamerscore")
                    .and_then(|g| g.parse::<f32>().ok())
                    .unwrap_or(0.0);
                ach_defs.push((ach_id.to_owned(), title, gamerscore));
            }
        }

        games.push(GameStats {
            app_idx,
            app_path: app.path.clone(),
            display_name: app.display_name.clone(),
            ach_defs,
            ach_state,
            scores,
        });

    }
    games
}

fn unlocked_count(game: &GameStats) -> usize {
    game.ach_state.iter().filter(|(_, p)| *p >= 100.0).count()
}

pub struct PlaqueStuff {
    buttons: Vec<id>,
    labels: Vec<id>,
}

pub fn setup(
    env: &mut Environment,
    delegate: id,
    main_view: id,
    app_frame: CGRect,
    games: &[GameStats],
) -> OfDashboardStuff {
    let frame = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: app_frame.size,
    };
    let of_main_view: id = msg_class![env; UIView alloc];
    let of_main_view: id = msg![env; of_main_view initWithFrame:frame];
    () = msg![env; of_main_view setBackgroundColor:(color(env, OF_DARK_BG))];
    () = msg![env; of_main_view setHidden:true];
    () = msg![env; main_view addSubview:of_main_view];

    let title = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 10.0, y: 8.0 },
            size: CGSize {
                width: app_frame.size.width - 20.0,
                height: 30.0,
            },
        },
        "OpenFeint",
        24.0,
        OF_ACCENT,
        UITextAlignmentCenter,
    );
    () = msg![env; main_view addSubview:title];

    let plaque_width = ((app_frame.size.width - 40.0) / 2.0).round();
    let plaque_height: CGFloat = 64.0;
    let max_rows = (((app_frame.size.height - 190.0) / (plaque_height + 12.0)) as usize).max(1);
    let max_slots = max_rows * 2;

    let mut buttons = Vec::new();
    let mut labels = Vec::new();
    for slot in 0..max_slots {
        let col = slot % 2;
        let row = slot / 2;
        let frame = CGRect {
            origin: CGPoint {
                x: (20.0 + (col as CGFloat) * (plaque_width + 12.0)).round(),
                y: (52.0 + (row as CGFloat) * (plaque_height + 12.0)).round(),
            },
            size: CGSize {
                width: plaque_width,
                height: plaque_height,
            },
        };
        let button = make_button(
            env,
            delegate,
            frame,
            "",
            "ofGameTapped:",
            (slot + 1) as NSInteger,
            15.0,
        );
        () = msg![env; main_view addSubview:button];
        buttons.push(button);
        let label = make_label(
            env,
            CGRect {
                origin: CGPoint {
                    x: frame.origin.x,
                    y: frame.origin.y + plaque_height - 2.0,
                },
                size: CGSize {
                    width: plaque_width,
                    height: 24.0,
                },
            },
            "",
            11.0,
            OF_DIM_TEXT,
            UITextAlignmentCenter,
        );
        () = msg![env; main_view addSubview:label];
        labels.push(label);
    }

    let all_apps_button = make_button(
        env,
        delegate,
        CGRect {
            origin: CGPoint { x: 20.0, y: app_frame.size.height - 90.0 },
            size: CGSize {
                width: app_frame.size.width - 40.0,
                height: 34.0,
            },
        },
        "All apps",
        "ofShowAllApps",
        0,
        17.0,
    );
    () = msg![env; main_view addSubview:all_apps_button];

    // ---- Detail view (per-game stats) ----
    let detail_view: id = msg_class![env; UIView alloc];
    let detail_view: id = msg![env; detail_view initWithFrame:frame];
    let _of_c = color(env, OF_DARK_BG);
    () = msg![env; detail_view setBackgroundColor:_of_c];
    () = msg![env; detail_view setHidden:true];
    () = msg![env; detail_view setTag:902];
    () = msg![env; main_view addSubview:detail_view];

    let back_button = make_button(
        env,
        delegate,
        CGRect {
            origin: CGPoint { x: 10.0, y: 6.0 },
            size: CGSize { width: 90.0, height: 30.0 },
        },
        "< Back",
        "ofBack",
        0,
        15.0,
    );
    () = msg![env; detail_view addSubview:back_button];

    let detail_title = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 110.0, y: 8.0 },
            size: CGSize {
                width: app_frame.size.width - 200.0,
                height: 26.0,
            },
        },
        "",
        18.0,
        OF_ACCENT,
        UITextAlignmentCenter,
    );
    () = msg![env; detail_view addSubview:detail_title];

    let detail_text = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 14.0, y: 42.0 },
            size: CGSize {
                width: app_frame.size.width - 28.0,
                height: app_frame.size.height - 160.0,
            },
        },
        "",
        12.5,
        OF_TEXT,
        UITextAlignmentLeft,
    );
    () = msg![env; detail_view addSubview:detail_text];

    let page_label = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 100.0, y: app_frame.size.height - 112.0 },
            size: CGSize {
                width: app_frame.size.width - 200.0,
                height: 18.0,
            },
        },
        "",
        12.0,
        OF_DIM_TEXT,
        UITextAlignmentCenter,
    );
    () = msg![env; detail_view addSubview:page_label];

    let prev_page_button = make_button(
        env,
        delegate,
        CGRect {
            origin: CGPoint { x: 20.0, y: app_frame.size.height - 108.0 },
            size: CGSize { width: 70.0, height: 30.0 },
        },
        "< Prev",
        "ofPrevPage",
        0,
        14.0,
    );
    () = msg![env; detail_view addSubview:prev_page_button];
    let next_page_button = make_button(
        env,
        delegate,
        CGRect {
            origin: CGPoint {
                x: app_frame.size.width - 90.0,
                y: app_frame.size.height - 108.0,
            },
            size: CGSize { width: 70.0, height: 30.0 },
        },
        "Next >",
        "ofNextPage",
        0,
        14.0,
    );
    () = msg![env; detail_view addSubview:next_page_button];

    let launch_button = make_button(
        env,
        delegate,
        CGRect {
            origin: CGPoint { x: 20.0, y: app_frame.size.height - 64.0 },
            size: CGSize {
                width: app_frame.size.width - 40.0,
                height: 36.0,
            },
        },
        "Play",
        "ofLaunch:",
        1,
        18.0,
    );
    let _of_c = color(env, OF_ACCENT);
    () = msg![env; launch_button setBackgroundColor:_of_c];
    let _of_c = color(env, (0.0, 0.0, 0.0, 1.0));
    () = msg![env; launch_button setTitleColor:_of_c
                                   forState:UIControlStateNormal];
    () = msg![env; detail_view addSubview:launch_button];

    update_plaques(env, games, &buttons, &labels, max_slots);
    // The dashboard is the default screen when it has data.
    () = msg![env; (of_main_view) setHidden:false];

    OfDashboardStuff {
        of_main_view,
        detail_view,
        detail_text,
        page_label,
    }
}

fn update_plaques(
    env: &mut Environment,
    games: &[GameStats],
    buttons: &[id],
    labels: &[id],
    max_slots: usize,
) {
    for (slot, (&button, &label)) in buttons.iter().zip(labels.iter()).enumerate() {
        if slot >= max_slots.min(games.len()) {
            () = msg![env; button setHidden:true];
            () = msg![env; label setHidden:true];
            continue;
        }
        let game = &games[slot];
        () = msg![env; button setHidden:false];
        () = msg![env; label setHidden:false];
        // Tag encodes the game index (0 would be the default/invalid tag).
        () = msg![env; button setTag:(game.app_idx as NSInteger + 1)];
        let ns_text = ns_string::from_rust_string(env, game.display_name.clone());
        () = msg![env; button setTitle:ns_text forState:UIControlStateNormal];
        let unlocked = unlocked_count(game);
        let subtitle = format!(
            "{}/{} {}",
            unlocked,
            game.ach_state.len(),
            if game.scores.is_empty() {
                "achievements".to_owned()
            } else {
                format!("ach, {} scores", game.scores.len())
            }
        );
        let ns_subtitle = ns_string::from_rust_string(env, subtitle);
        () = msg![env; label setText:ns_subtitle];
    }
}

pub fn show_all_apps(env: &mut Environment, stuff: &OfDashboardStuff) {
    () = msg![env; (stuff.of_main_view) setHidden:true];
}

pub fn show_dashboard(env: &mut Environment, stuff: &OfDashboardStuff) {
    () = msg![env; (stuff.of_main_view) setHidden:false];
    () = msg![env; (stuff.detail_view) setHidden:true];
}

pub fn hide_detail(env: &mut Environment, stuff: &OfDashboardStuff) {
    () = msg![env; (stuff.detail_view) setHidden:true];
}

pub fn show_detail(
    env: &mut Environment,
    stuff: &OfDashboardStuff,
    games: &[GameStats],
    game_idx: usize,
    page: usize,
) {
    if game_idx >= games.len() {
        return;
    }
    () = msg![env; (stuff.of_main_view) setHidden:false];
    () = msg![env; (stuff.detail_view) setHidden:false];

    let game = &games[game_idx];
    let mut lines: Vec<String> = Vec::new();
    lines.push(game.display_name.clone());
    let unlocked = unlocked_count(game);
    lines.push(format!(
        "{} / {} achievements",
        unlocked,
        game.ach_state.len()
    ));

    if !game.ach_defs.is_empty() {
        let defs: HashMap<&str, (&str, f32)> = game
            .ach_defs
            .iter()
            .map(|(id, title, score)| (id.as_str(), (title.as_str(), *score)))
            .collect();
        lines.push(String::new());
        lines.push("-- Achievements --".to_owned());
        for (ach_id, percent) in &game.ach_state {
            let (title, score) = defs
                .get(ach_id.as_str())
                .copied()
                .unwrap_or((ach_id.as_str(), 0.0));
            let mark = if *percent >= 100.0 { "[X]" } else { "[ ]" };
            if *percent >= 100.0 || *percent <= 0.0 {
                lines.push(format!("{} {} ({} pts)", mark, title, score as i32));
            } else {
                lines.push(format!("{} {} - {}%", mark, title, percent));
            }
        }
        for (ach_id, title, score) in &game.ach_defs {
            if !game.ach_state.iter().any(|(id, _)| id == ach_id) {
                lines.push(format!("[ ] {} ({} pts)", title, *score as i32));
            }
        }
    } else {
        lines.push(String::new());
        lines.push("-- Achievements --".to_owned());
        for (ach_id, percent) in &game.ach_state {
            let mark = if *percent >= 100.0 { "[X]" } else { "[ ]" };
            if *percent >= 100.0 || *percent <= 0.0 {
                lines.push(format!("{} {}", mark, ach_id));
            } else {
                lines.push(format!("{} {} - {}%", mark, ach_id, percent));
            }
        }
    }

    if !game.scores.is_empty() {
        lines.push(String::new());
        lines.push("-- Leaderboards --".to_owned());
        for (name, score) in &game.scores {
            lines.push(format!("{} - Best: {}", name, score));
        }
    }

    let pages = ((lines.len() + LINES_PER_PAGE - 1) / LINES_PER_PAGE).max(1);
    let page = page.min(pages - 1);
    let start = page * LINES_PER_PAGE;
    let end = (start + LINES_PER_PAGE).min(lines.len());
    let mut text = lines[start..end].join("\n");
    if pages > 1 {
        text.push_str(&format!("\nPage {}/{}", page + 1, pages));
        let ns_page = ns_string::from_rust_string(env, format!("Page {}/{}", page + 1, pages));
        () = msg![env; (stuff.page_label) setText:ns_page];
    } else {
        let ns_page = ns_string::from_rust_string(env, String::new());
        () = msg![env; (stuff.page_label) setText:ns_page];
    }

    let ns_text = ns_string::from_rust_string(env, text);
    () = msg![env; (stuff.detail_text) setText:ns_text];
    // Recomposite so the new page is visible right away.
    crate::frameworks::core_animation::recomposite_if_necessary(env, /* force: */ true);
}
