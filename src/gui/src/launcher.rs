/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Running an app, in its own process.
//!
//! The emulator is not something the frontend can call into. `Environment`
//! owns its process: it maps guest memory, drives an SDL event loop, and ends
//! a run by calling `std::process::exit`. One of those per process is the
//! design, and a library window that shares a process with a game would die
//! with it.
//!
//! So the frontend launches `tapHLE` the same way a person would from a
//! terminal, with the same arguments, and reads its output back through
//! pipes. That has four consequences worth stating, because they are the
//! reasons for the choice rather than side effects of it:
//!
//! * the emulator opens its own window, and the library window stays alive;
//! * a crash in a game cannot take the frontend with it, so the log and the
//!   diagnostics survive to be read afterwards;
//! * every option the frontend sets is an option the command line already
//!   accepts, so the two interfaces cannot drift apart;
//! * relaunching costs nothing, which is the whole developer loop.
//!
//! On Windows the child is created with no console (see [crate::process]), so
//! nothing flashes on screen.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

use crate::logstore::{self, LogLevel, LogOrigin, SharedLog};
use crate::process;

/// How a run ended.
#[derive(Clone, Debug, PartialEq)]
pub enum RunOutcome {
    /// The app closed itself, or the person closed its window.
    Finished,
    /// The emulator stopped early. The code is the process exit status.
    Failed { code: Option<i32> },
    /// The frontend asked it to stop.
    Stopped,
    /// The process could not be waited for at all.
    Lost(String),
}

impl RunOutcome {
    pub fn is_failure(&self) -> bool {
        matches!(self, RunOutcome::Failed { .. } | RunOutcome::Lost(_))
    }
}

/// A message from a run's watching thread.
pub enum RunEvent {
    Ended { run: u64, outcome: RunOutcome },
}

/// One app running, or one that has just stopped.
pub struct Run {
    pub id: u64,
    pub entry_id: String,
    pub app_name: Arc<str>,
    pub started: Instant,
    pub arguments: Vec<String>,
    child: Option<Child>,
}

impl Run {
    pub fn elapsed_seconds(&self) -> u64 {
        self.started.elapsed().as_secs()
    }
}

/// Everything currently running, and the plumbing that watches it.
pub struct Launcher {
    runs: Vec<Run>,
    next_id: u64,
    events: (Sender<RunEvent>, Receiver<RunEvent>),
    log: SharedLog,
}

impl Launcher {
    pub fn new(log: SharedLog) -> Self {
        Launcher {
            runs: Vec::new(),
            next_id: 1,
            events: channel(),
            log,
        }
    }

    pub fn running(&self) -> &[Run] {
        &self.runs
    }

    pub fn is_running(&self, entry_id: &str) -> bool {
        self.runs.iter().any(|run| run.entry_id == entry_id)
    }

    pub fn any_running(&self) -> bool {
        !self.runs.is_empty()
    }

    /// Start an app.
    ///
    /// `arguments` are the emulator options the settings asked for, and
    /// `environment` the variables. The working directory is the tapHLE
    /// installation, which is where the emulator looks for its resources.
    pub fn launch(
        &mut self,
        emulator: &Path,
        working_directory: &Path,
        entry_id: &str,
        app_name: &str,
        app_path: &Path,
        arguments: &[String],
        environment: &[(String, String)],
    ) -> Result<u64, String> {
        if !emulator.exists() {
            return Err(format!(
                "The tapHLE emulator was not found at {}. Set its location in \
                 Settings ▸ Paths.",
                emulator.display()
            ));
        }

        let mut command = Command::new(emulator);
        command
            .arg(app_path)
            .args(arguments)
            .current_dir(working_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (name, value) in environment {
            command.env(name, value);
        }
        let mut child = process::without_console(&mut command)
            .spawn()
            .map_err(|e| format!("Could not start {}: {e}", emulator.display()))?;

        let id = self.next_id;
        self.next_id += 1;
        let app_name: Arc<str> = Arc::from(app_name);

        let origin = LogOrigin::Run {
            id,
            app: app_name.clone(),
        };
        // Both streams are read on their own threads. A pipe that is not
        // drained fills up and blocks the writer, which would freeze the
        // emulator as soon as it had printed a few kilobytes.
        if let Some(stdout) = child.stdout.take() {
            spawn_reader(stdout, self.log.clone(), origin.clone());
        }
        if let Some(stderr) = child.stderr.take() {
            spawn_reader(stderr, self.log.clone(), origin.clone());
        }

        logstore::note(
            &self.log,
            LogLevel::Info,
            format!(
                "Launching {app_name}: {} {}",
                app_path.display(),
                arguments.join(" ")
            ),
        );

        self.runs.push(Run {
            id,
            entry_id: entry_id.to_string(),
            app_name,
            started: Instant::now(),
            arguments: arguments.to_vec(),
            child: Some(child),
        });
        Ok(id)
    }

    /// Ask a run to stop.
    ///
    /// There is no polite way to ask: the emulator has no control channel,
    /// and a guest app in a tight loop would ignore one anyway. The window's
    /// close button is the graceful path and this is the button for when
    /// that has stopped working, so it terminates.
    pub fn stop(&mut self, run_id: u64) {
        let Some(run) = self.runs.iter_mut().find(|run| run.id == run_id) else {
            return;
        };
        if let Some(child) = run.child.as_mut() {
            let name = run.app_name.clone();
            match child.kill() {
                Ok(()) => logstore::note(
                    &self.log,
                    LogLevel::Info,
                    format!("Stopped {name}."),
                ),
                Err(e) => logstore::note(
                    &self.log,
                    LogLevel::Warning,
                    format!("Could not stop {name}: {e}"),
                ),
            }
        }
    }

    pub fn stop_all(&mut self) {
        let ids: Vec<u64> = self.runs.iter().map(|run| run.id).collect();
        for id in ids {
            self.stop(id);
        }
    }

    /// Collect any run that has ended.
    ///
    /// Called once per frame. Waiting is non-blocking, so a running app never
    /// holds up the interface.
    pub fn poll(&mut self) -> Vec<(Run, RunOutcome)> {
        let mut ended = Vec::new();
        let mut index = 0;
        while index < self.runs.len() {
            let status = match self.runs[index].child.as_mut() {
                Some(child) => child.try_wait(),
                None => Ok(None),
            };
            match status {
                Ok(Some(status)) => {
                    let outcome = if status.success() {
                        RunOutcome::Finished
                    } else {
                        RunOutcome::Failed { code: status.code() }
                    };
                    ended.push((self.runs.remove(index), outcome));
                }
                Ok(None) => index += 1,
                Err(e) => {
                    ended.push((self.runs.remove(index), RunOutcome::Lost(e.to_string())));
                }
            }
        }
        // The event channel is kept for the reader threads' benefit even
        // though the exit status is collected here; draining it keeps it from
        // growing if a future change starts sending on it.
        while self.events.1.try_recv().is_ok() {}
        ended
    }

    /// A sender for anything that wants to report a run ending.
    pub fn event_sender(&self) -> Sender<RunEvent> {
        self.events.0.clone()
    }
}

fn spawn_reader<R: std::io::Read + Send + 'static>(
    stream: R,
    log: SharedLog,
    origin: LogOrigin,
) {
    std::thread::spawn(move || {
        let reader = BufReader::new(stream);
        // Bytes rather than lines: a guest app can print anything, and one
        // invalid UTF-8 sequence must not end the capture for the whole run.
        for line in reader.split(b'\n') {
            let Ok(bytes) = line else { break };
            let text = String::from_utf8_lossy(&bytes);
            logstore::append(&log, origin.clone(), text.trim_end_matches('\r'));
        }
    });
}

