# Ticket LINUX-2: Synthetic Multi-Touch Test Harness

- **Assigned Session:** Claude Code on Linux Dev Server
- **Branch:** `feature/ipados-core-contracts`
- **Prerequisite:** `LINUX-1`
- **Recommended Model:** `Claude Sonnet 5.5`
- **Layer:** `testkit` and L6 (`photocraft-ui-egui`)
- **Status:** READY TO PICK UP

---

## 1. Context & Objective
Because Linux cannot run the iOS Simulator, we need automated, headless simulation tests that prove:
1. Two fingers moving apart cleanly zooms the canvas.
2. Two fingers rotating rotates the canvas.
3. Two fingers dragging pans the canvas.
4. An active stylus touch (Apple Pencil) draws while an accidental palm resting on the canvas is ignored.

This ticket introduces synthetic multi-touch simulation helpers into `photocraft-testkit` and adds integration tests to `photocraft-ui-egui`.

---

## 2. Technical Specification

### A. Touch Simulator (`crates/testkit/src/touch_sim.rs`)
```rust
use egui::{Pos2, Event, TouchId, TouchDeviceId, TouchPhase};

pub struct TouchStreamBuilder {
    device_id: TouchDeviceId,
}

impl TouchStreamBuilder {
    pub fn new() -> Self {
        Self { device_id: TouchDeviceId(42) }
    }

    pub fn pinch(&self, center: Pos2, r_start: f32, r_end: f32, steps: usize) -> Vec<Vec<Event>> {
        // Generates sequence of frames with 2 touches expanding from r_start to r_end
    }

    pub fn rotate(&self, center: Pos2, radius: f32, angle_start_rad: f32, angle_end_rad: f32, steps: usize) -> Vec<Vec<Event>> {
        // Generates sequence of frames with 2 touches rotating around center
    }

    pub fn pan(&self, start: Pos2, end: Pos2, finger_count: usize, steps: usize) -> Vec<Vec<Event>> {
        // Generates multi-finger drag sequence
    }
}
```

### B. Integration Tests (`crates/ui-egui/tests/touch_gestures.rs`)
Write integration tests:
- `test_pinch_to_zoom_scales_viewport()`: Verifies canvas zoom level increases by expected ratio.
- `test_two_finger_rotate_canvas()`: Verifies canvas view angle updates without distorting layer raster data.
- `test_palm_rejection_with_stylus()`: Simulates a stylus stroke while simultaneously injecting a finger touch event; verifies that only the stylus brush stroke is committed to the layer bitmap.

---

## 3. Files to Touch
- `[NEW] crates/testkit/src/touch_sim.rs`
- `[MODIFY] crates/testkit/src/lib.rs` (expose module)
- `[NEW] crates/ui-egui/tests/touch_gestures.rs`

---

## 4. Verification & Acceptance Criteria
- [ ] Tests run headlessly and deterministically.
- [ ] No flakiness or timing dependencies.
- [ ] Run test suite:
  ```bash
  cargo test -p photocraft-ui-egui --test touch_gestures
  ```
