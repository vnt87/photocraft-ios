# Ticket MAC-3: iPadOS Platform Services Bridge (UIKit FFI)

- **Assigned Session:** Codex on macOS
- **Branch:** `feature/ipados-xcode-runner`
- **Recommended Model:** `GPT-6.1-Sol`
- **Prerequisite:** `MAC-2`
- **Status:** READY TO PICK UP

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
- [ ] No `unwrap()` or `expect()`.
- [ ] Compiles cleanly for `aarch64-apple-ios`.
- [ ] Security-scoped URLs are guaranteed to call `stopAccessingSecurityScopedResource` when reading finishes.
