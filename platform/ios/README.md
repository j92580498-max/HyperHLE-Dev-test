# tapHLE host for modern iOS

This directory contains tapHLE's experimental native host for modern iPhones
and iPads. The host runs supported early, 32-bit iPhone OS apps through
tapHLE's existing high-level emulation and Dynarmic CPU backend. It does not
emulate modern iOS guest apps.

The deployment target is iOS 17.4. JIT is currently required whenever the host
starts as a new process; there is no no-JIT ARM interpreter yet. Games, Apple
software, signing credentials, provisioning profiles, pairing files, and save
data are not included.

## Provenance and credit

The native host was adapted from johnny901901901's MPL-2.0-licensed
[`5a887ada` iOS port commit](https://github.com/johnny901901901/touchHLE/commit/5a887ada747899394f67b65817067e561b33f95f).
That work was built on touchHLE and retains touchHLE's original history and
credits. tapHLE's integration preserves the source author's Git authorship,
adapts the active product branding, and omits the source fork's release feed,
screenshots, and unrelated app-picker styling.

The host pins two dependency commits from the same author:

- `johnny901901901/rust-sdl2` commit `c4422f6481f3748a65e489b357f2121dc45cdfc0`;
- its nested `johnny901901901/SDL` commit
  `043ddb7af6555f0765ea30845855fa932ac7ab98`.

The first is MIT-licensed and the second retains SDL's zlib license. Their Git
histories and license notices remain in the recursive submodules.

The original port author reported a physical-device test on an iPhone 16 Pro
running iOS 27 beta 4. That is upstream evidence, not a tapHLE compatibility
claim. Any tapHLE result must be rerun from the tapHLE commit on the exact
physical device and iOS version being reported.

## Current host features

- A SwiftUI library for importing and launching local `.ipa` files.
- Guest title, icon, and orientation metadata in the library.
- Persistent settings and per-game save directories.
- Portrait and landscape launch handling.
- An in-game exit control that returns to the library.
- An optional FPS overlay.
- A shortcut to the `stikdebug://enable-jit` URL scheme.

## Source prerequisites

- macOS with Xcode 26 or newer and its command-line tools.
- Stable Rust installed with rustup.
- CMake, Ninja, and Boost headers.

Initialize all submodules recursively because the pinned rust-sdl2 checkout
contains the pinned SDL checkout:

```sh
git submodule update --init --recursive
```

Homebrew can install the non-Xcode prerequisites:

```sh
brew install cmake ninja boost
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim
export TAPHLE_BOOST_ROOT="$(brew --prefix boost)/include"
```

If Xcode is not installed at `/Applications/Xcode.app`, set `DEVELOPER_DIR` to
the active Xcode developer directory.

## Build

Build a simulator host for compile-time checking:

```sh
sh platform/ios/scripts/build-host.sh iphonesimulator Debug
```

Build an unsigned physical-device host:

```sh
sh platform/ios/scripts/build-host.sh iphoneos Release
```

To build and run from Xcode instead, first build the Rust static library:

```sh
sh platform/ios/scripts/build-rust.sh aarch64-apple-ios Debug
```

Then open `platform/ios/TapHLEHost.xcodeproj`, select the `TapHLEHost` target,
choose your own signing team and bundle identifier if necessary, select a
physical iPhone or iPad, and run it. Never commit Xcode signing data or
`xcuserdata`.

A successful simulator build is not device evidence. Dynarmic JIT behavior,
orientation, touch, audio, and rendering must be checked on a physical device.

## JIT

Dynarmic requires executable memory, so installing the host is not sufficient.
Enable JIT again whenever tapHLE starts as a new process. The library's bolt
button opens StikDebug using its URL scheme; install and configure a compatible
JIT-enablement tool separately. Pairing files and signing credentials are
private device material and must never be committed or attached to reports.

## Import apps and preserve saves

Use **Import Game** to select a lawfully obtained, decrypted 32-bit `.ipa` from
Files. Imported apps are copied to:

```text
Documents/tapHLE_apps
```

Guest save data is stored under:

```text
Documents/tapHLE_sandbox/<guest-bundle-id>
```

Removing a title from the library removes its imported app but deliberately
keeps its save directory. Deleting tapHLE from the device can remove the whole
container, so back up saves through Files first.

Compatibility is app-version and host-specific. Check the
[tapHLE compatibility database](https://taphle.ephun.net/compatibility), but do
not treat a Windows result as proof of iOS-host behavior. Follow the repository
artifact and reporting rules before testing or publishing a result.

## Diagnostics

Runtime output is written to:

```text
Documents/taphle-host.log
```

Before sharing an excerpt, remove game names when sensitive, local paths, and
personal data. Never attach apps, extracted assets, saves, pairing files,
certificates, provisioning profiles, signed IPAs, or device backups.

## Package an unsigned IPA

After an unsigned Release device build:

```sh
sh platform/ios/scripts/package-ipa.sh
sh platform/ios/scripts/verify-ipa.sh
```

The scripts create and verify `dist/tapHLE-iOS-unsigned.ipa` plus its SHA-256.
They refuse signed apps and common signing or pairing material. Release tags
and public release packaging still require the authorization and checks in
`dev-docs/releases.md`.

## License

tapHLE source is licensed under MPL-2.0. Binary distribution is GPL-3.0-or-later
because of dependency license compatibility. Preserve the repository license,
the touchHLE contributor history, the original iOS-port authorship above, and
all dependency notices. tapHLE is not affiliated with Apple.
