//! The OpenFeint dashboard: the default app-picker screen, restyled after the
//! original OpenFeint app (dark theme, green accents, bottom tab bar).
//!
//! Tabs: Discovery, My Games, Friends, Settings. My Games lists every
//! installed app with its locally tracked OpenFeint achievements and high
//! scores (written by the OpenFeint HLE layer into
//! `touchHLE_sandbox/<bundle id>/AppSandbox/Library/OpenFeint/`), plus the
//! real achievement/leaderboard titles from each app bundle's
//! `openfeint_offline_config.xml`.

use crate::environment::app_picker::AppInfo;
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string;
use crate::frameworks::foundation::NSInteger;
use crate::frameworks::uikit::ui_font::{UITextAlignment, UITextAlignmentCenter, UITextAlignmentLeft, UITextAlignmentRight};
use crate::frameworks::uikit::ui_view::ui_control::ui_button::UIButtonTypeCustom;
use crate::frameworks::uikit::ui_view::ui_control::{
    UIControlEventTouchUpInside, UIControlStateNormal,
};
use crate::objc::{id, msg, msg_class};
use crate::paths;
use crate::Environment;

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

// OpenFeint palette (sampled from the original screenshots).
const OF_TAB_BG: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.06, 0.06, 0.06, 1.0);
const OF_TOP_BG: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.24, 0.24, 0.24, 1.0);
const OF_BG: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.13, 0.13, 0.13, 1.0);
const OF_PANEL: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.17, 0.17, 0.17, 1.0);
const OF_HEADER: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.10, 0.10, 0.10, 1.0);
const OF_GREEN: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.35, 0.72, 0.22, 1.0);
const OF_GREEN_DARK: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.16, 0.42, 0.10, 1.0);
const OF_ROW: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.88, 0.88, 0.88, 1.0);
const OF_ROW_ALT: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.80, 0.80, 0.80, 1.0);
const OF_ROW_TEXT: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.15, 0.15, 0.15, 1.0);
const OF_WHITE: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.97, 0.97, 0.97, 1.0);
const OF_GRAY: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.62, 0.62, 0.62, 1.0);
const OF_RED: (CGFloat, CGFloat, CGFloat, CGFloat) = (0.78, 0.16, 0.12, 1.0);

pub const PAGE_DISCOVERY: usize = 0;
pub const PAGE_MY_GAMES: usize = 1;
pub const PAGE_FRIENDS: usize = 2;
pub const PAGE_SETTINGS: usize = 3;

const TAB_TITLES: [&str; 4] = ["Discovery", "My Games", "Friends", "Settings"];
const TAB_SELECTION: [&str; 4] = [
    "ofTab:",
    "ofTab:",
    "ofTab:",
    "ofTab:",
];

pub struct GameStats {
    pub app_idx: usize,
    pub app_path: PathBuf,
    pub display_name: String,
    pub bundle_id: String,
    /// `UIImage*` (app icon), if available.
    pub icon_ui_image: Option<id>,
    /// `(id, title, gamerscore)` from the offline config, if available.
    pub ach_defs: Vec<(String, String, f32)>,
    /// `(id, percent)` of every locally tracked achievement.
    pub ach_state: Vec<(String, f32)>,
    /// `(leaderboard title or id, best formatted score)`.
    pub scores: Vec<(String, String)>,
}

impl GameStats {
    fn gamerscore(&self) -> i32 {
        self.ach_state
            .iter()
            .filter(|(_, p)| *p >= 100.0)
            .filter_map(|(id_, _)| {
                self.ach_defs
                    .iter()
                    .find(|(def_id, _, _)| def_id == id_)
                    .map(|(_, _, s)| *s)
            })
            .sum::<f32>() as i32
    }

    fn unlocked(&self) -> usize {
        self.ach_state.iter().filter(|(_, p)| *p >= 100.0).count()
    }
}

