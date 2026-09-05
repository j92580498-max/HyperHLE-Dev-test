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
    let text = format!("Achievement Unlocked!\n{}", achievement_id);
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
