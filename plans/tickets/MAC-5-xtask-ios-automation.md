# Ticket MAC-5: `cargo xtask ios` Automation Pipeline

- **Assigned Session:** Codex on macOS
- **Branch:** `feature/ipados-xcode-runner`
- **Recommended Model:** `GPT-6-Luna`
- **Prerequisite:** `MAC-1`, `MAC-2`
- **Status:** READY TO PICK UP

---

## 1. Context & Objective
To make building, signing, and deploying PhotoCraft to the iPad Air M3 (and iOS Simulator) effortless, we integrate an `ios` sub-command directly into PhotoCraft's `cargo xtask` suite (`xtask/src/ios.rs`).

---

## 2. Technical Specification

### A. Subcommand Flags
`cargo xtask ios`:
- `--sim`: Builds for `aarch64-apple-ios-sim`, packages into `.app`, boots iPad Simulator (`xcrun simctl boot`), and launches app.
- `--device`: Builds for `aarch64-apple-ios`, copies staticlib to Xcode harness, runs `xcodebuild` with automatic signing, and installs onto connected iPad Air M3.
- `--release`: Adds `--release` flag to cargo build.

### B. Implementation (`xtask/src/ios.rs`)
```rust
pub fn run(args: &[String]) -> Result<(), String> {
    // 1. Parse --sim, --device, --release
    // 2. Determine target triple: aarch64-apple-ios-sim vs aarch64-apple-ios
    // 3. Run: cargo build -p photocraft-ios --target <triple>
    // 4. Invoke xcodebuild build
    // 5. If --sim: xcrun simctl install booted <app-path> && xcrun simctl launch booted ai.storyteller.photocraft
    // 6. If --device: devicectl / ios-deploy install
}
```

### C. Registration (`xtask/src/main.rs`)
- Register `ios` command in `match cmd.as_str()`.

---

## 3. Files to Touch
- `[NEW] xtask/src/ios.rs`
- `[MODIFY] xtask/src/main.rs`

---

## 4. Verification & Acceptance Criteria
- [ ] `cargo xtask ios --help` prints usage instructions.
- [ ] `cargo xtask ios --sim` compiles and launches in iPad Simulator.
