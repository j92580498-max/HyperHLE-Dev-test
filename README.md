# tapHLE

tapHLE is a high-level emulator for early iPhone OS applications, with one
narrow goal: make selected games run well on a Windows PC.

Instead of emulating an entire iPhone and operating system, tapHLE runs the
game's 32-bit ARM code and supplies its own implementations of frameworks such
as Foundation, UIKit, OpenGL ES, and OpenAL.

## Project direction

This fork is AI-development-led. Coding agents are first-class contributors
for investigation, implementation, testing, and documentation, while the
maintainer supplies product direction and chooses the games that matter.

The priorities are intentionally practical:

1. Make a specific game work on Windows.
2. Iterate quickly from logs and observed behavior.
3. Protect working behavior with focused tests.
4. Improve architecture when it helps deliver compatibility.

Bounded compatibility workarounds are acceptable. Elegance is welcome, but a
large from-the-ground-up implementation is not a prerequisite for a useful
fix.

Windows is the only supported product target. macOS support is retained as a
convenient way to compile, debug, or compare behavior. The inherited Android
source remains in the repository, but Android development and releases are out
of scope.

## Status

tapHLE is experimental. Compatibility is specific to an exact game version,
and many applications will not work yet. The project does not include games,
Apple software, decryption keys, or other proprietary material.

Exact, hash-verified results are published in [the compatibility
database](COMPATIBILITY.md). For unavailable historical builds, tapHLE follows
a narrow maintainer-reviewed testing policy documented in
`compatibility/README.md`. That policy permits canonical Archive.org provenance
references in qualifying records; it is not a blanket legal conclusion about
"abandonware," does not cover apps with a current market alternative, and
requires respect for DMCA notices and rightsholder requests.

The project is not affiliated with or endorsed by Apple Inc. iPhone, iOS,
iPod, iPod touch, and iPad are Apple trademarks.

## Build on Windows

The full prerequisites and troubleshooting notes are in
`dev-docs/building.md`. At a high level, install Git, Rust, CMake, a C/C++
toolchain, and Boost, then run:

```powershell
git clone --recurse-submodules https://github.com/ephun/tapHLE.git
cd tapHLE
cargo build --release
```

The executable is `target\release\tapHLE.exe`. A distributable directory also
needs `tapHLE_dylibs`, `tapHLE_fonts`, and `tapHLE_default_options.txt`; the
Windows bundle script assembles those files in CI.

Put locally authorized `.ipa` files or `.app` bundles in `tapHLE_apps` to use
the graphical app picker, or launch a path directly:

```powershell
.\target\release\tapHLE.exe "C:\path\to\Game.ipa"
```

Run `.\target\release\tapHLE.exe --help` for command-line options. User options
can be placed in `tapHLE_options.txt`; `OPTIONS_HELP.txt` documents each option.
Guest save data is stored in `tapHLE_sandbox`.

## Contributing

Start with `AGENTS.md` if you are a coding agent or are working with one. Human
contributors should read `CONTRIBUTING.md`; both documents describe the same
Windows-first, evidence-driven workflow.

Game compatibility reports are especially useful when they identify the exact
game version, Windows environment, reproduction steps, and sanitized tapHLE
log. Read `compatibility/README.md` before referencing Archive.org or updating
the compatibility database. Never attach an app binary or raw log.

## Origin and license

tapHLE is a fork of the
[touchHLE project](https://github.com/touchHLE/touchHLE). Upstream deserves
credit for the emulator architecture and the substantial implementation this
fork began with; tapHLE has independent goals and contribution policies.

tapHLE modifications are copyright their respective contributors. The
inherited code remains copyright the touchHLE project contributors and other
authors identified in the source and bundled notices.

The emulator source is licensed under the Mozilla Public License 2.0. Due to
dependency license compatibility, distributed binaries are licensed under the
GNU General Public License version 3 or later. Bundled dynamic libraries and
fonts have their own notices in `tapHLE_dylibs` and `tapHLE_fonts`.
