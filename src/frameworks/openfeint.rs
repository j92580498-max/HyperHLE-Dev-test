/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! High-level emulation of the OpenFeint social/achievements SDK.
//!
//! OpenFeint (server shut down in 2012) was statically linked into many
//! iPhone OS games. This module replaces the SDK with a local, offline
//! implementation so that achievements keep working in the emulator:
//!
//! - `+[OpenFeint initializeWithProductKey:andSecret:andDisplayName:andSettings:andDelegates:]`
//!   succeeds without any networking.
//! - `+[OFAchievementService updateAchievement:andPercentComplete:andShowNotification:]`
//!   (and its multi-achievement / invocation variants) records progress in a
//!   persistent per-app store inside the guest filesystem and shows a native
//!   "Achievement Unlocked!" toast the first time an achievement reaches 100%.
//!
//! All other OpenFeint classes are substituted with fake classes (see
//! `substitute_classes` in `crate::objc::classes`), which behave as if every
//! message was sent to nil, so the real (networked) SDK code never runs.

use crate::Environment;
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::NSInteger;
use crate::frameworks::uikit::ui_font::UITextAlignmentCenter;
use crate::mem::Ptr;
use crate::objc::{autorelease, id, msg, msg_class, nil, Class, SEL};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default)]
struct AchEntry {
    percent: f32,
    /// Whether the unlock toast has already been shown for this achievement.
    notified: bool,
}

#[derive(Clone, Debug, Default)]
struct HighScoreEntry {
    score: i64,
    display: String,
}

#[derive(Default)]
struct OpenFeintState {
    initialized: bool,
    achievements: HashMap<String, AchEntry>,
    highscores: HashMap<String, HighScoreEntry>,
    /// Achievement definitions parsed from the app's bundled
    /// `openfeint_offline_config.xml` (OF id -> definition).
    defs: HashMap<String, AchDef>,
    /// Leaderboard definitions parsed from the offline config (OF id -> name).
    boards: Vec<(String, String)>,
    /// Currently displayed dashboard overlay view, if any.
    overlay: Option<id>,
}

/// A single achievement definition from the offline config.
#[derive(Clone, Debug, Default)]
struct AchDef {
    title: String,
    description: String,
    gamerscore: u32,
    position: u32,
}

static STATE: std::sync::Mutex<Option<OpenFeintState>> = std::sync::Mutex::new(None);

fn with_state<R>(f: impl FnOnce(&mut OpenFeintState) -> R) -> R {
    let mut guard = STATE.lock().unwrap();
    let state = guard.get_or_insert_with(OpenFeintState::default);
    f(state)
}

// MARK: - Persistence

const STORE_DIR: &str = "Library/OpenFeint";
const STORE_FILE: &str = "Library/OpenFeint/achievements.tsv";
const SCORES_FILE: &str = "Library/OpenFeint/highscores.tsv";

