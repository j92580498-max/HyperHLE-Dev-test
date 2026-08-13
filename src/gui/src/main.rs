/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! tapHLE's desktop frontend.
//!
//! See `dev-docs/gui-architecture.md` for why this is a separate program from
//! the emulator and why it is built the way it is.

// The crate is named after the project, which is not snake case.
#![allow(non_snake_case)]
// A frontend must not open a console window. This is unconditional rather
// than release-only so that a debug build behaves the way the shipped one
// does; the frontend's own diagnostics go to its log panel and to
// tapHLE_frontend/frontend_log.txt instead of to a terminal.
#![windows_subsystem = "windows"]

mod compat;
mod http;
mod launcher;
mod library;
mod logstore;
mod metadata;
mod process;
mod settings;
mod storage;
mod timefmt;
mod updates;

fn main() {
    // Replaced in the next commit by the eframe entry point.
}
