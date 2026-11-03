# PhotoCraft iPad bundle

This Xcode target packages the Rust `photocraft-ios` static library. It contains no
Swift or Objective-C product source. Open `PhotoCraft.xcodeproj` and use the shared
`PhotoCraft` scheme. Set your development team in Xcode for physical-device signing.

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
xcodebuild -project packaging/ios/PhotoCraft.xcodeproj -scheme PhotoCraft \
  -configuration Debug -destination 'generic/platform=iOS Simulator' build
python3 -m unittest discover -s packaging/ios/tests -v
python3 packaging/ios/validate-bundle.py path/to/PhotoCraft.app
cargo xtask ios --sim
```

`xtask ios --sim` builds, boots, installs, and launches through the simulator CLI.
For a physical iPad pass `--device --device-id <udid> --team <id>`; `--bundle-id <id>` overrides
the bundle identifier for both build and launch when the product id cannot be registered to your
team. It does not require a desktop viewer. Xcode 27 provides Device Hub;
`--show-simulator` currently targets the older Simulator application.

The build phase resolves Cargo from `CARGO` or `$HOME/.cargo/bin` and builds only
`--lib`. It maps arm64 device, arm64 Simulator, and Intel Simulator architectures
and creates a universal archive when Xcode requests multiple architectures.
Debug uses Cargo's debug profile; Release uses `--release`. Artifacts default to
`target/ios-xcode`; `CARGO_TARGET_DIR` can override that location. Paths containing
spaces are supported. Rust targets must be installed before building.

The library must export Apple's ordinary C entry symbol `main`:
`extern "C" fn main(argc: i32, argv: *const *const c_char) -> i32`.
A dedicated link phase invokes Apple’s clang driver because Xcode skips its
standard link phase for targets with no C/Swift/Objective-C sources. It links each
requested architecture using Xcode’s SDK and deployment target, then creates the
bundle executable with lipo before signing. The linker pulls the Rust entry symbol
from the archive with `-u _main`. Release builds generate a dSYM.
The bundle validator checks that CFBundleExecutable exists, contains a Mach-O
EXECUTE header, and passes strict codesign verification. Use `--unsigned` only for
intentional unsigned builds. The runner owns
UIKit startup and must keep interop within its Apple bridge modules.

The plist permits iPad multitasking and declares opening documents in place.
The scene manifest declares one PhotoCraft window scene, required by iPadOS 27.
Multiple scenes remain disabled until independent window ownership is implemented.
Document type declarations advertise supported formats; native document pickers
are implemented, with provider selection and write-back acceptance still pending.
The entitlement file is deliberately empty: iOS already sandboxes applications;
macOS app-sandbox and user-selected-file entitlements do not belong here.

The asset catalog's `AppIcon` is the PhotoCraft kitsune from `assets/app-icon/`
(attributed in `ATTRIBUTION.md`), flattened onto the full-bleed `#2f7bf5` field with no
alpha channel, since iOS applies its own corner mask and rejects transparency. It is one
1024 pt image that Xcode scales to every iPad size.

## Carried patches over upstream

Some upstream code builds for macOS, Windows, Linux and wasm but not for iPadOS, so the port carries a local gate.
Re-check these after every upstream merge: if upstream rewrites the patched lines, the gate is lost silently and the
next device build fails with an error that does not point back here.

- **`crates/raster` — `sysinfo` is off the iOS build.** The RAM size and scratch-volume free space behind the memory
  limit and the spill check come from `sysinfo`, whose Apple backend calls libproc (`proc_pidinfo`,
  `PROC_PIDLISTFDS`, `proc_fdinfo`) — symbols iOS does not expose, so the crate fails to compile for
  `aarch64-apple-ios`. `Cargo.toml` gates the dependency on
  `cfg(all(not(target_arch = "wasm32"), not(target_os = "ios")))` (`rayon` stays: the iPad has cores to use), and the
  three call sites fall back to what the web build already does — `memory::measure()` returns `Snapshot::default()`
  (the explicit MB limit governs), `spill::free_space()` returns `None` (skips only the free-space check, not
  spilling), and `scratch_file`'s cross-process reap is skipped (iOS cannot enumerate processes to test liveness, and
  the scratch volume accepts the unlink-at-creation, so no abandoned names exist to reap).
  Gate check: `cargo check -p photocraft-raster --target aarch64-apple-ios` and the same for
  `aarch64-apple-ios-sim` (the Simulator is also `target_os = "ios"`) must stay clean, and so must the sync plan's
  Tier 1 `cargo check -p photocraft-ios --lib --target aarch64-apple-ios-sim`. Carried since 2026-11-03 (upstream
  v0.20.1 added the feature); the two raster checks passed and `cargo xtask ios --device` built, installed and
  launched on iPadOS 27.0.1. This belongs upstream and should go there so the gate stops being local.
