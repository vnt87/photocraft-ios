# Ticket LINUX-4: Layering, WASM Verification & CI Gate

- **Assigned Session:** Claude Code on Linux Dev Server
- **Branch:** `feature/ipados-core-contracts`
- **Prerequisite:** `LINUX-1`, `LINUX-2`, `LINUX-3`
- **Recommended Model:** `Claude Haiku 5.5`
- **Layer:** Workspace / xtask
- **Status:** READY TO PICK UP (Run before opening PR)

---

## 1. Context & Objective
Per `AGENTS.md` golden rules:
- **Rule 7:** "Never break wasm. L0–L6 must `cargo check --target wasm32-unknown-unknown` (run `cargo xtask wasm`)."
- **Layering:** "A crate may depend only on lower layers... Layering is enforced by `cargo xtask layers`."
- **Clippy:** `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]`.

This ticket performs the full verification sweep on Linux to ensure all code from the Linux track is clean, fully compliant, and ready to be merged into upstream Git for the Mac session.

---

## 2. Verification Steps

1. **Verify Layer Boundaries:**
   ```bash
   cargo xtask layers
   ```
   Ensure no new circular or forbidden dependencies exist between L0 and L6.

2. **Verify WASM Compatibility:**
   ```bash
   cargo xtask wasm
   ```
   Ensure `photocraft-ui-egui` and all core crates build without error for WebAssembly.

3. **Verify Clippy & Never-Crash Lints:**
   ```bash
   cargo clippy -p photocraft-ui-egui -p photocraft-testkit --all-targets -- -D warnings
   ```

4. **Verify Target Compilation for iOS:**
   ```bash
   cargo check --target aarch64-apple-ios -p photocraft-ui-egui -p photocraft-engine
   ```

5. **Update Dev Log:**
   Append a concise record to `log/devlog.md` detailing the Linux contracts and gesture engine implementation.

---

## 3. Deliverables
- Clean branch ready for PR: `feature/ipados-core-contracts` -> `origin/main` (or integration branch).
- Green CI check outputs.
