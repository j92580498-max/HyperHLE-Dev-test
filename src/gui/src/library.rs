/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The set of apps this installation knows about.
//!
//! An entry is keyed by the app's own identity — bundle identifier and
//! version — rather than by where its file happens to sit, so moving a file
//! keeps its settings, its rating and how long it has been played. The path
//! is recorded too, because something has to be launched, but it is data
//! about the entry rather than the entry's name.
//!
//! Nothing here modifies an app. Importing reads the bundle and records what
//! it said; it never copies, unpacks or rewrites the file.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::compat::{DatabaseSnapshot, LocalRating};
use crate::metadata::{self, AppMetadata, ReadApp};
use crate::settings::{EmulatorSettings, SortOrder};

/// One app in the library.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LibraryEntry {
    /// [AppMetadata::stable_id]: bundle identifier and version.
    pub id: String,
    pub path: PathBuf,
    pub metadata: AppMetadata,
    /// File name of the cached icon, if one was written.
    pub icon_cache: Option<String>,
    /// Unix seconds when the app entered the library.
    pub added: u64,
    pub last_played: Option<u64>,
    /// Total time the emulator has been running this app, in seconds.
    pub play_seconds: u64,
    pub play_count: u32,
    pub favorite: bool,
    /// Settings that apply to this app only. Anything unset is inherited.
    pub overrides: EmulatorSettings,
    /// This machine's own rating. Never sent anywhere.
    pub local_rating: LocalRating,
    /// Whether the file was there the last time the library was checked.
    /// Not stored: it is about this machine right now.
    #[serde(skip)]
    pub missing: bool,
}

impl LibraryEntry {
    pub fn title(&self) -> &str {
        self.metadata.title()
    }
}

/// Why an import did not add an app, in the terms the person needs.
#[derive(Debug, PartialEq)]
pub enum ImportOutcome {
    /// Added, along with anything that was wrong but not fatal — a missing
    /// icon, most often. Worth saying, not worth refusing the app over.
    Added { id: String, warnings: Vec<String> },
    /// The app is already in the library. `path_updated` says whether the
    /// entry was repointed at the newly given file, which is what happens
    /// when the old one has gone missing or the file has moved.
    Duplicate { id: String, path_updated: bool },
    /// The file is not something tapHLE opens.
    Unsupported { path: PathBuf, reason: String },
    /// It is the right kind of file but could not be read.
    Failed { path: PathBuf, reason: String },
}

impl ImportOutcome {
    pub fn is_success(&self) -> bool {
        matches!(self, ImportOutcome::Added { .. })
    }

    /// The library entry this outcome refers to, when there is one.
    pub fn entry_id(&self) -> Option<&str> {
        match self {
            ImportOutcome::Added { id, .. } | ImportOutcome::Duplicate { id, .. } => Some(id),
            _ => None,
        }
    }

    /// A sentence for the import report.
    pub fn describe(&self, library: &Library) -> String {
        match self {
            ImportOutcome::Added { id, warnings } => {
                let name = library.find(id).map_or(id.as_str(), |e| e.title());
                if warnings.is_empty() {
                    format!("Added {name}.")
                } else {
                    format!("Added {name}, but: {}", warnings.join(" "))
                }
            }
            ImportOutcome::Duplicate { id, path_updated } => {
                let name = library.find(id).map_or(id.as_str(), |e| e.title());
                if *path_updated {
                    format!("{name} was already in the library; its location was updated.")
                } else {
                    format!("{name} is already in the library.")
                }
            }
            ImportOutcome::Unsupported { path, reason } => {
                format!("{}: {reason}", file_label(path))
            }
            ImportOutcome::Failed { path, reason } => {
                format!("{}: {reason}", file_label(path))
            }
        }
    }
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Whether a path is the kind of thing tapHLE opens, and why not if it isn't.
///
/// This mirrors what the emulator's own bundle reader accepts, on purpose:
/// rather than letting a stray file produce the emulator's terse message,
/// the frontend says what it accepts.
pub fn check_supported(path: &Path) -> Result<(), String> {
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    match extension.as_deref() {
        Some("ipa") if path.is_file() => Ok(()),
        Some("ipa") => Err("this .ipa file could not be found".to_string()),
        Some("app") if path.is_dir() => Ok(()),
        Some("app") => Err("an .app bundle has to be a folder".to_string()),
        _ if path.is_dir() => Err(
            "this folder is not an .app bundle. Use File ▸ Add Folder to scan it for apps"
                .to_string(),
        ),
        _ if !path.exists() => Err("this file could not be found".to_string()),
        _ => Err("tapHLE opens .ipa files and .app bundles".to_string()),
    }
}

/// Everything in `library.json`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub entries: Vec<LibraryEntry>,
}

