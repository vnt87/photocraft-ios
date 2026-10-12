# Mac simulator validation — 2026-10-12

Integration base: `669635ca` (Linux-assist), on `feature/ipados-xcode-runner`.
Toolchain: Xcode 27.0. Runtime: iPad Air 11-inch M3 simulator, iPadOS 26.2.

| Area | Evidence | Remaining |
|---|---|---|
| MAC-5 | Debug `cargo xtask ios --sim` builds, installs, and launches; signed Mach-O validated | Device signing/deployment; Release runs |
| Native clipboard | Exact RGBA PNG round trip through UIPasteboard | Images copied from other apps and additional representations |
| Preferences | Native sandbox write/read probe; atomic failure-preservation tests | Device lifecycle acceptance |
| Open / Save | Native pickers visually inspected; valid encoded PSD staged before export | Provider selection, export success, cancel, reopen and subsequent write-back |
| Recovery | Host multiple-crash/adoption, failure/retry and moved-container tests | Real background/termination/reopen scenarios |
| Browser | UIApplication dispatch opens Safari at the requested URL | Simulator Safari page loading failed independently of dispatch; recheck on device |
| Pencil / gestures | Mapping/bridge host tests; four synthetic gesture integration tests; simulator startup has no Pencil bridge error | Pressure, tilt sign, roll, hover/zoom, double-tap and touch-end/palm behavior on hardware |

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
