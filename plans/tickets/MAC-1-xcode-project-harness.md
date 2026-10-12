# Ticket MAC-1: Xcode Project Harness & Packaging Structure

- **Assigned Session:** Codex on macOS
- **Branch:** `feature/ipados-xcode-runner`
- **Recommended Model:** `GPT-6.1-Sol`
- **Prerequisite:** macOS with Xcode 27+ installed
- **Status:** READY TO PICK UP

---

## 1. Context & Objective
To run PhotoCraft on an iPad Air M3 (and the iOS Simulator), the compiled Rust static library (`libphotocraft_ios.a`) needs a native iOS bundle container:
- An `Info.plist` configuring permissions, bundle identity, iPad orientation, Stage Manager resizability, and document types.
- A lightweight Xcode project (`PhotoCraft.xcodeproj`) that links the Rust static library and assets.
- Entitlements configured for standard app sandbox and document handling.

---

## 2. Technical Specification

### A. Packaging Directory Structure
Create `packaging/ios/`:
```text
packaging/ios/
  PhotoCraft/
    Info.plist
    PhotoCraft.entitlements
    Assets.xcassets/
      AppIcon.appiconset/
        Contents.json
  PhotoCraft.xcodeproj/
    project.pbxproj
```

### B. `Info.plist` Configuration
Ensure the following keys are set:
- `CFBundleIdentifier`: `ai.storyteller.photocraft`
- `CFBundleDisplayName`: `PhotoCraft`
- `CFBundleExecutable`: `PhotoCraft`
- `CFBundlePackageType`: `APPL`
- `LSRequiresIPhoneOS`: `true`
- `UISupportedInterfaceOrientations~ipad`:
  - `UIInterfaceOrientationLandscapeLeft`
  - `UIInterfaceOrientationLandscapeRight`
  - `UIInterfaceOrientationPortrait`
  - `UIInterfaceOrientationPortraitUpsideDown`
- `UIRequiresFullScreen`: `false` (critical: enables iPadOS Split View and Stage Manager resizability per `ipados-design-guidelines`).
- `LSSupportsOpeningDocumentsInPlace`: `true` (enables opening files in place from the Files app).
- `CFBundleDocumentTypes`: Register `.psd`, `.pcraft`, `.ora`, `.png`, `.jpg`, `.tiff`.

### C. Xcode Build Phase
Configure the project with a "Run Script" build phase:
```bash
if [ "$PLATFORM_NAME" == "iphonesimulator" ]; then
    TARGET="aarch64-apple-ios-sim"
else
    TARGET="aarch64-apple-ios"
fi
cargo build --package photocraft-ios --target "$TARGET" ${CARGO_FLAGS}
```
Link the produced `libphotocraft_ios.a` and necessary system frameworks (`Metal`, `MetalKit`, `UIKit`, `Foundation`, `QuartzCore`).

---

## 3. Files to Touch
- `[NEW] packaging/ios/PhotoCraft/Info.plist`
- `[NEW] packaging/ios/PhotoCraft/PhotoCraft.entitlements`
- `[NEW] packaging/ios/PhotoCraft.xcodeproj/project.pbxproj`

---

## 4. Verification & Acceptance Criteria
- [ ] Running `xcodebuild -project packaging/ios/PhotoCraft.xcodeproj -scheme PhotoCraft -destination 'generic/platform=iOS Simulator' build` succeeds.
- [ ] Generated `.app` bundle contains valid `Info.plist` and code signature structure.
