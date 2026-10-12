# Ticket MAC-2: Dedicated App Runner & Metal Lifecycle (`apps/photocraft-ios`)

- **Assigned Session:** Codex on macOS
- **Branch:** `feature/ipados-xcode-runner`
- **Recommended Model:** `GPT-6.1-Sol`
- **Prerequisite:** `MAC-1`
- **Status:** READY TO PICK UP

---

## 1. Context & Objective
Following `AGENTS.md` guidelines, PhotoCraft must maintain a dedicated app runner for iPadOS (`apps/photocraft-ios`), similar to `apps/photocraft` (desktop) and `apps/photocraft-web` (browser).
This crate connects:
1. `eframe` / `egui-winit` for the iOS event loop.
2. `wgpu 30` configured with the Metal backend on Apple Silicon M3 unified memory.
3. The sandboxed log directory in `Library/Application Support/ai.storyteller.photocraft/logs/`.

---

## 2. Technical Specification

### A. Manifest (`apps/photocraft-ios/Cargo.toml`)
```toml
[package]
name = "photocraft-ios"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "PhotoCraft on iPadOS"

[lib]
name = "photocraft_ios"
crate-type = ["staticlib", "cdylib"]

[[bin]]
name = "photocraft-ios"
path = "src/main.rs"

[lints]
workspace = true

[dependencies]
photocraft-doc = { workspace = true }
photocraft-color = { workspace = true }
photocraft-raster = { workspace = true }
photocraft-geom = { workspace = true }
photocraft-compose = { workspace = true }
photocraft-engine = { workspace = true }
photocraft-codecs = { workspace = true }
photocraft-io = { workspace = true }
photocraft-format = { workspace = true }
photocraft-ui-egui = { workspace = true }
eframe = { workspace = true, features = ["default_fonts", "wgpu", "accesskit"] }
egui = { workspace = true }
serde_json = { workspace = true }
log = { workspace = true }

[target.'cfg(target_os = "ios")'.dependencies]
objc2 = "=0.6.4"
objc2-foundation = { version = "=0.3.2", default-features = false, features = ["std", "NSString", "NSArray", "NSData", "NSURL", "NSBundle", "NSFileManager", "NSGeometry"] }
objc2-ui-kit = { version = "=0.3.2", default-features = false, features = ["std", "UIApplication", "UIWindow", "UIViewController", "UIDocumentPickerViewController", "UIPasteboard", "UIImage", "UITouch", "UIEvent"] }
block2 = "=0.6.2"
```

### B. Entry Point (`apps/photocraft-ios/src/lib.rs` & `src/main.rs`)
- Expose `pub extern "C" fn photocraft_ios_main() -> i32` for iOS app launch.
- Initialize `NativeOptions` with full-screen iPad viewport and `wgpu` Metal state.
- Wire in `PhotocraftApp::new(Session::new(), services::ios_services())`.
- In `PhotocraftApp::on_started`, configure window viewport dimensions for iPad Air M3 (Liquid Retina).

---

## 3. Files to Touch
- `[NEW] apps/photocraft-ios/Cargo.toml`
- `[NEW] apps/photocraft-ios/src/lib.rs`
- `[NEW] apps/photocraft-ios/src/main.rs`

---

## 4. Verification & Acceptance Criteria
- [ ] Builds with zero errors:
  ```bash
  cargo check --target aarch64-apple-ios -p photocraft-ios
  cargo check --target aarch64-apple-ios-sim -p photocraft-ios
  ```
- [ ] No `unwrap()` or `expect()` anywhere in non-test code.