/// Where the emulator executable is.
///
/// The frontend and the emulator are built and installed side by side, so the
/// answer is almost always "next to this program". The current directory is
/// tried as well, which is what makes an unpacked release directory work when
/// the frontend was started from somewhere else.
pub fn find_emulator(configured: Option<&Path>, data_dir: &Path) -> Option<PathBuf> {
    if let Some(path) = configured {
        return path.exists().then(|| path.to_path_buf());
    }
    let name = if cfg!(windows) { "tapHLE.exe" } else { "tapHLE" };
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(name));
        }
    }
    candidates.push(data_dir.join(name));
    candidates.into_iter().find(|path| path.is_file())
}

/// The message shown when a run ends badly.
///
/// The exit codes are the emulator's own: it calls `exit(-1)` for an error it
/// has already explained in the log, and Rust's runtime uses 101 for a panic.
/// Naming them turns "it vanished" into something a person can act on.
pub fn explain_outcome(outcome: &RunOutcome, app: &str) -> String {
    match outcome {
        RunOutcome::Finished => format!("{app} closed."),
        RunOutcome::Stopped => format!("{app} was stopped."),
        RunOutcome::Lost(reason) => {
            format!("tapHLE lost track of {app}: {reason}")
        }
        RunOutcome::Failed { code } => {
            let detail = match code {
                Some(101) => {
                    " It hit something tapHLE has not implemented, or an internal check \
                     failed. The last lines of the log say which."
                }
                Some(-1) | Some(255) => {
                    " The emulator reported an error; the log says what it was."
                }
                _ => " The log's last lines are the best clue.",
            };
            let code = match code {
                Some(code) => format!(" (exit code {code})"),
                None => " (it was terminated)".to_string(),
            };
            format!("{app} stopped unexpectedly{code}.{detail}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clean_exit_is_not_a_failure() {
        assert!(!RunOutcome::Finished.is_failure());
        assert!(!RunOutcome::Stopped.is_failure());
        assert!(RunOutcome::Failed { code: Some(101) }.is_failure());
    }

    /// A crash message has to say something a person can act on. The panic
    /// exit code is the common case in compatibility work and deserves its
    /// own wording.
    #[test]
    fn a_panic_is_explained_as_a_missing_feature() {
        let text = explain_outcome(&RunOutcome::Failed { code: Some(101) }, "Ricky");
        assert!(text.contains("Ricky"));
        assert!(text.contains("not implemented"));
        assert!(text.contains("101"));
    }

    #[test]
    fn a_termination_without_a_code_is_still_explained() {
        let text = explain_outcome(&RunOutcome::Failed { code: None }, "Ricky");
        assert!(text.contains("terminated"));
    }

    /// A configured path that does not exist must not be returned, or every
    /// launch fails with a confusing error from the operating system.
    #[test]
    fn a_configured_emulator_must_exist() {
        let missing = Path::new("Z:/definitely/not/here/tapHLE.exe");
        assert_eq!(find_emulator(Some(missing), Path::new(".")), None);
    }
}
