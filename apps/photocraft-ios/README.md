# PhotoCraft iPadOS runner

This crate provides the Rust static library linked by the Xcode harness in
`packaging/ios`. The Apple-only entry bridge exports `main` and
`photocraft_ios_main`; the harness must force-load the entry symbol with
`-Wl,-u,_main`. UIKit startup and the Metal surface lifecycle are owned by
winit/eframe 0.36.2. The renderer allows only Metal.

The root viewport follows the operating system's size instead of assuming an
11-inch or 13-inch screen. No simulator or device size is hard-coded.

The bootstrap hides the iOS status bar through winit's safe window API. Its current
app-delegate lifecycle does not provide UIKit scenes for egui's scene-based
safe-area query, so a visible status bar would overlap the editor menu. This
immersive setting avoids a hard-coded inset; scene-aware safe-area integration
remains part of later iPadOS lifecycle work.

This is the MAC-2 bootstrap, not the MAC-3 platform service implementation.
File dialogs, file persistence, image clipboard access, and recovery are disabled.
An initial notice describes those limitations. Documents created in this runner
are session-only. Apple Pencil bridges and iPad-specific UI work remain separate.

On iOS, startup diagnostics and log records are appended to
`$HOME/Library/Application Support/ai.storyteller.photocraft/logs/runner.log`
inside the app container. A missing container or an unwritable log file is
reported and does not prevent launch.

```sh
cargo test -p photocraft-ios --lib
cargo build --target aarch64-apple-ios -p photocraft-ios --lib
cargo build --target aarch64-apple-ios-sim -p photocraft-ios --lib
```

Host calls to the runner return an unsupported-platform error without opening a
window. The static library's Apple entry symbols are absent on other platforms.
