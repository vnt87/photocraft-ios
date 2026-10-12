# PhotoCraft iPadOS runner

This crate provides the Rust static library linked by the Xcode harness in
`packaging/ios`. The Apple-only entry bridge exports `main` and
`photocraft_ios_main`; the harness must force-load the entry symbol with
`-Wl,-u,_main`. UIKit startup and the Metal surface lifecycle are owned by
winit/eframe 0.36.2. The renderer allows only Metal.

The root viewport follows the operating system's size instead of assuming an
11-inch or 13-inch screen. No simulator or device size is hard-coded.

The runner registers a single UIKit window scene and attaches the winit window
to it, supporting the scene lifecycle required by iPadOS 27. Native picker and
Pencil lookups use that scene's window. The status bar remains hidden through
winit's safe API. Multiwindow ownership and lifecycle/resizing acceptance remain
follow-up work.

Native services provide asynchronous document pickers, encoded export staging,
security-scoped write-back bookmarks, PNG image clipboard access, browser dispatch,
atomic preferences, and background crash recovery. Recovery lives in Application
Support rather than purgeable Caches. Files are imported into managed snapshots;
recent files reopen those snapshots instead of refreshing external edits. Managed
recents and recovery references are rebased when a supported iOS app container moves.

On an iPad Air 13-inch M3 running iPadOS 27 the user verified pressure-dependent
brush width and the Pencil hover cursor; tilt and palm-before-pen are provisional
passes, and double-tap is deferred. Barrel roll and eraser behaviour are unverified.
Native clipboard/preferences round trips and picker presentation are verified on the
iPad Air M3 simulator with iPadOS 26.2. Provider selection, actual external saving,
native cancellation, touch gestures and multitasking remain acceptance checks.

On iOS, startup diagnostics and log records are appended to
`$HOME/Library/Application Support/ai.storyteller.photocraft/logs/runner.log`
inside the app container. A missing container or an unwritable log file is
reported and does not prevent launch.

```sh
cargo test -p photocraft-ios --lib
cargo build --target aarch64-apple-ios -p photocraft-ios --lib
cargo build --target aarch64-apple-ios-sim -p photocraft-ios --lib
cargo xtask ios --sim
# Physical iPad (development bundle id when the product id cannot be registered):
cargo xtask ios --device --device-id <udid> --team <team id> --bundle-id <dev id>
# Full Xcode installs can also open the desktop viewer:
cargo xtask ios --sim --show-simulator
```

Debug builds expose reproducible service probes through simctl:

```sh
SIMCTL_CHILD_PHOTOCRAFT_IOS_SERVICE_SMOKE=clipboard xcrun simctl launch \
  --terminate-running-process 'iPad Air 11-inch (M3)' ai.storyteller.photocraft
```

Modes `open` and `save` present the native pickers; `save` uses a synthetic 16×16
document. Mode `url` requests Safari at `https://example.com`. Probe results go to
`runner.log`; browser page loading is separate from successful URL dispatch.
These probes are excluded from release builds.

Host calls to the runner return an unsupported-platform error without opening a
window. The static library's Apple entry symbols are absent on other platforms.