pub struct OfDashboardStuff {
    pub of_main_view: id,
    top_title: id,
    pages: [id; 4],
    tab_buttons: [id; 4],
    detail_view: id,
    detail_title: id,
    detail_text: id,
    page_label: id,
    game_buttons: Vec<id>,
    game_sub_labels: Vec<id>,
    game_score_labels: Vec<id>,
    sidebar_name: id,
    sidebar_score: id,
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
    alignment: UITextAlignment,
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

fn make_view(env: &mut Environment, frame: CGRect, rgba: (CGFloat, CGFloat, CGFloat, CGFloat)) -> id {
    let view: id = msg_class![env; UIView alloc];
    let view: id = msg![env; view initWithFrame:frame];
    let _of_c = color(env, rgba);
    () = msg![env; view setBackgroundColor:_of_c];
    view
}

fn make_button(
    env: &mut Environment,
    delegate: id,
    frame: CGRect,
    title: &str,
    action_sel: &str,
    tag: NSInteger,
    font_size: CGFloat,
    text_rgba: (CGFloat, CGFloat, CGFloat, CGFloat),
    bg_rgba: (CGFloat, CGFloat, CGFloat, CGFloat),
) -> id {
    let button: id = msg_class![env; UIButton buttonWithType:UIButtonTypeCustom];
    () = msg![env; button setFrame:frame];
    () = msg![env; button setTag:tag];
    let ns_title = ns_string::from_rust_string(env, title.to_owned());
    () = msg![env; button setTitle:ns_title forState:UIControlStateNormal];
    () = msg![env; button layoutSubviews];
    let font: id = msg_class![env; UIFont boldSystemFontOfSize:font_size];
    () = msg![env; button setFont:font];
    let _of_c = color(env, text_rgba);
    () = msg![env; button setTitleColor:_of_c forState:UIControlStateNormal];
    let _of_c = color(env, bg_rgba);
    () = msg![env; button setBackgroundColor:_of_c];
    if !action_sel.is_empty() {
        let sel = env.objc.lookup_selector(action_sel).unwrap();
        () = msg![env; button addTarget:delegate
                           action:sel
                 forControlEvents:UIControlEventTouchUpInside];
    }
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

fn player_name() -> String {
    let path = paths::user_data_base_path().join("OpenFeint_player.txt");
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Player".to_owned())
}

/// Gather OpenFeint stats for every installed app (all of them are listed, so
/// the dashboard is always usable; apps without OpenFeint data simply show
/// zero progress).
pub fn collect_game_stats(apps: &mut [AppInfo]) -> Vec<GameStats> {
    let mut games = Vec::new();
    for (app_idx, app) in apps.iter_mut().enumerate() {
        let bundle_id = app.bundle_id.clone();
        let ach_state: Vec<(String, f32)> = read_tsv_lines(&bundle_id, "achievements.tsv")
            .into_iter()
            .filter_map(|(id, percent, _)| percent.parse::<f32>().ok().map(|p| (id, p)))
            .collect();
        let scores: Vec<(String, String)> = read_tsv_lines(&bundle_id, "highscores.tsv")
            .into_iter()
            .map(|(id, score, display)| {
                let name = if display.is_empty() { id } else { display };
                (name, score)
            })
            .collect();

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
            icon_ui_image: app.icon_ui_image,
            bundle_id,
            ach_defs,
            ach_state,
            scores,
        });
    }
    games
}

