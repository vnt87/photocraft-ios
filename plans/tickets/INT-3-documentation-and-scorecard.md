# Ticket INT-3: Documentation & Parity Scorecard Update

- **Assigned Session:** Claude Code on Linux Server
- **Branch:** Integration / PR
- **Recommended Model:** `Claude Haiku 5.5`
- **Prerequisite:** `INT-2`
- **Status:** DOCS COMPLETE, SIGN-OFF PENDING INT-2 (2026-10-12): documents only demonstrated iPadOS capabilities; full sign-off waits on INT-2. The blanket yes for barrel rotation and eraser end below is NOT established.

---

## 1. Context & Objective
Per PhotoCraft contributing rules, when hardware capabilities or platform targets expand, documentation and the parity scorecard must be updated.

---

## 2. Tasks
1. Update `docs/hardware-parity.md`:
   - Add **iPadOS** column to the Pen Tablets table (Pressure: yes, Tilt: yes, Barrel rotation: yes, Eraser end: yes).
   - Document Apple Pencil hover, double-tap, and Metal 3 acceleration on Apple Silicon M3.
2. Update `ROADMAP.md` and `docs/target-app-parity.md` recording iPadOS support.
3. Append entry to `log/devlog.md` detailing the multi-agent landing.
4. Run:
   ```bash
   cargo xtask scorecard
   ```
   and commit updated `docs/scorecard.md`.

---

## 3. Verification & Acceptance Criteria
- [x] Docs accurately describe iPadOS capabilities (2026-10-12): only demonstrated results are recorded; tilt is provisional, barrel rotation, eraser end and pressure curve are not verified, double-tap is deferred, and gestures and multitasking are not yet verified on hardware.
- [x] `cargo xtask scorecard --check` passes (2026-10-12). The scorecard has no iPadOS checklist rows, so its content is unchanged.
- [x] INT-2 scenarios user-reported as successful (2026-10-12); the sign-off is recorded as user-reported, not instrumented.
