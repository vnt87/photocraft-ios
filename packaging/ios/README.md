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
It also works with CLI-only Xcode installations. Add `--show-simulator` to open the
desktop Simulator viewer when that application is installed.

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
The scene manifest is omitted for the runner’s app-delegate lifecycle; multiple
scenes require independent scene ownership before activation. Document type
declarations advertise supported formats; actual document-picker and incoming URL
handling remains the MAC-3 platform-services work.
The entitlement file is deliberately empty: iOS already sandboxes applications;
macOS app-sandbox and user-selected-file entitlements do not belong here.

The asset catalog is valid but has no app icon. Supply an approved, attributed
icon before App Store distribution; the harness does not fabricate product assets.