impl Library {
    pub fn find(&self, id: &str) -> Option<&LibraryEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut LibraryEntry> {
        self.entries.iter_mut().find(|entry| entry.id == id)
    }

    pub fn remove(&mut self, id: &str) -> Option<LibraryEntry> {
        let index = self.entries.iter().position(|entry| entry.id == id)?;
        Some(self.entries.remove(index))
    }

    /// Note which entries no longer have a file behind them.
    ///
    /// A missing app is kept rather than deleted: its settings, rating and
    /// play time are worth more than the tidiness, and a removable drive
    /// comes back.
    pub fn mark_missing(&mut self) {
        for entry in &mut self.entries {
            entry.missing = !entry.path.exists();
        }
    }

    /// Add an app that has already been read.
    ///
    /// `icon_dir` is where the icon is cached; a failure to write the cache
    /// is not a failure to import, since the icon can be read again later.
    ///
    /// Reading is the slow part — an `.ipa` is an archive, and its icon has
    /// to be decompressed and decoded — so [read_for_import] does it on a
    /// worker thread and the result is handed here.
    pub fn absorb(&mut self, result: ScanResult, icon_dir: &Path) -> ImportOutcome {
        let (canonical, read) = match result {
            ScanResult::Unsupported { path, reason } => {
                return ImportOutcome::Unsupported { path, reason }
            }
            ScanResult::Failed { path, reason } => return ImportOutcome::Failed { path, reason },
            ScanResult::Read { path, read } => (path, *read),
        };
        let warnings = read.warnings.clone();

        let id = read.metadata.stable_id();
        if let Some(existing) = self.find_mut(&id) {
            // The same app given again from a different place is not a
            // second copy. Repoint the entry when the old file has gone, so
            // moving a collection does not orphan its history.
            let path_updated = existing.path != canonical && !existing.path.exists();
            if path_updated {
                existing.path = canonical;
                existing.missing = false;
            }
            return ImportOutcome::Duplicate { id, path_updated };
        }

        let icon_cache = read.icon.as_ref().and_then(|icon| {
            let name = metadata::icon_cache_name(&id);
            metadata::write_icon_cache(icon_dir, &name, icon)
                .ok()
                .map(|()| name)
        });

        self.entries.push(LibraryEntry {
            id: id.clone(),
            path: canonical,
            metadata: read.metadata,
            icon_cache,
            added: crate::timefmt::now_seconds(),
            ..Default::default()
        });
        ImportOutcome::Added { id, warnings }
    }

    /// Record that a run finished.
    pub fn record_play(&mut self, id: &str, seconds: u64) {
        if let Some(entry) = self.find_mut(id) {
            entry.play_seconds += seconds;
            entry.play_count += 1;
            entry.last_played = Some(crate::timefmt::now_seconds());
        }
    }
}

/// An app read from disk, or the reason it could not be.
pub enum ScanResult {
    Unsupported { path: PathBuf, reason: String },
    Failed { path: PathBuf, reason: String },
    Read { path: PathBuf, read: Box<ReadApp> },
}

/// Read one app in preparation for adding it to a library.
///
/// This is the expensive half of an import and is meant to be run off the
/// interface thread.
pub fn read_for_import(path: &Path) -> ScanResult {
    if let Err(reason) = check_supported(path) {
        return ScanResult::Unsupported {
            path: path.to_path_buf(),
            reason,
        };
    }
    // A canonical path is what makes the same file given twice by different
    // routes — a shortcut, a relative path, a drag from a search result —
    // recognisable as the same file.
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    match metadata::read(&canonical) {
        Ok(read) => ScanResult::Read {
            path: canonical,
            read: Box::new(read),
        },
        Err(reason) => ScanResult::Failed {
            path: path.to_path_buf(),
            reason,
        },
    }
}

