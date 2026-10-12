# Handoff: Linux session to Codex on macOS

Written 2026-10-12 by the Claude Code Linux session while the Codex session was paused by a rate
limit. Read this first, then `plans/README.md` and the ticket files. Everything below was written
and checked on Linux; **nothing here has run on a simulator or device**.

## Branches

| Branch | What it is |
|---|---|
| `feature/ipados-core-contracts` | Linux track only (LINUX-1..3 plus lint fixes, the cross-track note), merged with `upstream/main`. |
| `feature/ipados-xcode-runner` | Your branch, untouched by us. Tip was `9f4212b5` when we read it. |
| **`feature/ipados-linux-assist`** | **Start here.** Contains your runner (`9f4212b5`), all the Linux work, and the MAC-4 and MAC-5 work done on Linux. It is a superset of both branches above. |

To resume: `git fetch origin && git checkout feature/ipados-xcode-runner && git merge origin/feature/ipados-linux-assist`
(or fast-forward onto it if you have nothing new). Conflicts were not seen when we merged your
branch into ours.

## What the Linux session landed

| Ticket | Where | State |
|---|---|---|
| LINUX-1 gesture engine | `crates/ui-egui/src/gestures.rs`, wired in `canvas.rs` | Done, 26+ unit tests. Extended for the Pencil (below). |
| LINUX-2 touch harness | `crates/testkit/src/touch_sim.rs`, `crates/ui-egui/tests/touch_gestures.rs` | Done, 4 tests. |
| LINUX-3 bookmark tokens | `crates/engine/src/security_token.rs`, `recent_file_tokens` in prefs, `FileDialogReply::send_with_tokens` | Done but **probably redundant** with your sidecar-bookmark design. See "Decision for you". |
| LINUX-4 sweep | | layers, wasm, clippy, tests green. iOS targets now checked too (below). |
| MAC-4 Pencil (Linux part) | `apps/photocraft-ios/src/pencil/{mapping,bridge,native}.rs`, `stylus.rs`, `gestures.rs`, `canvas.rs` | Written and type-checked. **Unverified on hardware.** |
| MAC-5 `cargo xtask ios` | `xtask/src/ios.rs` | Written, 13 tests. **Never run against real xcodebuild/simctl/devicectl.** |
| Lint fixes | seven `collapsible_match` sites under Rust 1.95 | Done, behaviour preserved. |

## Verified on Linux (and how to re-run)

Use `cargo +1.95` (the default 1.88 is too old). iOS targets are installable on Linux for
type-checking only: `rustup +1.95 target add aarch64-apple-ios aarch64-apple-ios-sim`.

```sh
cargo xtask layers
cargo xtask wasm
cargo clippy --workspace --all-targets -- -D warnings
cargo check --target aarch64-apple-ios -p photocraft-ios
cargo check --target aarch64-apple-ios-sim -p photocraft-ios
cargo clippy --target aarch64-apple-ios -p photocraft-ios -- -D warnings
cargo test -p photocraft-ios -p xtask
cargo test -p photocraft-ui-egui --lib --test touch_gestures
```

## MAC-4 design summary (read before touching Pencil code)

- winit 0.30 / egui-winit 0.36 already deliver every `UITouch`, Pencil included, as
  `WindowEvent::Touch` with force and max force. Fingers and pen share one device id; the touch id is
  the `UITouch` address. egui's emulated pointer follows the first touch that lands.
- Lost on that path: azimuth (tilt), roll, coalesced high-rate samples, touch type, hover.
- `pencil/mapping.rs` (pure, tested): force -> pressure (zero/NaN max force gives 1.0, never 0),
  altitude/azimuth -> tilt per W3C, roll -> rotation, hover samples with pressure 0.
- `pencil/bridge.rs` (pure, tested): state between UIKit and the app; feeds `StylusFeed`, remembers
  pen touch ids, holds eraser state, turns hover into egui pointer moves.
- `pencil/native.rs` (iOS only, `unsafe` with `SAFETY:` comments): a window touch observer that
  never delays or cancels touches, a Pencil-only hover recognizer, a `UIPencilInteraction` delegate.
  `rollAngle` and hover angles are called only after `respondsToSelector` because the deployment
  target is iOS 17.0.
- Double-tap: there are no `tool.eraser` / `tool.previous` commands (tools are UI state). Double-tap
  toggles the `eraser` flag on pen samples; the existing `Stylus::sync_eraser_tool` switches tool.
- Palm rejection: pen touch ids are remembered, `GestureRecognizer::feed_frame_with` keeps them out of
  the finger count, and a palm landing after the pen no longer blocks the stroke.

## Known open issues

1. **A palm that lands before the pen** owns egui's pointer, so painting stays blocked until it lifts.
   Needs egui-winit to hand the pointer to the pen, or the palm touch cancelled natively.
2. `cargo fmt --check` shows three diffs that predate this work: `file_dialog/tests.rs`,
   `gestures.rs` (a test helper), `ui-egui/src/lib.rs`. Not touched.
3. The scorecard counts one more "setting" (156) because of LINUX-3's `recent_file_tokens`.

## Decision for you: bookmark design

Your documents layer copies each picked file into a managed snapshot with a sidecar
`destination.bookmark`. The recent list holds in-sandbox paths and needs no token, so our
`DocumentSecurityToken` / `recent_file_tokens` are unused by `apps/photocraft-ios`
(`git grep DocumentSecurityToken -- apps/` is empty). Our recommendation, in
`plans/cross-track-notes.md` item 1: keep your design and delete ours (back to 155 settings).
Tell the Linux session, or delete it in a small commit (engine `security_token.rs`, the prefs field,
`send_with_tokens`, then `cargo xtask scorecard`).

## What is left, in priority order

1. **Verify MAC-4 on a device** (checklist in `plans/tickets/MAC-4-apple-pencil-pipeline.md`):
   pressure, tilt sign (flip by adding pi in `tilt_degrees` if inverted), Pencil Pro roll, observer
   does not block finger gestures or delay touch ends, hover on an M2+ iPad (including zoom != 1),
   double-tap settings (watch for double toggling on iOS 17.5+), first-gen Pencil, simulator.
2. **Run and fix MAC-5** (`cargo xtask ios --sim`, `--device`, `--release`). Uncertain items are
   marked in `xtask/src/ios.rs` and listed in the MAC-5 ticket: default simulator name,
   `-allowProvisioningUpdates`, `DEVELOPMENT_TEAM=`, `devicectl` flag spellings.
3. **Finish MAC-3**: persistent saves, recovery/autosave to the sandbox, `open_url`, prefs.
   Documents are still session-only. Pure logic can be tested on Linux; UIKit calls cannot.
4. **Test the gestures on the simulator** (pinch, pan, rotate; zoom drift, rotation sign on a
   flipped view, safe-area interaction).
5. **INT-1** is mostly done by this branch (merge and host/iOS checks). Remaining: confirm device and
   simulator builds warning-free on the Mac.
6. **INT-2** physical iPad Air M3 sign-off (Mac only, needs the device).
7. **INT-3** docs and scorecard: Linux can do it after INT-2. Do not document Pencil capabilities
   (tilt, hover, double-tap) in `docs/hardware-parity.md` before INT-2 confirms them.
8. Section 5 of `AGENTS.md` items not done: `docs/` parity updates, `cargo xtask parity`.

## Practical notes

- Disk is tight on the Linux box; each cargo target dir is 10-20 GB.
- On Linux the Mac-only crates compile only because `cargo check` does not link; nothing here has
  been linked or run for iOS.
