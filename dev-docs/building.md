# Building tapHLE

Windows is tapHLE's only product target. The macOS build is retained as a
development convenience for debugging and behavioral comparison. Android is
not an active target.

If you are building to contribute, also read `../CONTRIBUTING.md` and the root
`AGENTS.md`.

## Windows prerequisites

Install:

- [Git](https://git-scm.com/), including Git Bash for the shell scripts;
- the stable [Rust toolchain](https://www.rust-lang.org/tools/install);
- Visual Studio 2022 Build Tools with “Desktop development with C++”;
- [CMake](https://cmake.org/) available on `PATH`; and
- Boost headers.

The native Dynarmic dependency currently expects Boost 1.81 in the repository
on Windows. Download Boost 1.81 and extract/rename its top-level directory to
`vendor/boost`, so `vendor/boost/boost/` exists.

Clone and initialize the native submodules:

```powershell
git clone https://github.com/ephun/tapHLE.git
cd tapHLE
git submodule update --init
```

## Build and run

From a Visual Studio developer PowerShell, or another shell with the compiler
and CMake available:

```powershell
cargo build
cargo run
```

For a release build:

```powershell
cargo build --release
.\target\release\tapHLE.exe --help
```

Running from the repository root lets tapHLE find `tapHLE_dylibs`,
`tapHLE_fonts`, and `tapHLE_default_options.txt`. To run elsewhere, copy those
resources beside the executable. CI uses `dev-scripts/make-windows-bundle.sh`
from Git Bash to assemble `tapHLE_windows_bundle`.

To run a game directly:

```powershell
.\target\release\tapHLE.exe "C:\path\to\Game.ipa"
```

Only use legally obtained, decrypted apps. Do not add them to the repository.

## Checks for contributors

Fast checks that do not require the custom iPhone OS test SDK:

```powershell
cargo metadata --no-deps --format-version 1
cargo fmt --all -- --check
cargo test --workspace --lib
cargo test -- --skip test_app
```

The full integration test rebuilds `tests/TestApp.app` and needs LLVM plus the
custom SDK described in `../tests/README.md`:

```powershell
cargo test
```

The shell helpers also run formatting, Clippy, and documentation checks when
their prerequisites are installed:

```sh
bash dev-scripts/format.sh --check
bash dev-scripts/lint.sh
```

## macOS development build

macOS is useful for comparing guest behavior with host Apple frameworks and
for debugging shared code, but it is not a tapHLE release target. Install Rust,
Git, CMake, a C/C++ toolchain, and Boost (for example, `brew install boost`),
then initialize submodules and use the same Cargo commands.

The inherited macOS bundle helper is:

```sh
dev-scripts/make-macos-bundle.sh \
    target/release/tapHLE \
    "$(cargo run --package tapHLE_version)" \
    "$(cargo run --package tapHLE_version -- --branding)"
```

macOS failures should not displace Windows game work unless they block a
debugging path needed for that work.

## Troubleshooting

- If CMake cannot find Boost, confirm `vendor/boost/boost/version.hpp` exists.
- If Cargo reports missing native source, rerun `git submodule update --init`.
- If the executable cannot open fonts or dynamic libraries, run it from the
  repository root or copy all three runtime resources beside it.
- If a game fails, preserve the full tapHLE log and follow
  `agent-workflow.md` before implementing unrelated framework stubs.

Generate internal Rust documentation with:

```powershell
cargo doc --workspace --no-deps --open
```
