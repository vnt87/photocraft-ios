# PhotoCraft iPadOS Port: Dual-Session Master Execution Plan

> **Target Device:** iPad Air M3 (Apple Silicon ARM64, 8-core CPU, 10-core GPU, 8 GB Unified Memory, Apple Pencil)  
> **Mac Worktree:** Codex on macOS (Xcode 27, iOS SDKs, Simulator, iPad Air M3 device) — Branch: `feature/ipados-xcode-runner`  
> **Linux Worktree:** Claude Code on Linux Server (Pure Rust toolchain, synthetic harness, CI) — Branch: `feature/ipados-core-contracts`  
> **Available Models:** GPT-6.1-Sol, GPT-6-Luna, Claude Opus 5.5, Claude Sonnet 5.5, Claude Haiku 5.5

---

## 1. Multi-Session Strategy & Boundaries

To enable Codex (Mac) and Claude Code (Linux) to work in parallel without lock collisions or Git conflicts:
- **Zero Cross-Platform Blocker:** The Linux dev server does not attempt to link iOS SDKs or run Xcode. It implements pure Rust math, state machines, and headless test harnesses.
- **Dedicated Runner Isolation:** The Mac worktree owns `apps/photocraft-ios`, `packaging/ios`, and `xtask/src/ios.rs`.
- **Git Synchronization:** Work is developed on separate feature branches and synchronized via pull requests to a shared integration branch.

```mermaid
graph TD
    subgraph Linux_Track [Linux Dev Server: Claude Code (feature/ipados-core-contracts)]
        L1[LINUX-1: Touch Gesture Engine] --> L2[LINUX-2: Multi-Touch Test Harness]
        L1 --> L3[LINUX-3: Sandboxed Document Contracts]
        L2 --> L4[LINUX-4: Layering & WASM CI Gate]
        L3 --> L4
    end

    subgraph Mac_Track [Mac Worktree: Codex (feature/ipados-xcode-runner)]
        M1[MAC-1: Xcode Project Harness] --> M2[MAC-2: apps/photocraft-ios Runner]
        M2 --> M3[MAC-3: UIKit FFI Services]
        M2 --> M4[MAC-4: Apple Pencil Hardware Pipeline]
        M3 --> M5[MAC-5: cargo xtask ios Automation]
        M4 --> M5
    end

    L4 --> INT1[INT-1: Worktree Merge & Rebase]
    M5 --> INT1
    INT1 --> INT2[INT-2: Physical iPad Air M3 Verification]
    INT2 --> INT3[INT-3: Documentation & Parity Scorecard]
```

---

## 2. Model Allocation Guide

Use this matrix when prompting agents in each session:

| Model | Session | Strengths & Role in this Plan |
|---|---|---|
| **GPT-6.1-Sol** | Mac (Codex) | **Primary for Complex FFI & Hardware.** Best for Objective-C runtime interop (`objc2-ui-kit`, `block2`), `CAMetalLayer` binding, Xcode project definitions, and Apple Pencil kinematics. |
| **GPT-6-Luna** | Mac (Codex) | **Fast Automation & Tooling.** Best for `xtask/src/ios.rs` automation, shell scripts, and device deployment loops. |
| **Claude Opus 5.5** | Linux (Claude Code) | **Foundational Architecture & State Machines.** Best for gesture state machine theory, palm rejection protocols, and multi-stage lifecycle design. |
| **Claude Sonnet 5.5** | Linux (Claude Code) | **High-Velocity Rust Implementation.** Best for gesture coordinate math, synthetic test engines, unit tests, and trait implementations. |
| **Claude Haiku 5.5** | Linux / Mac | **Lightweight Verification & Maintenance.** Best for running `cargo xtask layers`, `cargo xtask wasm`, updating `Cargo.toml`, and updating documentation. |

---

## 3. Ticket Directory & Status

**2026-10-12:** MAC-1 and the MAC-2 bootstrap are implemented and verified on the
iPad Air M3 simulator (iPadOS 26.2, built with Xcode 27). Device and simulator Rust
checks pass. MAC-3 platform services, MAC-4 Pencil input, MAC-5 automation, the Linux
track, and physical-device/iPadOS 27 runtime validation remain open. Documents are
session-only in the bootstrap.

