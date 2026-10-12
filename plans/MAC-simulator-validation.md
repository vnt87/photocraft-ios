# Mac simulator validation — 2026-10-12

Integration base: `669635ca` (Linux-assist), on `feature/ipados-xcode-runner`.
Toolchain: Xcode 27.0. Runtime: iPad Air 11-inch M3 simulator, iPadOS 26.2.

| Area | Evidence | Remaining |
|---|---|---|
| MAC-5 | Debug `cargo xtask ios --sim` builds, installs, and launches; signed Mach-O validated | Release runs |
| Native clipboard | Exact RGBA PNG round trip through UIPasteboard | Images copied from other apps and additional representations |
| Preferences | Native sandbox write/read probe; atomic failure-preservation tests | Device lifecycle acceptance |
| Open / Save | Native pickers visually inspected; valid encoded PSD staged before export | Provider selection, export success, cancel, reopen and subsequent write-back |
| Recovery | Host multiple-crash/adoption, failure/retry and moved-container tests | Real background/termination/reopen scenarios |
| Browser | UIApplication dispatch opens Safari at the requested URL | Simulator Safari page loading failed independently of dispatch; recheck on device |
| Pencil / gestures | Mapping/bridge host tests; four synthetic gesture integration tests; simulator startup has no Pencil bridge error | Tilt sign, roll, hover/zoom, double-tap and touch-end/palm behavior on hardware |

The managed-snapshot and sidecar-bookmark design is retained. The unused engine/UI
token abstraction and preference map are deleted; old JSON still loads. Settings
count returns from 156 to 155. Recent paths and recovery references are rebased only
from supported iOS container prefixes to existing, contained managed snapshots.
External/archive paths, malformed paths, and escaping symlinks are not rewritten.

Final gates pass: full engine and UI package tests, 47 runner tests (12 storage),
62 tooling tests, four touch-harness integration tests, five packaging tests,
host and iOS-device/simulator Clippy with warnings denied, formatting, dependency
layers, the full WASM check, and scorecard freshness. The UI library contributes
2,234 passing tests with 11 ignored. The final signed Debug app was installed and
remained running after launch.

The CLI viewer is optional: `--sim` does not require the desktop `Simulator.app`;
use `--show-simulator` when a full Xcode install provides it. This is an explicit
option, not a swallowed viewer-launch error.

## Physical acceptance next

Connect and unlock the iPad Air M3 with its paired Pencil available. Confirm trust,
Developer Mode, signing team, and OS version as needed, then install the Debug app.
Test local Files and iCloud open/save/cancel, reopen and write-back after restart,
clipboard from another app, recovery after background/termination, two-finger
gestures, and Pencil pressure/tilt/hover/roll/double-tap. The palm-before-pen case
remains a known open issue. Do not raise hardware-parity claims until these pass.

## Physical iPad acceptance — 2026-10-12

Debug installation and launch passed on iPad Air 13-inch (M3), iPadOS 27.0
(build 24A437). The user confirmed the editor renders and Apple Pencil brush
strokes vary in width with pressure. Device logs confirm Metal initialization
and Pencil bridge installation; the process remained running after launch.

The runner now registers a single UIWindowScene delegate and attaches the
winit window to its connected scene. Scene-enabled simulator launch/rendering
also passed. Multiple windows, lifecycle reconnect, safe-area resizing, and the
remaining Pencil capabilities are not yet accepted. Six packaging tests pass.

Signing required a unique development bundle identifier because the product
identifier was unavailable to the selected team. Building for the actual device
with provisioning-device registration produced an installable profile; the user
then trusted it on the iPad. Repository product identifiers remain unchanged.

Xcode 27 provides Device Hub; absence of the older Simulator.app does not imply
a CLI-only Xcode installation. Default CLI deployment works without a viewer.

User acceptance additionally passed: new document → save → reopen → edit →
save again, with edits persisted. File provider, format, and behavior after an
app restart were not specified, so those remain separate checks. The user
observed Pencil hover working over menus; canvas hover remains unverified.

Pencil follow-up: tilt appears functional but subtle; palm-before-pen produced
no stray mark in the user's test. These are provisional hardware passes.
Double-tap is deferred at the user's request. Canvas hover has no visible brush
cursor: tracing found that tool_cursor selects the native desktop cursor path
on iOS, while winit's iOS cursor setter is ignored. A painter-fallback repair
and regression coverage are in progress; hardware retest remains required.

Canvas hover repair verified on hardware: iOS now uses the existing painted
brush/crosshair/outline fallback because winit cannot display OS cursor bitmaps
on iOS. After the updated signed Debug build was installed, the user confirmed
the brush outline follows the hovering Pencil tip. Focused cursor tests passed
(14 passed, one visual-preview test ignored), iOS Clippy and layering pass.
Tilt and palm rejection remain provisional passes; double-tap remains deferred.

- Full `cargo xtask wasm` compatibility check passed after the hover repair.
