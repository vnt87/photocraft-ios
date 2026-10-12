# Ticket INT-3: Documentation & Parity Scorecard Update

- **Assigned Session:** Claude Code on Linux Server
- **Branch:** Integration / PR
- **Recommended Model:** `Claude Haiku 5.5`
- **Prerequisite:** `INT-2`
- **Status:** PARTIAL (2026-10-12): documents only demonstrated iPadOS capabilities; full sign-off waits on INT-2. The blanket yes for barrel rotation and eraser end below is NOT established.

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
- [ ] Docs accurately describe iPadOS capabilities.
- [ ] `cargo xtask scorecard` passes without unexpected regressions.