### Phase 1: Linux Track (Claude Code on Linux Server)
- [LINUX-1: Pure Rust Touch Gesture Engine & Math](file:///Users/naam.vu/Downloads/Github/photocraft-ios/plans/tickets/LINUX-1-gesture-engine.md) — *Model: Opus 5.5 (Arch) + Sonnet 5.5 (Code)*
- [LINUX-2: Synthetic Multi-Touch Test Harness](file:///Users/naam.vu/Downloads/Github/photocraft-ios/plans/tickets/LINUX-2-touch-test-harness.md) — *Model: Sonnet 5.5*
- [LINUX-3: Sandboxed Document State & Persistence Contracts](file:///Users/naam.vu/Downloads/Github/photocraft-ios/plans/tickets/LINUX-3-sandboxed-document-contracts.md) — *Model: Sonnet 5.5*
- [LINUX-4: Layering, WASM Verification & CI Gate](file:///Users/naam.vu/Downloads/Github/photocraft-ios/plans/tickets/LINUX-4-layering-and-wasm-ci.md) — *Model: Haiku 5.5*

### Phase 2: Mac Track (Codex on macOS with Xcode 27)
- [MAC-1: Xcode Project Harness & Packaging Structure](file:///Users/naam.vu/Downloads/Github/photocraft-ios/plans/tickets/MAC-1-xcode-project-harness.md) — *Model: GPT-6.1-Sol*
- [MAC-2: Dedicated App Runner & Metal Lifecycle](file:///Users/naam.vu/Downloads/Github/photocraft-ios/plans/tickets/MAC-2-app-runner-metal.md) — *Model: GPT-6.1-Sol*
- [MAC-3: iPadOS Platform Services Bridge (UIKit FFI)](file:///Users/naam.vu/Downloads/Github/photocraft-ios/plans/tickets/MAC-3-uikit-ffi-services.md) — *Model: GPT-6.1-Sol*
- [MAC-4: Apple Pencil Hardware Pipeline & Hover Hook](file:///Users/naam.vu/Downloads/Github/photocraft-ios/plans/tickets/MAC-4-apple-pencil-pipeline.md) — *Model: GPT-6.1-Sol*
- [MAC-5: cargo xtask ios Automation Pipeline](file:///Users/naam.vu/Downloads/Github/photocraft-ios/plans/tickets/MAC-5-xtask-ios-automation.md) — *Model: GPT-6-Luna*

### Phase 3: Integration & Validation (Both Sessions)
- [INT-1: Worktree Merge & End-to-End Build](file:///Users/naam.vu/Downloads/Github/photocraft-ios/plans/tickets/INT-1-integration-and-merge.md) — *Model: GPT-6.1-Sol*
- [INT-2: Physical iPad Air M3 Testing & Sign-off](file:///Users/naam.vu/Downloads/Github/photocraft-ios/plans/tickets/INT-2-device-validation-ipad-air-m3.md) — *Model: GPT-6-Luna*
- [INT-3: Documentation & Parity Scorecard Update](file:///Users/naam.vu/Downloads/Github/photocraft-ios/plans/tickets/INT-3-documentation-and-scorecard.md) — *Model: Haiku 5.5*

---

## 4. Golden Rules for Both Agents

1. **Never Crash:** Non-test code never panics (`unwrap()`, `expect()`, `panic!` forbidden). Always propagate errors with `Result`.
2. **Never Break WASM:** Every change must compile under `cargo check --target wasm32-unknown-unknown` (`cargo xtask wasm`).
3. **Layering is Enforced:** Crates depend only downward (`cargo xtask layers`).
4. **Clean-Room:** All Apple platform integrations are implemented via public specs and official APIs.
5. **Isolated Native Interop:** `unsafe` is permitted only in the Apple bridge modules specified in `AGENTS.md`, behind safe Rust interfaces and with documented safety guarantees. Shared Rust logic remains safe.