pub fn setup(
    env: &mut Environment,
    delegate: id,
    main_view: id,
    app_frame: CGRect,
    games: &[GameStats],
) -> OfDashboardStuff {
    let width = app_frame.size.width;
    let height = app_frame.size.height;

    let of_main_view = make_view(
        env,
        CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: app_frame.size,
        },
        OF_BG,
    );
    () = msg![env; of_main_view setTag:900];
    () = msg![env; main_view addSubview:of_main_view];

    // ---- Top bar ----
    let top_bar = make_view(
        env,
        CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize { width, height: 40.0 },
        },
        OF_TOP_BG,
    );
    () = msg![env; of_main_view addSubview:top_bar];
    let top_title = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 60.0, y: 6.0 },
            size: CGSize {
                width: width - 120.0,
                height: 28.0,
            },
        },
        "Discovery",
        21.0,
        OF_WHITE,
        UITextAlignmentCenter,
    );
    () = msg![env; top_bar addSubview:top_title];
    let close_button = make_button(
        env,
        delegate,
        CGRect {
            origin: CGPoint {
                x: width - 44.0,
                y: 2.0,
            },
            size: CGSize { width: 40.0, height: 36.0 },
        },
        "X",
        "ofSelectDiscovery",
        0,
        20.0,
        OF_WHITE,
        OF_GREEN_DARK,
    );
    () = msg![env; top_bar addSubview:close_button];

    // ---- Sidebar ----
    let sidebar = make_view(
        env,
        CGRect {
            origin: CGPoint { x: 0.0, y: 40.0 },
            size: CGSize { width: 56.0, height: height - 96.0 },
        },
        OF_HEADER,
    );
    () = msg![env; of_main_view addSubview:sidebar];
    let sidebar_name = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 2.0, y: 6.0 },
            size: CGSize { width: 52.0, height: 30.0 },
        },
        &player_name(),
        9.5,
        OF_WHITE,
        UITextAlignmentCenter,
    );
    () = msg![env; sidebar addSubview:sidebar_name];
    let sidebar_score = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 2.0, y: 36.0 },
            size: CGSize { width: 52.0, height: 20.0 },
        },
        "0",
        13.0,
        OF_GREEN,
        UITextAlignmentCenter,
    );
    () = msg![env; sidebar addSubview:sidebar_score];

    // ---- Tab bar ----
    let tab_bar = make_view(
        env,
        CGRect {
            origin: CGPoint {
                x: 0.0,
                y: height - 56.0,
            },
            size: CGSize { width, height: 56.0 },
        },
        OF_TAB_BG,
    );
    () = msg![env; of_main_view addSubview:tab_bar];
    let tab_width = (width / 4.0).round();
    let mut tab_buttons: Vec<id> = Vec::new();
    for (tab_idx, tab_title) in TAB_TITLES.iter().enumerate() {
        let tab_button = make_button(
            env,
            delegate,
            CGRect {
                origin: CGPoint {
                    x: tab_width * tab_idx as CGFloat,
                    y: 0.0,
                },
                size: CGSize {
                    width: tab_width,
                    height: 56.0,
                },
            },
            tab_title,
            TAB_SELECTION[tab_idx],
            (tab_idx + 1) as NSInteger,
            14.0,
            OF_GRAY,
            OF_TAB_BG,
        );
        () = msg![env; tab_bar addSubview:tab_button];
        tab_buttons.push(tab_button);
    }
    let tab_buttons = [tab_buttons[0], tab_buttons[1], tab_buttons[2], tab_buttons[3]];

    // Content geometry (between top bar and tab bar, right of sidebar).
    let content_x = 62.0;
    let content_y = 46.0;
    let content_w = width - content_x - 6.0;
    let content_h = height - content_y - 62.0;
    let page_frame = CGRect {
        origin: CGPoint {
            x: content_x,
            y: content_y,
        },
        size: CGSize {
            width: content_w,
            height: content_h,
        },
    };

    // ---- Discovery page ----
    let discovery_page = make_view(env, page_frame, OF_PANEL);
    () = msg![env; of_main_view addSubview:discovery_page];
    let welcome_card = make_view(
        env,
        CGRect {
            origin: CGPoint { x: 10.0, y: 10.0 },
            size: CGSize {
                width: content_w - 20.0,
                height: 72.0,
            },
        },
        OF_HEADER,
    );
    () = msg![env; discovery_page addSubview:welcome_card];
    let welcome_title = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 24.0, y: 8.0 },
            size: CGSize {
                width: content_w - 48.0,
                height: 28.0,
            },
        },
        &format!("Welcome {}!", player_name()),
        21.0,
        OF_WHITE,
        UITextAlignmentLeft,
    );
    () = msg![env; welcome_card addSubview:welcome_title];
    let welcome_sub = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 24.0, y: 40.0 },
            size: CGSize {
                width: content_w - 48.0,
                height: 22.0,
            },
        },
        "Where would you like to explore?",
        13.0,
        OF_GRAY,
        UITextAlignmentLeft,
    );
    () = msg![env; welcome_card addSubview:welcome_sub];

    let big_w = ((content_w - 40.0) / 3.0).round();
    let big_titles = ["Global Chat", "My Friends", "Help"];
    for (big_idx, big_title) in big_titles.iter().enumerate() {
        let big_x = 10.0 + (big_w + 10.0) * big_idx as CGFloat;
        let big_button = make_button(
            env,
            delegate,
            CGRect {
                origin: CGPoint { x: big_x, y: 96.0 },
                size: CGSize {
                    width: big_w,
                    height: 110.0,
                },
            },
            big_title,
            "",
            0,
            14.0,
            OF_ROW_TEXT,
            OF_ROW,
        );
        () = msg![env; discovery_page addSubview:big_button];
    }
    let of_note = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 10.0, y: 220.0 },
            size: CGSize {
                width: content_w - 20.0,
                height: 40.0,
            },
        },
        "Offline mode: achievements and high scores are saved on this computer.",
        12.0,
        OF_GRAY,
        UITextAlignmentCenter,
    );
    () = msg![env; discovery_page addSubview:of_note];

    // ---- My Games page ----
    let games_page = make_view(env, page_frame, OF_PANEL);
    () = msg![env; games_page setHidden:true];
    () = msg![env; of_main_view addSubview:games_page];
    let games_header = make_view(
        env,
        CGRect {
            origin: CGPoint { x: 10.0, y: 8.0 },
            size: CGSize {
                width: content_w - 20.0,
                height: 36.0,
            },
        },
        OF_HEADER,
    );
    () = msg![env; games_page addSubview:games_header];
    let games_header_label = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 16.0, y: 6.0 },
            size: CGSize {
                width: content_w - 48.0,
                height: 24.0,
            },
        },
        "My Games",
        19.0,
        OF_WHITE,
        UITextAlignmentLeft,
    );
    () = msg![env; games_header addSubview:games_header_label];

    // Game rows (up to 8 visible).
    let row_h = 62.0;
    let max_rows = (((content_h - 60.0) / (row_h + 4.0)) as usize).clamp(1, 8);
    let mut game_buttons = Vec::new();
    let mut game_sub_labels = Vec::new();
    let mut game_score_labels = Vec::new();
    for row in 0..max_rows {
        let y = 54.0 + (row_h + 4.0) * row as CGFloat;
        let game_button = make_button(
            env,
            delegate,
            CGRect {
                origin: CGPoint { x: 10.0, y },
                size: CGSize {
                    width: content_w - 20.0,
                    height: row_h,
                },
            },
            "",
            "ofGameTapped:",
            0,
            15.0,
            OF_ROW_TEXT,
            OF_ROW,
        );
        () = msg![env; games_page addSubview:game_button];
        game_buttons.push(game_button);
        let game_sub_label = make_label(
            env,
            CGRect {
                origin: CGPoint {
                    x: 24.0,
                    y: y + row_h - 15.0,
                },
                size: CGSize {
                    width: content_w - 110.0,
                    height: 12.0,
                },
            },
            "",
            10.5,
            OF_GRAY,
            UITextAlignmentLeft,
        );
        () = msg![env; games_page addSubview:game_sub_label];
        game_sub_labels.push(game_sub_label);
        let game_score_label = make_label(
            env,
            CGRect {
                origin: CGPoint {
                    x: content_w - 78.0,
                    y: y + 6.0,
                },
                size: CGSize { width: 60.0, height: 22.0 },
            },
            "",
            15.0,
            OF_GREEN_DARK,
            UITextAlignmentRight,
        );
        () = msg![env; games_page addSubview:game_score_label];
        game_score_labels.push(game_score_label);
    }

    // ---- Game detail page (sheet over everything) ----
    let detail_view = make_view(
        env,
        CGRect {
            origin: CGPoint { x: 0.0, y: 40.0 },
            size: CGSize {
                width,
                height: height - 96.0,
            },
        },
        OF_BG,
    );
    () = msg![env; detail_view setHidden:true];
    () = msg![env; detail_view setTag:902];
    () = msg![env; of_main_view addSubview:detail_view];
    let back_button = make_button(
        env,
        delegate,
        CGRect {
            origin: CGPoint { x: 8.0, y: 6.0 },
            size: CGSize { width: 90.0, height: 30.0 },
        },
        "Back",
        "ofBack",
        0,
        15.0,
        OF_WHITE,
        OF_GREEN_DARK,
    );
    () = msg![env; detail_view addSubview:back_button];
    let detail_title = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 106.0, y: 6.0 },
            size: CGSize {
                width: width - 212.0,
                height: 28.0,
            },
        },
        "",
        19.0,
        OF_WHITE,
        UITextAlignmentCenter,
    );
    () = msg![env; detail_view addSubview:detail_title];
    let detail_text = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 14.0, y: 40.0 },
            size: CGSize {
                width: width - 28.0,
                height: height - 96.0 - 130.0,
            },
        },
        "",
        12.0,
        OF_WHITE,
        UITextAlignmentLeft,
    );
    () = msg![env; detail_view addSubview:detail_text];
    let page_label = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 100.0, y: height - 96.0 - 34.0 },
            size: CGSize {
                width: width - 200.0,
                height: 16.0,
            },
        },
        "",
        11.0,
        OF_GRAY,
        UITextAlignmentCenter,
    );
    () = msg![env; detail_view addSubview:page_label];
    let prev_page_button = make_button(
        env,
        delegate,
        CGRect {
            origin: CGPoint { x: 14.0, y: height - 96.0 - 36.0 },
            size: CGSize { width: 76.0, height: 30.0 },
        },
        "< Prev",
        "ofPrevPage",
        0,
        14.0,
        OF_WHITE,
        OF_PANEL,
    );
    () = msg![env; detail_view addSubview:prev_page_button];
    let next_page_button = make_button(
        env,
        delegate,
        CGRect {
            origin: CGPoint {
                x: width - 90.0,
                y: height - 96.0 - 36.0,
            },
            size: CGSize { width: 76.0, height: 30.0 },
        },
        "Next >",
        "ofNextPage",
        0,
        14.0,
        OF_WHITE,
        OF_PANEL,
    );
    () = msg![env; detail_view addSubview:next_page_button];
    let launch_button = make_button(
        env,
        delegate,
        CGRect {
            origin: CGPoint { x: 20.0, y: height - 96.0 - 0.0 - 44.0 + 8.0 },
            size: CGSize {
                width: width - 40.0,
                height: 34.0,
            },
        },
        "Play",
        "ofLaunch:",
        1,
        17.0,
        OF_WHITE,
        OF_GREEN,
    );
    () = msg![env; detail_view addSubview:launch_button];

    // ---- Friends page ----
    let friends_page = make_view(env, page_frame, OF_PANEL);
    () = msg![env; friends_page setHidden:true];
    () = msg![env; of_main_view addSubview:friends_page];
    let find_bar = make_view(
        env,
        CGRect {
            origin: CGPoint { x: 40.0, y: 12.0 },
            size: CGSize {
                width: content_w - 80.0,
                height: 40.0,
            },
        },
        OF_HEADER,
    );
    () = msg![env; friends_page addSubview:find_bar];
    let find_label = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 0.0, y: 9.0 },
            size: CGSize {
                width: content_w - 80.0,
                height: 22.0,
            },
        },
        "Find My Friends",
        16.0,
        OF_GRAY,
        UITextAlignmentCenter,
    );
    () = msg![env; find_bar addSubview:find_label];
    let friends_seg = make_button(
        env,
        delegate,
        CGRect {
            origin: CGPoint { x: 30.0, y: 66.0 },
            size: CGSize {
                width: (content_w - 60.0) / 2.0,
                height: 30.0,
            },
        },
        "* Friends",
        "",
        0,
        15.0,
        OF_ROW_TEXT,
        OF_ROW,
    );
    () = msg![env; friends_page addSubview:friends_seg];
    let pending_seg = make_button(
        env,
        delegate,
        CGRect {
            origin: CGPoint {
                x: 30.0 + (content_w - 60.0) / 2.0,
                y: 66.0,
            },
            size: CGSize {
                width: (content_w - 60.0) / 2.0,
                height: 30.0,
            },
        },
        "Pending",
        "",
        0,
        15.0,
        OF_ROW_TEXT,
        OF_ROW_ALT,
    );
    () = msg![env; friends_page addSubview:pending_seg];
    let friends_row = make_button(
        env,
        delegate,
        CGRect {
            origin: CGPoint { x: 30.0, y: 104.0 },
            size: CGSize {
                width: content_w - 60.0,
                height: 54.0,
            },
        },
        "",
        "",
        0,
        15.0,
        OF_ROW_TEXT,
        OF_ROW,
    );
    () = msg![env; friends_page addSubview:friends_row];
    let friends_row_text = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 44.0, y: 116.0 },
            size: CGSize {
                width: content_w - 90.0,
                height: 32.0,
            },
        },
        "Offline mode - no friends yet",
        13.0,
        OF_ROW_TEXT,
        UITextAlignmentLeft,
    );
    () = msg![env; friends_page addSubview:friends_row_text];

    // ---- Settings page ----
    let settings_page = make_view(env, page_frame, OF_PANEL);
    () = msg![env; settings_page setHidden:true];
    () = msg![env; of_main_view addSubview:settings_page];
    let settings_header = make_view(
        env,
        CGRect {
            origin: CGPoint { x: 10.0, y: 10.0 },
            size: CGSize {
                width: content_w - 20.0,
                height: 36.0,
            },
        },
        OF_HEADER,
    );
    () = msg![env; settings_page addSubview:settings_header];
    let settings_header_label = make_label(
        env,
        CGRect {
            origin: CGPoint { x: 16.0, y: 6.0 },
            size: CGSize {
                width: content_w - 48.0,
                height: 24.0,
            },
        },
        &format!("{}'s Settings", player_name()),
        17.0,
        OF_WHITE,
        UITextAlignmentLeft,
    );
    () = msg![env; settings_header addSubview:settings_header_label];
    let settings_rows = [
        format!("Change Your Feint Name ({})", player_name()),
        "Import Friends".to_owned(),
        "Device Settings: offline emulator".to_owned(),
    ];
    for (row_idx, row_text) in settings_rows.iter().enumerate() {
        let settings_row = make_button(
            env,
            delegate,
            CGRect {
                origin: CGPoint {
                    x: 10.0,
                    y: 56.0 + 42.0 * row_idx as CGFloat,
                },
                size: CGSize {
                    width: content_w - 20.0,
                    height: 38.0,
                },
            },
            &format!("{row_text} >"),
            "",
            0,
            14.0,
            OF_ROW_TEXT,
            OF_ROW,
        );
        () = msg![env; settings_page addSubview:settings_row];
    }

    update_my_games(env, games, &game_buttons, &game_sub_labels, &game_score_labels);
    let stuff = OfDashboardStuff {
        of_main_view,
        top_title,
        pages: [discovery_page, games_page, friends_page, settings_page],
        tab_buttons,
        detail_view,
        detail_title,
        detail_text,
        page_label,
        game_buttons,
        game_sub_labels,
        game_score_labels,
        sidebar_name,
        sidebar_score,
    };
    show_page(env, &stuff, PAGE_DISCOVERY);
    let _ = delegate;
    stuff
}

