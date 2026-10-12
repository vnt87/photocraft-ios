# Ticket MAC-4: Apple Pencil Hardware Pipeline & Hover Hook

- **Assigned Session:** Codex on macOS
- **Branch:** `feature/ipados-xcode-runner`
- **Recommended Model:** `GPT-6.1-Sol`
- **Prerequisite:** `MAC-2`
- **Status:** READY TO PICK UP

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
