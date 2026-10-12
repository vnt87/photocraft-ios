# Ticket INT-1: Worktree Merge & End-to-End Build

- **Assigned Session:** Codex on macOS
- **Branch:** Integration / PR
- **Recommended Model:** `GPT-6.1-Sol`
- **Prerequisite:** `LINUX-4`, `MAC-5`
- **Status:** GATES PASS (2026-10-12): `cargo xtask layers` (30 crates), `cargo xtask wasm`, `cargo clippy -p photocraft-ios -p photocraft-ui-egui -p xtask --all-targets -- -D warnings`, `cargo test -p photocraft-ios --lib` (47 passed). Not run: `panic_hunt` (no engine commands changed). Remaining: review and commit the uncommitted UI/documentation edits, then PR.

---

## 1. Context & Objective
Once the Linux track (`feature/ipados-core-contracts`) and Mac track (`feature/ipados-xcode-runner`) have completed their respective deliverables:
1. Merge `feature/ipados-core-contracts` into the integration branch.
2. Verify that the pure Rust multi-touch gesture engine from Linux integrates seamlessly with the UIKit runner and Apple Pencil bridge from Mac.
3. Verify that all workspace checks (`layers`, `wasm`, `clippy`) pass cleanly.

---

## 2. Integration Steps
1. Fetch and merge `feature/ipados-core-contracts`.
2. Connect `photocraft-ui-egui::gestures` with the touch stream from `apps/photocraft-ios`.
3. Run full verification:
   ```bash
   cargo xtask layers
   cargo xtask wasm
   cargo clippy -p photocraft-ios -p photocraft-ui-egui --all-targets -- -D warnings
   cargo check --target aarch64-apple-ios -p photocraft-ios
   ```

---

## 3. Verification & Acceptance Criteria
- [ ] Merge commit is clean with zero git conflicts.
- [ ] Zero clippy warnings across touched crates.
- [ ] Both device and simulator builds compile without warnings.