fn update_my_games(
    env: &mut Environment,
    games: &[GameStats],
    buttons: &[id],
    sub_labels: &[id],
    score_labels: &[id],
) {
    for (row, ((&button, &sub_label), &score_label)) in buttons
        .iter()
        .zip(sub_labels.iter())
        .zip(score_labels.iter())
        .enumerate()
    {
        if row >= games.len() {
            () = msg![env; button setHidden:true];
            () = msg![env; sub_label setHidden:true];
            () = msg![env; score_label setHidden:true];
            continue;
        }
        let game = &games[row];
        () = msg![env; button setHidden:false];
        () = msg![env; sub_label setHidden:false];
        () = msg![env; score_label setHidden:false];
        // Tag encodes the games index (0 would be the default/invalid tag).
        () = msg![env; button setTag:(row as NSInteger + 1)];
        let ns_text = ns_string::from_rust_string(env, game.display_name.clone());
        () = msg![env; button setTitle:ns_text forState:UIControlStateNormal];
        let mut subtitle = if game.ach_state.is_empty() {
            if game.scores.is_empty() {
                "No OpenFeint data yet".to_owned()
            } else {
                format!("{} leaderboards", game.scores.len())
            }
        } else {
            format!(
                "{}/{} achievements, {} leaderboards",
                game.unlocked(),
                game.ach_state.len(),
                game.scores.len()
            )
        };
        subtitle.truncate(38);
        let ns_subtitle = ns_string::from_rust_string(env, subtitle);
        () = msg![env; sub_label setText:ns_subtitle];
        let ns_score = ns_string::from_rust_string(env, game.gamerscore().to_string());
        () = msg![env; score_label setText:ns_score];
    }
}

