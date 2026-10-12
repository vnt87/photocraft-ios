# Ticket MAC-5: `cargo xtask ios` Automation Pipeline

- **Assigned Session:** Codex on macOS
- **Branch:** `feature/ipados-xcode-runner`
- **Recommended Model:** `GPT-6-Luna`
- **Prerequisite:** `MAC-1`, `MAC-2`
- **Status:** DEBUG SIMULATOR VERIFIED (2026-10-12); device signing/deployment and Release verification pending

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
- [x] `cargo xtask ios --help` prints usage instructions.
- [x] `cargo xtask ios --sim` compiles and launches in iPad Simulator.

### Mac verification (2026-10-12)

The integrated Debug pipeline builds a signed universal simulator bundle, boots the
named iPad Air M3, installs, and launches successfully. The desktop viewer is now
explicitly optional (`--show-simulator`), with regression coverage: this Mac has
working CLI simulator tools but no `Simulator.app`. Device install/launch flag
spellings match the installed `devicectl` help; actual device signing/deployment
still require the connected iPad and a configured development team. Release
execution remains unverified.

---

## Linux-assist (2026-10-12)

Done on Linux (commit on `feature/ipados-linux-assist` lineage): `xtask/src/ios.rs` plus registration
and help in `xtask/src/main.rs`. Flags: `--sim`, `--device`, `--release`, `--help`, plus `--dry-run`,
`--sim-name`, `--device-id` / `$PHOTOCRAFT_IOS_DEVICE`, `--team` / `$PHOTOCRAFT_IOS_TEAM`. Argument parsing and the
command `Plan` (xcodebuild, simctl, devicectl) are pure and unit-tested (13 tests); non-macOS hosts
refuse `--sim`/`--device`, while `--help` and `--dry-run` work anywhere. The Xcode build phases already run
`build-rust.sh`/`link-rust.sh`, so the plan does not call cargo separately (deviation from the sketch in section 2.B).
Without a device id, `--device` builds and lists devices (`devicectl list devices`) instead of installing.

Only the Mac can verify (acceptance box 2 is still open):
- A real `cargo xtask ios --sim` run: xcodebuild succeeds with `-derivedDataPath target/ios-derived`, the
  `.app` lands at `Build/Products/<Config>-iphonesimulator/PhotoCraft.app`, and the default simulator name
  `iPad Air 11-inch (M3)` exists (else pass `--sim-name`).
- `simctl boot` / `bootstatus -b` / `install` / `launch` behaviour with a simulator name (vs UDID).
- `--device`: automatic signing, whether `-allowProvisioningUpdates` and `DEVELOPMENT_TEAM=` are right,
  and the `xcrun devicectl device install app` / `device process launch` flag spellings on the installed Xcode.
- `--release` end to end (dSYM, Release signing).
