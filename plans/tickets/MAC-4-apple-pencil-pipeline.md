# Ticket MAC-4: Apple Pencil Hardware Pipeline & Hover Hook

- **Assigned Session:** Codex on macOS
- **Branch:** `feature/ipados-xcode-runner`
- **Recommended Model:** `GPT-6.1-Sol`
- **Prerequisite:** `MAC-2`
- **Status:** IMPLEMENTED / DEVICE-PARTIALLY-VERIFIED (2026-10-12): pressure and hover confirmed on iPad Air M3; tilt provisional; double-tap deferred; barrel rotation and eraser end unverified.

---

## 1. Context & Objective
On an iPad Air M3, the Apple Pencil provides:
- High-precision touch coordinates with sub-pixel resolution.
- Variable tip force (`touch.force / touch.maximumPossibleForce`).
- Tilt angles (`altitudeAngle` and `azimuthAngle`).
- Barrel roll (`rollAngle` on Apple Pencil Pro).
- Hover sensing (M2/M3 iPads detect the Pencil tip up to 12 mm above the display).

PhotoCraft's brush engine (`crates/ui-egui/src/stylus.rs`) already accepts `PenSample` via `StylusFeed`. This ticket bridges UIKit's Apple Pencil touch stream directly into `StylusFeed`.

---

## 2. Technical Specification

### A. Touch Stream Interception (`apps/photocraft-ios/src/pencil.rs`)
Implement `PencilMonitor`:

```rust
use photocraft_ui_egui::stylus::{PenSample, StylusFeed};

pub fn extract_pencil_sample(touch: &objc2_ui_kit::UITouch) -> Option<PenSample> {
    // 1. Check touch.type == UITouchType::Pencil
    // 2. Read force: (touch.force / touch.maximumPossibleForce).clamp(0.0, 1.0)
    // 3. Convert altitudeAngle (radians to degrees, -90..90) -> tilt_y
    // 4. Convert azimuthAngle -> tilt_x
    // 5. Read rollAngle (if available) -> rotation
    // 6. Return PenSample { pressure, tilt_x, tilt_y, rotation, eraser: false }
}
```

### B. Apple Pencil Hover Hook
- Attach a `UIHoverGestureRecognizer` to the main canvas view.
- When hovering:
  - Extract hover coordinates.
  - Pipe `PenSample { pressure: 0.0, tilt_x, tilt_y, rotation: 0.0, eraser: false }` into `StylusFeed`.
  - PhotoCraft automatically displays the circular brush outline cursor under the floating tip.

### C. Double-Tap / Squeeze Tool Switching
- Attach `UIPencilInteractionDelegate`:
  - When double-tap is detected: toggle between current tool and Eraser (PhotoCraft command `tool.eraser` / `tool.previous`).

---

## 3. Files to Touch
- `[NEW] apps/photocraft-ios/src/pencil.rs`
- `[MODIFY] apps/photocraft-ios/src/lib.rs` (invoke pencil monitor installation)

---

## 4. Verification & Acceptance Criteria
- [ ] No `unwrap()` or `expect()`.
- [ ] Compiles cleanly for `aarch64-apple-ios`.
- [ ] Pressure and tilt calculations match the W3C / Photoshop Pointer Events convention documented in `crates/ui-egui/src/stylus.rs`.

---

## Linux-assist (2026-10-12)

Written and type-checked on Linux (`cargo +1.95 check`/`clippy --target aarch64-apple-ios[-sim]`);
nothing UIKit-side has run yet. The status line above is unchanged: the Mac owns verification.

**Finding.** winit 0.30.13 already forwards every `UITouch`, Pencil included, as
`WindowEvent::Touch` with `Force::Calibrated` (egui gets `Event::Touch { force: force/max }` plus
an emulated pointer that follows the *first* touch). It drops azimuth, roll, coalesced samples
and the touch type, delivers no hover, and uses one device id for fingers and pen. Strokes keep
using winit's path; the bridge adds only what is missing.

**Done.**
- `apps/photocraft-ios/src/pencil/mapping.rs` (host-tested): force/max -> pressure (zero/NaN max
  -> 1.0, never 0), W3C altitude/azimuth -> tilt X/Y (flat-pen special cases, azimuth wrap),
  roll -> rotation 0..360, hover sample (pressure 0).
- `pencil/bridge.rs` (host-tested): pen samples + eraser latch into `StylusFeed`, pen touch ids,
  hover -> egui `PointerMoved`/`PointerGone` through an egui `Plugin::input_hook`, double-tap.
- `pencil/native.rs` (iOS only, unsafe with SAFETY comments): `TouchObserver` gesture-recognizer
  subclass on the key window (coalesced Pencil samples, marks pen touch ids, never delays or
  cancels touches), `UIHoverGestureRecognizer` (Pencil only), `UIPencilInteraction` delegate.
  Installed from `lib.rs` in the eframe creation callback.
- Double-tap: there are no `tool.eraser` / `tool.previous` commands (tools are UI state, not engine
  commands). It toggles the samples' `eraser` flag, which the existing pen-flip code
  (`Stylus::sync_eraser_tool`) turns into Eraser / back to the previous tool. Settings "Switch to
  previous" is handled the same way.
- Palm rejection (cross-track note 2): `StylusFeed` remembers pen touch ids;
  `GestureRecognizer::feed_frame_with` keeps them out of the finger set and mirrors egui-winit's
  pointer owner, so a palm no longer blocks a pen that owns the pointer, and a pen alone is no
  longer a "lone finger". `Stylus::update` skips the pen's duplicate winit force sample.

**Mac to verify.**
1. Pencil strokes paint with pressure. Lean the cap to the right: tilt X positive; towards you:
   tilt Y positive (if inverted, UIKit's azimuth points at the tip: add pi in `mapping::tilt_degrees`).
2. Apple Pencil Pro roll changes rotation; check its zero and direction against W3C `twist`.
3. The observer sees each touch before winit (pen id marked in time: a pen-only stroke is not
   palm-blocked) and does not delay touch ends (no stuck strokes); finger pinch/pan still work.
4. Hover (M2+ iPad): the brush outline follows the floating tip and leaves with it; trackpad
   hover is not taken as Pencil; position right at egui zoom != 1.
5. Double-tap toggles Brush <-> Eraser with Settings = Switch Eraser; Ignore does nothing; on
   iOS 17.5+ one tap toggles once, not twice.
6. A palm that lands before the pen keeps painting blocked until it lifts (egui-winit gives the
   pointer to the palm). Fixing that needs the pointer re-assigned to the pen (egui-winit change
   or cancelling palm touches natively): follow-up.
7. No crash or log spam with a 1st-gen Pencil (no hover, no roll) or in the simulator.