fn set_tab_colors(env: &mut Environment, stuff: &OfDashboardStuff, active: usize) {
    for (tab_idx, &tab_button) in stuff.tab_buttons.iter().enumerate() {
        let rgba = if tab_idx == active { OF_GREEN } else { OF_GRAY };
        let _of_c = color(env, rgba);
        () = msg![env; tab_button setTitleColor:_of_c forState:UIControlStateNormal];
    }
}

pub fn show_page(env: &mut Environment, stuff: &OfDashboardStuff, page: usize) {
    () = msg![env; (stuff.of_main_view) setHidden:false];
    () = msg![env; (stuff.detail_view) setHidden:true];
    for (page_idx, &page_view) in stuff.pages.iter().enumerate() {
        () = msg![env; page_view setHidden:(page_idx != page)];
    }
    let title = TAB_TITLES.get(page).copied().unwrap_or("Discovery");
    let ns_title = ns_string::from_rust_string(env, title.to_owned());
    () = msg![env; (stuff.top_title) setText:ns_title];
    set_tab_colors(env, stuff, page);
    crate::frameworks::core_animation::recomposite_if_necessary(env, /* force: */ true);
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
    for &page_view in stuff.pages.iter() {
        () = msg![env; page_view setHidden:true];
    }
    set_tab_colors(env, stuff, PAGE_MY_GAMES);

    let game = &games[game_idx];
    let ns_title = ns_string::from_rust_string(env, game.display_name.clone());
    () = msg![env; (stuff.detail_title) setText:ns_title];

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "{} / {} achievements, {} pts",
        game.unlocked(),
        game.ach_state.len(),
        game.gamerscore()
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

    let lines_per_page = 14; let pages = ((lines.len() + lines_per_page - 1) / lines_per_page).max(1);
    let page = page.min(pages - 1);
    let start = page * lines_per_page;
    let end = (start + lines_per_page).min(lines.len());
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
