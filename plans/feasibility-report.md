# PhotoCraft on iPadOS: Comprehensive Feasibility & Architectural Report

> **Target Hardware:** iPad Air M3 (Apple Silicon ARM64, 8-core CPU, 10-core GPU, 8 GB Unified Memory)  
> **Target OS:** iPadOS 18 / 26 / 27+  
> **Feasibility Assessment:** **HIGH (Compiler verified on `aarch64-apple-ios` with 0 errors)**  
> **Date:** October 12, 2026

---

## 1. Executive Summary

Porting **PhotoCraft** to iPadOS on an **iPad Air M3** is **technically highly feasible and directly aligned** with the project's native Rust and Metal architecture.

During our active compiler and platform probe:
1. **100% Core Compilation Success:** The foundational layers L0–L5 (`photocraft-doc`, `photocraft-engine`, `photocraft-gpu`, `photocraft-compose`, `photocraft-paint`, `photocraft-vector`, `photocraft-text`, `photocraft-io`, `photocraft-codecs`, `photocraft-format`) and the UI layer L6 (`photocraft-ui-egui`) **compiled cleanly for the target `aarch64-apple-ios`** with zero errors using `rustc 1.99` and `wgpu 30`.
2. **Metal Acceleration on Apple Silicon:** `photocraft-gpu` and `eframe`/`egui-wgpu` build against Apple's Metal framework (`objc2-metal`, `wgpu 30`), which runs natively on the M3 GPU with unified memory.
3. **Clean Decoupling:** PhotoCraft's existing design cleanly abstracts platform services through [`photocraft_ui_egui::Services`](file:///Users/naam.vu/Downloads/Github/photocraft-ios/crates/ui-egui/src/lib.rs#L362-L449) and stylus hardware input through [`photocraft_ui_egui::stylus::StylusFeed`](file:///Users/naam.vu/Downloads/Github/photocraft-ios/crates/ui-egui/src/stylus.rs#L80-L100).
4. **Desktop Shell Isolation:** The failure of `apps/photocraft` to compile for iOS is strictly confined to desktop-only dependencies (`rfd`, `arboard`, AppKit/X11 tablet monitors, CLI argument parsing).
5. **No Tauri/Webview Rule:** Following `AGENTS.md` rules (100% Rust, no webview/Tauri shells), iPadOS will be supported via a dedicated sibling runner `apps/photocraft-ios` (mirroring how `apps/photocraft-web` handles browser wasm and `apps/photocraft` handles desktop).

---

## 2. Hardware Alignment: iPad Air M3

The iPad Air M3 provides desktop-class hardware:
- **Processor:** Apple M3 (4 performance cores + 4 efficiency cores).
- **GPU:** 10-core GPU with unified memory architecture, hardware-accelerated mesh shading, and Metal 3 support.
- **Memory:** 8 GB unified memory with ~100 GB/s bandwidth. PhotoCraft's COW tile raster system (`photocraft-raster`) and parallel rayon filters will perform at desktop-grade speeds.
- **Display:** 11" (2360 × 1640) or 13" (2732 × 2048) Liquid Retina display with P3 wide color gamut.
- **Stylus:** Apple Pencil (USB-C and Apple Pencil Pro), featuring pressure sensitivity, tilt/altitude, azimuth, barrel roll, double-tap, and hover sensing (enabled by the M3).

---

## 3. Technical Component Breakdown & Feasibility Matrix

| Component Area | Current Implementation | iPadOS Reality & Requirements | Feasibility | Effort |
|---|---|---|:---:|:---:|
| **Rendering & GPU** | `wgpu 30` + `egui-wgpu` + Metal on macOS | Supported directly via Metal on iOS (`CAMetalLayer`). M3 GPU texture limits and compute pipelines are fully compatible. | **100% Ready** | Low |
| **Document & Engine** | Pure Rust (`photocraft-doc`, `photocraft-engine`) | Fully portable; zero platform dependencies. | **100% Ready** | None |
| **Codecs & Formats** | Pure Rust codecs (PSD, PNG, JPEG, TIFF, OpenEXR, WebP, etc.) | Fully portable; all image read/write operates on byte buffers and memory streams. | **100% Ready** | None |
| **App Shell & Lifecycle** | Desktop `eframe::run_native` (winit Desktop event loop) | iOS requires `UIApplication`, `UIWindowScene`, and bundle structure (`Info.plist`). Can use `eframe`/`winit` iOS backend with a lightweight Xcode wrapper. | **Feasible** | Medium |
| **File Access & Sandboxing** | Desktop direct paths via `rfd` file dialogs | Sandboxed app container (`Documents/`, `tmp/`). Files outside sandbox require `UIDocumentPickerViewController` with security-scoped URLs. | **Solvable** | Medium |
| **Clipboard** | Desktop `arboard` (macOS NSPasteboard / Win32 / Wayland) | Must use `UIPasteboard` via `objc2-ui-kit` to copy/paste image and text buffers. | **Solvable** | Low |
| **Apple Pencil Input** | macOS AppKit event monitor (`photocraft-tablet`) | Needs UIKit touch event hook (`UITouch.type == .pencil`, `force`, `altitudeAngle`, `azimuthAngle`, `rollAngle`) writing to `StylusFeed`. | **Solvable** | Low-Medium |
| **Touch & Gestures** | Mouse-centric with basic trackpad scroll | Needs 2-finger pan, pinch-to-zoom, and rotation gestures on canvas; touch palm rejection when Pencil is active. | **Feasible** | Medium |
| **UI Layout & Density** | Fixed desktop layout tokens; ~20-28 pt buttons | Aligned design decision: Strict Desktop Density. Keeps 1:1 Photoshop desktop metrics, optimized for Apple Pencil & Magic Keyboard trackpad. | **Feasible** | Low |

---

## 4. Key Design Decisions Aligned from `/grill-me`

1. **App Shell Architecture:**
   Dedicated Rust app runner (`apps/photocraft-ios`) using `eframe` + `wgpu` Metal backend, with thin UIKit platform bindings (`objc2-ui-kit`, `objc2-foundation`). Retains the 100% Rust workspace architecture without violating `AGENTS.md` rules.
2. **UI Density & Layout:**
   Strict Desktop Density: Keep exact desktop pixel metrics and layout, optimized strictly for Apple Pencil and Magic Keyboard / trackpad precision on the iPad Air M3 Liquid Retina display.
3. **File Management:**
   Direct Editor with System File Picker: Launch directly into the PhotoCraft editor, using `UIDocumentPickerViewController` sheets for File → Open / Save As (supporting iCloud Drive, local iPad storage, and external drives with security-scoped URL access).
4. **Input & Gestures:**
   Pencil-for-Drawing + 2-Finger Canvas Gestures: Apple Pencil is strictly for drawing/tools (with palm rejection, pressure, tilt, hover preview, and barrel roll if supported); 2-finger gestures are reserved for canvas pan, pinch-zoom, and canvas rotate.
5. **Build, Packaging & Deployment:**
   Cargo xtask automation (`cargo xtask ios`) + lightweight Xcode wrapper project: seamless compilation of `aarch64-apple-ios` / `aarch64-apple-ios-sim`, automated signing, and one-click deployment to physical iPad Air M3.
