# tapHLE

tapHLE is a high-level emulator for early iPhone OS applications. Its goal is
to make as many early iPhone games as possible run well on Windows.

## Want a game to work?

You can use your own coding agent to work on it. You do not need to know how to
program, and you do not need to wait for someone else to take your request.
tapHLE gives the agent rules, tests, and a step-by-step debugging guide.

**[Start here: help a game work with a coding agent](HELP_A_GAME.md)**

Opening an issue does not promise that another contributor will do the work.
It gives you and your agent a place to record the goal and avoid duplicate
work. Never upload an IPA, game files, or a raw log.

Instead of emulating an entire iPhone and operating system, tapHLE runs the
game's 32-bit ARM code and supplies its own implementations of frameworks such
as Foundation, UIKit, OpenGL ES, and OpenAL.

## Project direction

This fork is AI-development-led. Coding agents are first-class contributors
for investigation, implementation, testing, and documentation. Contributors
may choose the games they care about. Each game is a practical step toward
broader compatibility.

The priorities are intentionally practical:

1. Move one chosen game closer to working on Windows.
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

The first tapHLE release line is `0.3`; numbered alpha/beta/RC/stable builds are
Windows x86_64 releases, while green `trunk` builds are commit-identified
previews. The versioning and packaging policy is documented in
`dev-docs/releases.md`.

**[See the compatibility ratings (1–5 stars)](https://taphle.ephun.net/compatibility).**
That live database is the current record. The legacy JSON records remain only
until they are migrated. Results are
hash-checked for exact app builds. For an old game that is no longer sold, a
record may link to the exact Archive.org file that was tested. The file name,
hashes, and app version must all match. Read `compatibility/README.md` for the
full rules. These rules do not cover a game that is still sold, and the project
respects DMCA notices and rightsholder requests.

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

To open the graphical app picker, run the executable with no app path from the
repository (or unpacked Windows bundle) directory:

```powershell
.\target\release\tapHLE.exe
```

That directory must contain `tapHLE_dylibs`, `tapHLE_fonts`, and
`tapHLE_default_options.txt`. The picker scans `tapHLE_apps` for `.ipa` and
`.app` entries and labels them using each bundle's embedded display name.
Those app files are intentionally local and ignored by Git, so you can keep
any playtest targets there without committing or redistributing them. To
bypass the picker, launch a path directly:

```powershell
.\target\release\tapHLE.exe "C:\path\to\Game.ipa"
```

The version and numbered Windows release rules are in `dev-docs/releases.md`.

Run `.\target\release\tapHLE.exe --help` for command-line options. User options
can be placed in `tapHLE_options.txt`; `OPTIONS_HELP.txt` documents each option.
Guest save data is stored in `tapHLE_sandbox`.

## Contributing

If you want to use a coding agent for a game, start with `HELP_A_GAME.md`.
Agents must read `AGENTS.md`. Human contributors can find more detail in
`CONTRIBUTING.md`.

Game compatibility reports are especially useful when they identify the exact
game version, Windows environment, reproduction steps, and sanitized tapHLE
log. Read `compatibility/README.md` before referencing Archive.org or recording
a result in the compatibility database. Never attach an app binary or raw log.

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
