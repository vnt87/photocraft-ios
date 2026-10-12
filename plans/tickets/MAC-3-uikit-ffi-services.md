# Ticket MAC-3: iPadOS Platform Services Bridge (UIKit FFI)

- **Assigned Session:** Codex on macOS
- **Branch:** `feature/ipados-xcode-runner`
- **Recommended Model:** `GPT-6.1-Sol`
- **Prerequisite:** `MAC-2`
- **Status:** IMPLEMENTED / SIMULATOR-PARTIALLY-VERIFIED (2026-10-12); provider and physical-device acceptance pending

---

## 1. Context & Objective
PhotoCraft interacts with the platform via `photocraft_ui_egui::Services`.
On iPadOS, standard desktop facilities (`rfd` file dialogs, `arboard` clipboard) do not exist or fail to compile.
This ticket provides the native iPadOS implementations of `Services` via safe Objective-C FFI (`objc2-ui-kit`, `objc2-foundation`).

---

## 2. Technical Specification

### A. Implementation (`apps/photocraft-ios/src/services.rs`)
Implement `pub fn ios_services() -> Services`:

1. **`file_dialog` (`UIDocumentPickerViewController`):**
   - Present `UIDocumentPickerViewController` modally on top of the root `UIViewController`.
   - Configure picker for supported UTTypes:
     - `com.adobe.photoshop-image` (`.psd`, `.psb`)
     - `public.image` (`.png`, `.jpg`, `.tiff`, `.webp`)
     - Custom extension for `.pcraft`.
   - Wrap selection in `startAccessingSecurityScopedResourceWithURL:`.
   - Deliver file bytes asynchronously through `reply` channel.
   - For `Save As`: present picker in export mode (`initForExportingURLs:`).

2. **`clipboard_set_image` & `clipboard_get_image` (`UIPasteboard`):**
   - Read/write `UIPasteboard.generalPasteboard`.
   - Convert raw RGBA8 pixel buffers to `UIImage` (PNG representation) on copy.
   - Extract `UIImage` PNG representation to raw RGBA8 on paste.

3. **`load_prefs` & `save_prefs`:**
   - Locate sandboxed application support directory:
     `NSSearchPathForDirectoriesInDomains(NSApplicationSupportDirectory, NSUserDomainMask, YES)`.
   - Read/write `prefs.json`.

4. **`open_url`:**
   - Call `UIApplication.sharedApplication.openURL:options:completionHandler:`.

5. **`autosave`:**
   - Write snapshot `.pcraft` recovery states to sandboxed `Library/Caches/recovery/`.

---

## 3. Files to Touch
- `[NEW] apps/photocraft-ios/src/services.rs`

---

## 4. Verification & Acceptance Criteria
- [x] No `unwrap()` or `expect()` in non-test code.
- [x] Compiles cleanly for `aarch64-apple-ios`.
- [x] Security-scoped access is balanced by RAII for successful starts.

### Mac implementation / evidence (2026-10-12)

- Runner modules: `services.rs`, `documents.rs`/`documents/native.rs`, `clipboard.rs`, `storage.rs`.
- Native picker exports start only after encoding a staging file; saves use a worker even if background jobs are disabled. Native Cancel remains available after presentation; queued presentation can still be cancelled through the job.
- Managed snapshots and bounded atomic sidecar bookmarks support write-back. Failed import batches roll back their reservations. No bookmark tokens are stored in preferences.
- Durable recovery uses Application Support, avoiding purgeable Caches. Recents and recovery references can follow moved, supported app containers; external/archive paths are preserved.
- Simulator probes verify native clipboard, preferences, browser dispatch, and Open/Save picker presentation. Host tests verify recovery adoption, failed-write retry, codec depth, and moved-container behavior.
- Remaining: actual provider selection, successful external export/reopen/write-back, native cancellation, background/reopen lifecycle, and physical-device acceptance. Recent snapshots do not automatically fetch changes made by another app.
