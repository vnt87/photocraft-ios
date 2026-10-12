# Ticket INT-2: Physical iPad Air M3 Testing & Sign-off

- **Assigned Session:** Codex on macOS (with physical iPad Air M3 attached)
- **Branch:** Integration / PR
- **Recommended Model:** `GPT-6-Luna`
- **Prerequisite:** `INT-1`
- **Status:** BLOCKED (Pending INT-1 merge)

---

## 1. Context & Objective
Verify PhotoCraft running live on the physical **iPad Air M3** hardware.

---

## 2. Test Plan on Physical Hardware

1. **Launch & Metal Verification:**
   - Run `cargo xtask ios --device`.
   - Verify PhotoCraft launches directly to the full-canvas desktop Photoshop layout on the Liquid Retina display.
   - Inspect console logs: Verify `wgpu` selected the Apple M3 Metal device (`rs.adapter.get_info()`).

2. **Apple Pencil Validation:**
   - **Pressure:** Draw light to heavy strokes; stroke thickness and opacity scale smoothly.
   - **Tilt:** Tilt the pencil; shading angle and calligraphic stroke width adapt.
   - **Hover:** Hover pencil tip 5–10 mm above screen; circular brush outline tracks tip without touching.
   - **Double-Tap:** Double-tap flat edge of Apple Pencil; tools toggle between Brush and Eraser.
   - **Palm Rejection:** Rest wrist firmly on canvas while drawing; zero stray marks.

3. **Multi-Touch Gestures:**
   - Pinch with 2 fingers to zoom canvas in and out.
   - Drag with 2 fingers to pan canvas.
   - Rotate with 2 fingers to rotate view angle smoothly.

4. **File I/O & Files App Integration:**
   - `File → Open`: `UIDocumentPicker` opens. Pick `.psd` from iCloud Drive or "On My iPad"; opens with layers.
   - `File → Save As`: Saves `.pcraft` or `.psd` back to chosen directory.

5. **iPadOS Multitasking:**
   - Resize window in Stage Manager; verify viewport reflows without crashing or resetting zoom.
   - Test Split View at 1/2 and 2/3 width.

---

## 3. Acceptance Sign-off
- [ ] All 5 physical device test scenarios pass without crash, stutter, or corruption.
