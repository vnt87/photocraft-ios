# Ticket LINUX-1: Pure Rust Touch Gesture Engine & Math

- **Assigned Session:** Claude Code on Linux Dev Server
- **Branch:** `feature/ipados-core-contracts`
- **Recommended Model:** `Claude Opus 5.5` (Architecture) & `Claude Sonnet 5.5` (Implementation)
- **Layer:** L6 (`photocraft-ui-egui`)
- **Status:** READY TO PICK UP

---

## 1. Context & Objective
iPad users expect natural multi-touch gestures on the canvas:
- 2-finger pinch to zoom in/out with focus on the pinch midpoint.
- 2-finger drag to pan across the document canvas.
- 2-finger rotation to rotate the canvas view (PhotoCraft's `Rotate View` tool).
- Complete palm rejection: single-finger contact must NOT draw or create brush marks when an Apple Pencil is touching or hovering.

Because the Linux dev server does not have Xcode or UIKit, this ticket builds the pure Rust gesture engine that consumes platform-agnostic `egui::Event::Touch` / multi-touch streams and converts them into canvas transformations.

---

## 2. Technical Specification

### A. Gesture Recognizer State Machine (`crates/ui-egui/src/gestures.rs`)
Implement `TouchGestureState` and `GestureRecognizer`:

```rust
use egui::{Pos2, Vec2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchPoint {
    pub id: u64,
    pub pos: Pos2,
    pub phase: TouchPhase,
}

#[derive(Default, Debug)]
pub struct GestureRecognizer {
    active_touches: Vec<TouchPoint>,
    initial_distance: Option<f32>,
    initial_angle: Option<f32>,
    last_centroid: Option<Pos2>,
}

pub struct GestureDelta {
    pub pan_delta: Vec2,
    pub zoom_factor: f32,
    pub rotation_delta_rad: f32,
    pub pivot: Pos2,
}

impl GestureRecognizer {
    pub fn update(&mut self, touch: TouchPoint, stylus_active: bool) -> Option<GestureDelta> {
        // 1. If stylus_active is true, ignore single touch (palm rejection).
        // 2. Track active touches (maintain 2 touch points max for canvas transform).
        // 3. Compute centroid: (p1 + p2) / 2.0.
        // 4. Compute distance: ||p1 - p2||.
        // 5. Compute angle: atan2(p2.y - p1.y, p2.x - p1.x).
        // 6. Return relative deltas against previous frame.
    }
}
```

### B. Canvas Integration (`crates/ui-egui/src/canvas.rs`)
- Wire `GestureRecognizer` into the canvas input loop.
- When `GestureDelta` is produced:
  - Apply `pan_delta` to document pan offset.
  - Multiply document zoom by `zoom_factor` centered around `pivot`.
  - Apply `rotation_delta_rad` to view rotation angle.
  - Request repaint via `ctx.request_repaint()`.

---

## 3. Files to Touch
- `[NEW] crates/ui-egui/src/gestures.rs`
- `[MODIFY] crates/ui-egui/src/lib.rs` (register `pub mod gestures;`)
- `[MODIFY] crates/ui-egui/src/canvas.rs` (integrate gesture recognizer into canvas event handler)

---

## 4. Verification & Acceptance Criteria
- [ ] No `unwrap()` or `expect()` in non-test code.
- [ ] Palm rejection: When `StylusFeed` has `pressure > 0.0` or hover state, single-touch events emit no painting actions.
- [ ] Math accuracy: Zoom in/out preserves focal point under fingers; rotation handles angle wrap-around seamlessly.
- [ ] Automated tests:
  ```bash
  cargo test -p photocraft-ui-egui --lib gestures
  ```