fn load_store(env: &mut Environment) {
    let text = {
        let Ok(mut file) = env.fs.open(crate::fs::GuestPath::new(STORE_FILE)) else {
            return;
        };
        let mut buf = Vec::new();
        if std::io::Read::read_to_end(&mut file, &mut buf).is_err() {
            return;
        }
        String::from_utf8_lossy(&buf).into_owned()
    };
    for line in text.lines() {
        let mut parts = line.split('\t');
        let (Some(id_str), Some(percent_str), Some(notified_str)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let Ok(percent) = percent_str.parse::<f32>() else {
            continue;
        };
        with_state(|s| {
            s.achievements.insert(
                id_str.to_string(),
                AchEntry {
                    percent,
                    notified: notified_str == "1",
                },
            );
        });
    }
}

fn save_store(env: &mut Environment) {
    let text = with_state(|s| {
        let mut lines: Vec<String> = s
            .achievements
            .iter()
            .map(|(k, v)| format!("{}\t{}\t{}", k, v.percent, v.notified as u8))
            .collect();
        lines.sort();
        lines.join("\n")
    });
    if env
        .fs
        .create_dir_all(crate::fs::GuestPath::new(STORE_DIR))
        .is_err()
    {
        log!("Warning: OpenFeint HLE: could not create store directory {}", STORE_DIR);
        return;
    }
    let mut options = crate::fs::GuestOpenOptions::new();
    options.write().create().truncate();
    match env.fs.open_with_options(crate::fs::GuestPath::new(STORE_FILE), options) {
        Ok(mut file) => {
            if let Err(e) = std::io::Write::write_all(&mut file, text.as_bytes()) {
                log!("Warning: OpenFeint HLE: could not save achievements store: {}", e);
            }
        }
        Err(_) => {
            log!("Warning: OpenFeint HLE: could not open achievements store for writing");
        }
    }
}

// MARK: - Offline config

/// Parse the app's bundled `openfeint_offline_config.xml` (a snapshot of the
/// dead OpenFeint server data) so achievements and leaderboards have their
/// real titles and descriptions in the HLE layer.
fn load_offline_config(env: &mut Environment) {
    let path = env.bundle.bundle_path().join("openfeint_offline_config.xml");
    let Ok(mut file) = env.fs.open(&path) else {
        return;
    };
    let mut buf = Vec::new();
    if std::io::Read::read_to_end(&mut file, &mut buf).is_err() {
        return;
    }
    let xml = String::from_utf8_lossy(&buf).into_owned();

    // Extract a tag's inner text without pulling in an XML crate.
    fn tag_text(block: &str, tag: &str) -> String {
        let open = format!("<{}>", tag);
        let close = format!("</{}>", tag);
        let Some(start) = block.find(&open) else {
            return String::new();
        };
        let rest = &block[start + open.len()..];
        let Some(end) = rest.find(&close) else {
            return String::new();
        };
        rest[..end].trim().to_string()
    }

    // Split the document into `<achievement>` / `<leaderboard>` blocks.
    let mut defs = HashMap::new();
    let mut boards = Vec::new();
    let mut split_on = |xml: &str, tag: &str| -> Vec<String> {
        let open = format!("<{}>", tag);
        let close = format!("</{}>", tag);
        let mut blocks = Vec::new();
        let mut rest = xml;
        while let Some(open_at) = rest.find(&open) {
            let after_open = &rest[open_at + open.len()..];
            let Some(close_at) = after_open.find(&close) else {
                break;
            };
            blocks.push(after_open[..close_at].to_string());
            rest = &after_open[close_at + close.len()..];
        }
        blocks
    };

    for block in split_on(&xml, "achievement") {
        let id = tag_text(&block, "id");
        if id.is_empty() {
            continue;
        }
        let title = tag_text(&block, "title");
        let description = tag_text(&block, "description");
        if title.is_empty() {
            continue;
        }
        defs.insert(
            id,
            AchDef {
                title: title.clone(),
                description: description.clone(),
                gamerscore: tag_text(&block, "gamerscore").parse().unwrap_or(0),
                position: tag_text(&block, "position").parse().unwrap_or(0),
            },
        );
    }
    for block in split_on(&xml, "leaderboard") {
        let id = tag_text(&block, "id");
        let name = tag_text(&block, "name");
        if !id.is_empty() && !name.is_empty() {
            boards.push((id, name));
        }
    }
    let (a, l) = (defs.len(), boards.len());
    if a > 0 || l > 0 {
        with_state(|s| {
            s.defs = defs;
            s.boards = boards;
        });
        log!(
            "OpenFeint HLE: loaded offline config: {} achievement definitions, {} leaderboards",
            a,
            l
        );
    }
}

// MARK: - Unlock toast

/// Show a short-lived "Achievement Unlocked!" toast over the key window.
fn show_unlock_toast(env: &mut Environment, achievement_id: &str) {
    let app: id = msg_class![env; UIApplication sharedApplication];
    let window: id = msg![env; app keyWindow];
    if window == nil {
        log!("OpenFeint HLE: no key window, skipping unlock toast");
        return;
    }
    let screen: id = msg_class![env; UIScreen mainScreen];
    let bounds: CGRect = msg![env; screen bounds];
    let width = 280.0f32;
    let height = 64.0f32;
    let frame = CGRect {
        origin: CGPoint {
            x: (bounds.size.width - width) / 2.0,
            y: 48.0,
        },
        size: CGSize {
            width,
            height,
        },
    };
    let label: id = msg_class![env; UILabel new];
    () = msg![env; label initWithFrame:frame];
    let title = with_state(|s| {
        s.defs
            .get(achievement_id)
            .map(|d| d.title.clone())
            .unwrap_or_else(|| achievement_id.to_string())
    });
    let text = format!("Achievement Unlocked!\n{}", title);
    let ns_text = crate::frameworks::foundation::ns_string::from_rust_string(env, text);
    () = msg![env; label setText:ns_text];
    () = msg![env; label setTextAlignment:UITextAlignmentCenter];
    () = msg![env; label setNumberOfLines:(2 as NSInteger)];
    () = msg![env; label setAdjustsFontSizeToFitWidth:true];
    let font: id = msg_class![env; UIFont boldSystemFontOfSize:(14.0f32)];
    () = msg![env; label setFont:font];
    let text_color: id = msg_class![env; UIColor whiteColor];
    () = msg![env; label setTextColor:text_color];
    let bg_color: id = msg_class![env; UIColor colorWithWhite:(0.0f32) alpha:(0.85f32)];
    () = msg![env; label setBackgroundColor:bg_color];
    () = msg![env; label setHidden:true];
    () = msg![env; window addSubview:label];
    // Fade-in is not available (no CA support for this shim), just show it.
    () = msg![env; label setHidden:false];

    // Auto-dismiss after 4 seconds: NSTimer targeting the label itself.
    let remove_sel: SEL = env
        .objc
        .lookup_selector("removeFromSuperview")
        .unwrap_or_else(|| {
            env.objc
                .register_host_selector("removeFromSuperview".to_string(), &mut env.mem)
        });
    let _: id = msg_class![env;
        NSTimer scheduledTimerWithTimeInterval:(4.0f64)
        target:label
        selector:remove_sel
        userInfo:nil
        repeats:false
    ];
    autorelease(env, label);
    log!("OpenFeint HLE: showing unlock toast for achievement {:?}", achievement_id);
}

// MARK: - Achievement progress

fn record_progress(env: &mut Environment, achievement_id: &str, percent: f32, show_notification: bool) {
    let unlocked = percent >= 100.0;
    let (just_unlocked, new_percent) = with_state(|s| {
        let entry = s
            .achievements
            .entry(achievement_id.to_string())
            .or_default();
        let just_unlocked = unlocked && !entry.notified && !entry.percent_ge_100();
        if percent >= entry.percent {
            entry.percent = percent;
        }
        if just_unlocked {
            entry.notified = true;
        }
        (just_unlocked, entry.percent)
    });
    log!(
        "OpenFeint HLE: achievement {:?} progress {}% (just unlocked: {}) for {}",
        achievement_id,
        new_percent,
        just_unlocked,
        env.bundle.bundle_identifier(),
    );
    save_store(env);
    if just_unlocked && show_notification {
        show_unlock_toast(env, achievement_id);
    }
}

impl AchEntry {
    fn percent_ge_100(&self) -> bool {
        self.percent >= 100.0
    }
}

// MARK: - Dashboard overlay

/// Show a native, tap-to-dismiss overlay listing the game's achievements
/// (with unlocked state) or leaderboards (with local best scores), replacing
/// the dead OpenFeint dashboard.
fn show_dashboard(env: &mut Environment, page: &str) {
    // Dismiss any existing overlay first.
    dismiss_dashboard(env);

    let app: id = msg_class![env; UIApplication sharedApplication];
    let mut window: id = msg![env; app keyWindow];
    if window == nil {
        // Some games never explicitly make their window key. Fall back to
        // the first application window so the dashboard can still show.
        let windows: id = msg![env; app windows];
        let count: crate::frameworks::foundation::NSUInteger = msg![env; windows count];
        if count > 0 {
            window = msg![env; windows objectAtIndex:(0)];
        }
    }
    if window == nil {
        log!("OpenFeint HLE: no key window, cannot show dashboard");
        return;
    }
    let screen: id = msg_class![env; UIScreen mainScreen];
    let bounds: CGRect = msg![env; screen bounds];

    let overlay: id = msg_class![env; UIView new];
    () = msg![env; overlay initWithFrame:bounds];
    let bg: id = msg_class![env; UIColor colorWithWhite:(0.08f32) alpha:(0.94f32)];
    () = msg![env; overlay setBackgroundColor:bg];

    // Title.
    let title_frame = CGRect {
        origin: CGPoint { x: 0.0, y: 20.0 },
        size: CGSize {
            width: bounds.size.width,
            height: 30.0,
        },
    };
    let title: id = msg_class![env; UILabel new];
    () = msg![env; title initWithFrame:title_frame];
    let title_text = if page == "leaderboards" {
        "Leaderboards".to_string()
    } else {
        "Achievements".to_string()
    };
    let ns_title = crate::frameworks::foundation::ns_string::from_rust_string(env, title_text);
    () = msg![env; title setText:ns_title];
    () = msg![env; title setTextAlignment:UITextAlignmentCenter];
    let title_font: id = msg_class![env; UIFont boldSystemFontOfSize:(18.0f32)];
    () = msg![env; title setFont:title_font];
    let white: id = msg_class![env; UIColor whiteColor];
    () = msg![env; title setTextColor:white];
    let title_bg: id = msg_class![env; UIColor clearColor];
    () = msg![env; title setBackgroundColor:title_bg];
    () = msg![env; title setUserInteractionEnabled:false];
    () = msg![env; overlay addSubview:title];

    // Content.
    let content_frame = CGRect {
        origin: CGPoint { x: 12.0, y: 58.0 },
        size: CGSize {
            width: bounds.size.width - 24.0,
            height: bounds.size.height - 96.0,
        },
    };
    let content: id = msg_class![env; UILabel new];
    () = msg![env; content initWithFrame:content_frame];
    let text = if page == "leaderboards" {
        dashboard_leaderboards_text(env)
    } else {
        dashboard_achievements_text(env)
    };
    let ns_text = crate::frameworks::foundation::ns_string::from_rust_string(env, text);
    () = msg![env; content setText:ns_text];
    () = msg![env; content setTextAlignment:UITextAlignmentCenter];
    () = msg![env; content setNumberOfLines:(0 as NSInteger)];
    let content_font: id = msg_class![env; UIFont systemFontOfSize:(13.0f32)];
    () = msg![env; content setFont:content_font];
    () = msg![env; content setTextColor:white];
    () = msg![env; content setBackgroundColor:title_bg];
    () = msg![env; content setUserInteractionEnabled:false];
    () = msg![env; overlay addSubview:content];

    // Hint.
    let hint_frame = CGRect {
        origin: CGPoint {
            x: 0.0,
            y: bounds.size.height - 32.0,
        },
        size: CGSize {
            width: bounds.size.width,
            height: 24.0,
        },
    };
    let hint: id = msg_class![env; UILabel new];
    () = msg![env; hint initWithFrame:hint_frame];
    let hint_text = "Tap anywhere to close";
    let ns_hint = crate::frameworks::foundation::ns_string::from_rust_string(env, hint_text.to_string());
    () = msg![env; hint setText:ns_hint];
    () = msg![env; hint setTextAlignment:UITextAlignmentCenter];
    let hint_font: id = msg_class![env; UIFont systemFontOfSize:(11.0f32)];
    () = msg![env; hint setFont:hint_font];
    let gray: id = msg_class![env; UIColor colorWithWhite:(0.7f32) alpha:(1.0f32)];
    () = msg![env; hint setTextColor:gray];
    () = msg![env; hint setBackgroundColor:title_bg];
    () = msg![env; hint setUserInteractionEnabled:false];
    () = msg![env; overlay addSubview:hint];

    // Tap anywhere on the overlay to dismiss.
    let remove_sel: SEL = env
        .objc
        .lookup_selector("removeFromSuperview")
        .unwrap_or_else(|| {
            env.objc
                .register_host_selector("removeFromSuperview".to_string(), &mut env.mem)
        });
    let tap: id = msg_class![env; UITapGestureRecognizer alloc];
    let tap: id = msg![env; tap initWithTarget:overlay action:remove_sel];
    () = msg![env; overlay addGestureRecognizer:tap];

    () = msg![env; window addSubview:overlay];
    let overlay_for_state = overlay;
    with_state(|s| s.overlay = Some(overlay_for_state));
    autorelease(env, overlay);
    log!("OpenFeint HLE: showing {} dashboard overlay", page);
}

fn dismiss_dashboard(env: &mut Environment) {
    let overlay = with_state(|s| s.overlay.take());
    if let Some(overlay) = overlay {
        () = msg![env; overlay removeFromSuperview];
        log!("OpenFeint HLE: dismissed dashboard overlay");
    }
}

fn dashboard_achievements_text(env: &mut Environment) -> String {
    let mut defs: Vec<(u32, String, String, u32)> = with_state(|s| {
        s.defs
            .values()
            .map(|d| (d.position, d.title.clone(), d.description.clone(), d.gamerscore))
            .collect()
    });
    if defs.is_empty() {
        // No offline config: fall back to a progress summary.
        let (total, unlocked) = with_state(|s| {
            (
                s.achievements.len(),
                s.achievements
                    .values()
                    .filter(|e| e.percent_ge_100())
                    .count(),
            )
        });
        if total == 0 {
            return "No achievements unlocked yet.\nAchievements unlock during gameplay.".to_string();
        }
        return format!(
            "{}/{} achievements unlocked.\nKeep playing to unlock the rest!",
            unlocked, total
        );
    }
    defs.sort();
    let unlocked_count = with_state(|s| {
        s.achievements
            .iter()
            .filter(|(_, e)| e.percent_ge_100())
            .count()
    });
    let total = defs.len();
    let mut text = format!("\u{2713} {}/{} unlocked\n\n", unlocked_count, total);
    for (_, title, description, gamerscore) in defs {
        text.push_str(&format!(
            "\u{25A1} {} ({}G)\n{}\n\n",
            title, gamerscore, description
        ));
    }
    text
}

fn dashboard_leaderboards_text(env: &mut Environment) -> String {
    let boards: Vec<(String, String)> = with_state(|s| s.boards.clone());
    if boards.is_empty() {
        return "No leaderboards available.".to_string();
    }
    let scores: Vec<(String, i64)> = with_state(|s| {
        s.highscores
            .iter()
            .map(|(k, v)| (k.clone(), v.score))
            .collect()
    });
    let mut text = String::new();
    for (id, name) in &boards {
        let best = scores
            .iter()
            .find(|(k, _)| k == id)
            .map(|(_, v)| *v)
            .unwrap_or(0);
        text.push_str(&format!("{}\nBest: {}\n\n", name, best));
    }
    text
}

// MARK: - Message interpose

/// Returns true if the message was handled by the OpenFeint HLE and the
/// caller (objc_msgSend_inner) should return immediately.
pub fn try_openfeint_interpose(
    env: &mut Environment,
    _receiver: id,
    selector: SEL,
    orig_class: crate::objc::Class,
) -> bool {
    let sel = selector.as_str(&env.mem).to_string();
    // Cheap pre-filter before touching the class metadata.
    let mut handled = matches!(
        sel.as_str(),
        "initializeWithProductKey:andSecret:andDisplayName:andSettings:andDelegates:"
            | "initializeWithProductKey:andDelegates:"
            | "isOnline"
            | "isUserLoggedIn"
            | "updateAchievement:andPercentComplete:andShowNotification:"
            | "updateAchievement:andPercentComplete:andShowNotification:onSuccessInvocation:onFailureInvocation:"
            | "updateAchievements:withPercentCompletes:onSuccessInvocation:onFailureInvocation:"
            | "queueUpdateAchievement:andPercentComplete:andShowNotification:"
            | "queueUpdateAchievement:andPercentComplete:andShowNotification:onSuccessInvocation:onFailureInvocation:"
            | "localUpdateAchievement:forUser:andPercentComplete:"
            // OpenFeint 2.x API (e.g. Fruit Ninja 1.x):
            | "unlockAchievement:"
            | "unlockAchievement:onSuccess:onFailure:"
            | "unlockAchievements:onSuccess:onFailure:"
            | "queueUnlockedAchievement:"
            | "submitQueuedUnlockedAchievements:onFailure:"
            | "alreadyUnlockedAchievement:forUser:"
            | "setAutomaticallyPromptToPostUnlocks:"
            | "setAutomaticallyPromptToPostUnlocks"
            // Official SDK (github.com/Plingot/OpenFeint-iOS-Framework):
            | "updateAchievement:andPercentComplete:"
            | "updateAchievement:andPercentComplete:andShowNotification:onSuccess:onFailure:"
            | "submitQueuedUpadteAchievements:onFailure:"
            | "hasUserApprovedFeint"
            | "userDidApproveFeint:"
            | "versionNumber"
    );
    if !handled && sel.starts_with("setHighScore") {
        handled = true;
    }
    if !handled
        && (sel.starts_with("launchDashboard") || sel == "presentDashboard" || sel == "dismissDashboard")
    {
        handled = true;
    }
    if !handled {
        return false;
    }
    let class_name = env.objc.get_class_name(orig_class).to_owned();
    let is_score_service =
        class_name == "OFHighScoreService" && sel.starts_with("setHighScore");
    if class_name != "OpenFeint"
        && class_name != "OFAchievementService"
        && !is_score_service
    {
        return false;
    }

    // Raw argument registers (r2, r3) and stack arguments, captured before
    // any nested host call can clobber them.
    let arg2 = env.cpu.regs()[2];
    let arg3 = env.cpu.regs()[3];
    let sp = env.cpu.regs()[13];
    let stack_arg = |env: &Environment, index: usize| -> u32 {
        env.mem.read(crate::mem::ConstPtr::<u32>::from_bits(sp + (index as u32) * 4))
    };

    match (class_name.as_str(), sel.as_str()) {
        ("OpenFeint", "initializeWithProductKey:andSecret:andDisplayName:andSettings:andDelegates:")
        | ("OpenFeint", "initializeWithProductKey:andDelegates:") => {
            with_state(|s| s.initialized = true);
            load_offline_config(env);
            log!(
                "OpenFeint HLE: initialized for {} (offline local achievements mode)",
                env.bundle.bundle_identifier()
            );
            env.cpu.regs_mut()[0] = 1;
            env.cpu.regs_mut()[1] = 0;
            true
        }
        ("OpenFeint", "hasUserApprovedFeint") => {
            // Pretend the user has approved the (now defunct) OpenFeint
            // terms so games do not gate their social features.
            env.cpu.regs_mut()[0] = 1;
            env.cpu.regs_mut()[1] = 0;
            true
        }
        ("OpenFeint", "userDidApproveFeint:") => {
            env.cpu.regs_mut()[0..2].fill(0);
            true
        }
        ("OpenFeint", "versionNumber") => {
            // Report an OpenFeint 2.10-ish version.
            env.cpu.regs_mut()[0] = 0x0002_0A00;
            env.cpu.regs_mut()[1] = 0;
            true
        }
        ("OpenFeint", "dismissDashboard") => {
            dismiss_dashboard(env);
            env.cpu.regs_mut()[0..2].fill(0);
            true
        }
        (_, _) if class_name == "OpenFeint"
            && (sel.starts_with("launchDashboard") || sel == "presentDashboard") =>
        {
            let page = if sel.contains("Achievement") {
                "achievements"
            } else if sel.contains("Leaderboard") || sel.contains("Highscore") {
                "leaderboards"
            } else {
                "achievements"
            };
            show_dashboard(env, page);
            env.cpu.regs_mut()[0..2].fill(0);
            true
        }
        ("OpenFeint", "isOnline") | ("OpenFeint", "isUserLoggedIn") => {
            // Behave as "online and logged in" so games do not disable
            // their achievement/leaderboard features. All networked
            // services are faked, so no actual traffic happens.
            env.cpu.regs_mut()[0] = 1;
            env.cpu.regs_mut()[1] = 0;
            true
        }
        ("OFAchievementService", "updateAchievement:andPercentComplete:andShowNotification:")
        | (
            "OFAchievementService",
            "updateAchievement:andPercentComplete:andShowNotification:onSuccessInvocation:onFailureInvocation:",
        )
        | (
            "OFAchievementService",
            "queueUpdateAchievement:andPercentComplete:andShowNotification:",
        )
        | (
            "OFAchievementService",
            "queueUpdateAchievement:andPercentComplete:andShowNotification:onSuccessInvocation:onFailureInvocation:",
        )
        | (
            "OFAchievementService",
            "localUpdateAchievement:forUser:andPercentComplete:",
        ) => {
            // armv7 AAPCS: id goes in r2, BOOL in r3, the float
            // percentComplete is passed in VFP s0, and any further arguments
            // go on the stack.
            let ach_id_obj: id = Ptr::from_bits(arg2);
            // The official header declares `andPercentComplete:(double)`,
            // which an armv7 AAPCS caller passes in d0; some SDK builds used
            // a plain `float` in s0. Distinguish by value sanity.
            let s0 = env.cpu.ext_reg(0);
            let as_f32 = f32::from_bits(s0);
            let as_f64 = f64::from_bits(((env.cpu.ext_reg(1) as u64) << 32) | s0 as u64);
            let percent = if as_f32 > 0.0 && as_f32 <= 100.5 {
                as_f32
            } else if as_f64 > 0.0 && as_f64 <= 100.5 {
                as_f64 as f32
            } else {
                as_f32.max(0.0).min(100.0)
            };
            let show_notification = arg3 != 0;
            let ach_id =
                crate::frameworks::foundation::ns_string::to_rust_string(env, ach_id_obj).to_string();
            record_progress(env, &ach_id, percent, show_notification);
            env.cpu.regs_mut()[0..2].fill(0);
            true
        }
        (
            "OFAchievementService",
            "updateAchievements:withPercentCompletes:onSuccessInvocation:onFailureInvocation:",
        ) => {
            let ids: id = Ptr::from_bits(arg2);
            let percents: id = Ptr::from_bits(arg3);
            let show_notification = true;
            let count: crate::frameworks::foundation::NSUInteger = msg![env; ids count];
            for i in 0..count {
                let ach_id_obj: id = msg![env; ids objectAtIndex:(i)];
                let percent_number: id = msg![env; percents objectAtIndex:(i)];
                let percent_f64: f64 = msg![env; percent_number doubleValue];
                let ach_id =
                    crate::frameworks::foundation::ns_string::to_rust_string(env, ach_id_obj)
                        .to_string();
                record_progress(env, &ach_id, percent_f64 as f32, show_notification);
            }
            env.cpu.regs_mut()[0..2].fill(0);
            true
        }
        // ---- OpenFeint 2.x API (older games, e.g. Fruit Ninja 1.x) ----
        ("OFAchievementService", "unlockAchievement:")
        | ("OFAchievementService", "unlockAchievement:onSuccess:onFailure:") => {
            // r2 = achievement id (NSString*); r3 / stack[0] are the
            // success/failure callbacks (faked, so ignored).
            let ach_id_obj: id = Ptr::from_bits(arg2);
            let ach_id =
                crate::frameworks::foundation::ns_string::to_rust_string(env, ach_id_obj).to_string();
            record_progress(env, &ach_id, 100.0, true);
            env.cpu.regs_mut()[0..2].fill(0);
            true
        }
        ("OFAchievementService", "unlockAchievements:onSuccess:onFailure:") => {
            let ids: id = Ptr::from_bits(arg2);
            let count: crate::frameworks::foundation::NSUInteger = msg![env; ids count];
            for i in 0..count {
                let ach_id_obj: id = msg![env; ids objectAtIndex:(i)];
                let ach_id = crate::frameworks::foundation::ns_string::to_rust_string(env, ach_id_obj)
                    .to_string();
                record_progress(env, &ach_id, 100.0, true);
            }
            env.cpu.regs_mut()[0..2].fill(0);
            true
        }
        ("OFAchievementService", "queueUnlockedAchievement:") => {
            // Queued unlocks are recorded silently; the banner is shown when
            // they are "submitted" (see below).
            let ach_id_obj: id = Ptr::from_bits(arg2);
            let ach_id =
                crate::frameworks::foundation::ns_string::to_rust_string(env, ach_id_obj).to_string();
            record_progress(env, &ach_id, 100.0, false);
            env.cpu.regs_mut()[0..2].fill(0);
            true
        }
        ("OFAchievementService", "submitQueuedUnlockedAchievements:onFailure:")
        | ("OFAchievementService", "submitQueuedUpadteAchievements:onFailure:") => {
            with_state(|s| {
                for (id, e) in s.achievements.iter_mut() {
                    if e.percent >= 100.0 {
                        e.notified = false;
                    }
                    let _ = id;
                }
            });
            save_store(env);
            env.cpu.regs_mut()[0..2].fill(0);
            true
        }
        ("OFAchievementService", "alreadyUnlockedAchievement:forUser:") => {
            // r2 = achievement id, r3 = user (ignored). Returns whether the
            // achievement is already unlocked in the local store.
            let ach_id_obj: id = Ptr::from_bits(arg2);
            let ach_id =
                crate::frameworks::foundation::ns_string::to_rust_string(env, ach_id_obj).to_string();
            load_store(env);
            let unlocked = with_state(|s| {
                s.achievements
                    .get(&ach_id)
                    .map(|e| e.percent >= 100.0)
                    .unwrap_or(false)
            });
            env.cpu.regs_mut()[0] = unlocked as u32;
            env.cpu.regs_mut()[1] = 0;
            true
        }
        ("OFAchievementService", "setAutomaticallyPromptToPostUnlocks:")
        | ("OFAchievementService", "setAutomaticallyPromptToPostUnlocks") => {
            // Void setter; consume the call.
            env.cpu.regs_mut()[0..2].fill(0);
            true
        }
        ("OFHighScoreService", _s) if _s.starts_with("setHighScore") => {
            // `+setHighScore:(int64_t)score ... forLeaderboard:(NSString*)...`
            // AAPCS: the 64-bit score occupies the aligned r2/r3 pair; all
            // remaining parameters spill to the stack starting at slot 0.
            let score = ((arg3 as u64) << 32) | (arg2 as u64);
            let parts: Vec<&str> = sel.split(':').collect();
            // Parameter k (1-based) has name parts[k-1]; parameters >= 2 are
            // at stack slot (k - 2) because the 64-bit score ate r2 and r3.
            let param_idx = |name: &str| -> Option<usize> {
                parts
                    .iter()
                    .position(|p| *p == name || p.ends_with(name))
                    .map(|idx| idx.checked_sub(1))
                    .flatten()
            };
            let mut read_str = |env: &mut Environment, idx: Option<usize>| -> Option<String> {
                let idx = idx?;
                let p = stack_arg(env, idx);
                if p == 0 {
                    return None;
                }
                let obj: id = Ptr::from_bits(p);
                if env.objc.get_host_object(obj).is_none() {
                    return None;
                }
                let cls: Class = msg![env; obj class];
                if cls == nil {
                    return None;
                }
                let name = env.objc.get_class_name(cls).to_owned();
                if name.contains("String") {
                    Some(
                        crate::frameworks::foundation::ns_string::to_rust_string(env, obj)
                            .to_string(),
                    )
                } else {
                    None
                }
            };
            let lb = read_str(env, param_idx("forLeaderboard")).unwrap_or_default();
            let display = read_str(env, param_idx("withDisplayText")).unwrap_or_default();
            record_high_score(env, &lb, score as i64, &display);
            env.cpu.regs_mut()[0..2].fill(0);
            true
        }
        _ => false,
    }
}

fn record_high_score(env: &mut Environment, leaderboard_id: &str, score: i64, display: &str) {
    load_scores(env);
    let is_new = with_state(|s| {
        let entry = s
            .highscores
            .entry(leaderboard_id.to_string())
            .or_insert_with(|| HighScoreEntry {
                score: i64::MIN,
                display: String::new(),
            });
        if score > entry.score {
            entry.score = score;
            if !display.is_empty() {
                entry.display = display.to_string();
            }
            true
        } else {
            false
        }
    });
    if is_new {
        save_scores(env);
        log!(
            "OpenFeint HLE: new high score {} on leaderboard {:?} ({})",
            score,
            leaderboard_id,
            display
        );
    }
}

fn load_scores(env: &mut Environment) {
    {
        let Ok(mut file) = env.fs.open(crate::fs::GuestPath::new(SCORES_FILE)) else {
            return;
        };
        let mut buf = Vec::new();
        if std::io::Read::read_to_end(&mut file, &mut buf).is_err() {
            return;
        }
        let text = String::from_utf8_lossy(&buf).into_owned();
        for line in text.lines() {
            let mut parts = line.split('\t');
            let (Some(lb), Some(score_str)) = (parts.next(), parts.next()) else {
                continue;
            };
            let Ok(score) = score_str.parse::<i64>() else {
                continue;
            };
            let display = parts.next().unwrap_or("").to_string();
            with_state(|s| {
                s.highscores.insert(
                    lb.to_string(),
                    HighScoreEntry { score, display },
                );
            });
        }
    }
}

fn save_scores(env: &mut Environment) {
    let text = with_state(|s| {
        let mut lines: Vec<String> = s
            .highscores
            .iter()
            .map(|(k, v)| {
                format!("{}\t{}\t{}", k, v.score, v.display.replace('\t', " "))
            })
            .collect();
        lines.sort();
        lines.join("\n")
    });
    if env
        .fs
        .create_dir_all(crate::fs::GuestPath::new(STORE_DIR))
        .is_err()
    {
        log!("Warning: OpenFeint HLE: could not create store directory {}", STORE_DIR);
        return;
    }
    let mut options = crate::fs::GuestOpenOptions::new();
    options.write().create().truncate();
    match env
        .fs
        .open_with_options(crate::fs::GuestPath::new(SCORES_FILE), options)
    {
        Ok(mut file) => {
            if let Err(e) = std::io::Write::write_all(&mut file, text.as_bytes()) {
                log!("Warning: OpenFeint HLE: could not save high scores: {}", e);
            }
        }
        Err(e) => log!("Warning: OpenFeint HLE: could not save high scores: {:?}", e),
    }
}