/// The app files directly inside a folder.
///
/// One level only, the same as the emulator's own app picker: a collection is
/// a folder of apps, and descending further would sweep up the contents of
/// every `.app` bundle it found.
pub fn scan_folder(folder: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    let entries = std::fs::read_dir(folder)
        .map_err(|e| format!("Could not read {}: {e}", folder.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if check_supported(&path).is_ok() {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

/// The folders scanned when the library is refreshed.
pub fn default_folders() -> Vec<PathBuf> {
    vec![crate::storage::data_dir().join(tapHLE::paths::APPS_DIR)]
}

/// What the library view is showing, in order.
///
/// Kept as a function over entries rather than as a property of the grid, so
/// that a list view, a search box or another filter is a change here and not
/// a rewrite of the view.
pub struct ViewFilter<'a> {
    pub search: &'a str,
    pub favorites_only: bool,
    pub sort: SortOrder,
    pub descending: bool,
}

pub fn visible_entries(
    library: &Library,
    filter: &ViewFilter<'_>,
    database: &DatabaseSnapshot,
) -> Vec<usize> {
    let needle = filter.search.trim().to_lowercase();
    let mut indices: Vec<usize> = library
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| !filter.favorites_only || entry.favorite)
        .filter(|(_, entry)| needle.is_empty() || matches_search(entry, &needle))
        .map(|(index, _)| index)
        .collect();

    indices.sort_by(|&a, &b| {
        let (a, b) = (&library.entries[a], &library.entries[b]);
        let ordering = match filter.sort {
            SortOrder::Title => title_key(a).cmp(&title_key(b)),
            SortOrder::Publisher => publisher_key(a).cmp(&publisher_key(b)),
            // Never played sorts last whichever way the order runs, because
            // "recently played" is a question about the ones that have been.
            SortOrder::RecentlyPlayed => b.last_played.cmp(&a.last_played),
            SortOrder::Compatibility => {
                let rating = |entry: &LibraryEntry| {
                    entry
                        .local_rating
                        .stars
                        .or_else(|| database.find(&entry.metadata.bundle_identifier)?.rating)
                        .unwrap_or(0)
                };
                rating(b).cmp(&rating(a))
            }
            SortOrder::DateAdded => b.added.cmp(&a.added),
        };
        ordering.then_with(|| title_key(a).cmp(&title_key(b)))
    });
    if filter.descending {
        indices.reverse();
    }
    indices
}

fn matches_search(entry: &LibraryEntry, needle: &str) -> bool {
    let metadata = &entry.metadata;
    [
        metadata.title(),
        &metadata.bundle_identifier,
        metadata.publisher.as_deref().unwrap_or(""),
        metadata.genre.as_deref().unwrap_or(""),
    ]
    .iter()
    .any(|field| field.to_lowercase().contains(needle))
}

/// Titles sort as a person reads them, ignoring case.
fn title_key(entry: &LibraryEntry) -> String {
    entry.title().to_lowercase()
}

/// An app with no recorded publisher sorts after those that have one, rather
/// than under an empty heading at the top.
fn publisher_key(entry: &LibraryEntry) -> (bool, String) {
    match entry.metadata.publisher.as_deref() {
        Some(publisher) if !publisher.trim().is_empty() => (false, publisher.to_lowercase()),
        _ => (true, String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str, id: &str) -> LibraryEntry {
        LibraryEntry {
            id: format!("{id}@1.0"),
            metadata: AppMetadata {
                display_name: title.to_string(),
                bundle_identifier: id.to_string(),
                bundle_version: "1.0".to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn library_of(entries: Vec<LibraryEntry>) -> Library {
        Library { entries }
    }

    #[test]
    fn only_ipa_files_and_app_folders_are_accepted() {
        assert!(
            check_supported(Path::new("game.ipa")).is_err(),
            "an .ipa that is not there is still not usable"
        );

        // A file that exists but is the wrong kind is the case where the
        // message has to say what tapHLE does accept; a path that is simply
        // absent gets the other message, tested below.
        let wrong_kind = std::env::temp_dir().join("tapHLE-gui-not-an-app.zip");
        std::fs::write(&wrong_kind, b"not an app").unwrap();
        let error = check_supported(&wrong_kind).unwrap_err();
        let _ = std::fs::remove_file(&wrong_kind);
        assert!(
            error.contains(".ipa") && error.contains(".app"),
            "the message should say what is accepted, got {error:?}"
        );
    }

    /// A missing file and an unsupported file are different problems and
    /// need different messages, or the person cannot tell a typo from an
    /// unsupported format.
    #[test]
    fn a_missing_file_says_so() {
        let error = check_supported(Path::new("definitely-not-here.bin")).unwrap_err();
        assert!(error.contains("could not be found"));
    }

    #[test]
    fn search_covers_the_fields_a_person_would_type() {
        let mut game = entry("Baby Monkey", "com.kihon.babymonkey");
        game.metadata.publisher = Some("Kihon".to_string());
        let library = library_of(vec![game]);
        let database = DatabaseSnapshot::default();
        for needle in ["monkey", "KIHON", "com.kihon"] {
            let filter = ViewFilter {
                search: needle,
                favorites_only: false,
                sort: SortOrder::Title,
                descending: false,
            };
            assert_eq!(
                visible_entries(&library, &filter, &database).len(),
                1,
                "searching for {needle:?} should find the app"
            );
        }
    }

    #[test]
    fn titles_sort_without_regard_to_case() {
        let library = library_of(vec![
            entry("zebra", "com.a"),
            entry("Apple", "com.b"),
            entry("banana", "com.c"),
        ]);
        let filter = ViewFilter {
            search: "",
            favorites_only: false,
            sort: SortOrder::Title,
            descending: false,
        };
        let order = visible_entries(&library, &filter, &DatabaseSnapshot::default());
        let titles: Vec<&str> = order.iter().map(|&i| library.entries[i].title()).collect();
        assert_eq!(titles, ["Apple", "banana", "zebra"]);
    }

    /// The database rating stands in when this machine has no opinion, so
    /// sorting by compatibility works before anyone has rated anything.
    #[test]
    fn compatibility_sorting_falls_back_to_the_database() {
        let library = library_of(vec![entry("Low", "com.low"), entry("High", "com.high")]);
        let database = crate::compat::parse_snapshot(
            r#"{"apps":[
                {"app_id":1,"name":"Low","rating":1,
                 "extra":{"bundle_identifier":"com.low"},"url":"/a/1"},
                {"app_id":2,"name":"High","rating":5,
                 "extra":{"bundle_identifier":"com.high"},"url":"/a/2"}]}"#,
            0,
        )
        .unwrap();
        let filter = ViewFilter {
            search: "",
            favorites_only: false,
            sort: SortOrder::Compatibility,
            descending: false,
        };
        let order = visible_entries(&library, &filter, &database);
        assert_eq!(library.entries[order[0]].title(), "High");
    }

    /// A local rating is this machine's own answer and outranks the shared
    /// one for the purposes of this user's own view.
    #[test]
    fn a_local_rating_outranks_the_database_for_sorting() {
        let mut low = entry("Low", "com.low");
        low.local_rating.stars = Some(5);
        let library = library_of(vec![low, entry("High", "com.high")]);
        let database = crate::compat::parse_snapshot(
            r#"{"apps":[{"app_id":2,"name":"High","rating":4,
                 "extra":{"bundle_identifier":"com.high"},"url":"/a/2"}]}"#,
            0,
        )
        .unwrap();
        let filter = ViewFilter {
            search: "",
            favorites_only: false,
            sort: SortOrder::Compatibility,
            descending: false,
        };
        let order = visible_entries(&library, &filter, &database);
        assert_eq!(library.entries[order[0]].title(), "Low");
    }

    #[test]
    fn play_time_accumulates() {
        let mut library = library_of(vec![entry("A", "com.a")]);
        library.record_play("com.a@1.0", 60);
        library.record_play("com.a@1.0", 30);
        let entry = library.find("com.a@1.0").unwrap();
        assert_eq!(entry.play_seconds, 90);
        assert_eq!(entry.play_count, 2);
        assert!(entry.last_played.is_some());
    }

    /// Importing the same app twice must not create a second row; the
    /// library keys on the app's identity, not on where the file is.
    #[test]
    fn a_duplicate_is_recognized_by_identity() {
        let mut library = library_of(vec![entry("A", "com.a")]);
        assert!(library.find("com.a@1.0").is_some());
        let outcome = ImportOutcome::Duplicate {
            id: "com.a@1.0".to_string(),
            path_updated: false,
        };
        assert_eq!(outcome.entry_id(), Some("com.a@1.0"));
        assert!(!outcome.is_success());
        assert!(outcome
            .describe(&library)
            .contains("already in the library"));
    }
}
