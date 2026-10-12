# Mac → Claude continuation — 2026-10-12

Start in this existing Mac checkout: `/Users/naam.vu/Downloads/Github/photocraft-ios`.
Branch: `feature/ipados-xcode-runner`; committed base: `b194ce01`.
Read `AGENTS.md`, this file, `MAC-simulator-validation.md`, then INT-1/2/3 tickets.
The older Linux-to-Mac handoff is historical. Its branch merge and duplicate
security-token design are already resolved; do not repeat them.

## Preserve the current working tree

Scene migration, hover repair, and acceptance documentation are **uncommitted**.
There are also concurrent UI/branding/documentation edits from another session.
Inspect `git diff` and preserve them; do not reset, blindly stash, or overwrite
shared files. A fresh remote checkout alone will miss the latest device fixes.

Our code changes: `apps/photocraft-ios/{Cargo.toml,src/lib.rs,src/scenes.rs}`,
`src/documents/native.rs`, `src/pencil/native.rs`, iOS Info.plist and harness tests,
and `crates/ui-egui/src/tool_cursor.rs` plus its tests. Runner/package READMEs and
plans document the results. The dev log under `log/` is gitignored.

## Verified physical acceptance

- iPad Air 13-inch M3, iPadOS 27.0 (24A437), wired; Developer Mode enabled.
- Apple Account signed in, development profile trusted. Signed Debug install,
  launch, sustained process, editor rendering, and Metal initialization pass.
- User confirms pressure-dependent brush width.
- New → save → reopen → edit → save persisted. Provider/format/restart details
  were not specified; don't infer iCloud or layered PSD fidelity from this.
- Canvas Pencil hover now follows the tip after the cursor repair.
- Tilt appears functional but subtle, and palm-before-pen produced no stray
  marks: provisional passes, not exhaustive sign-off.
- **Double-tap is deferred by user request; do not block integration on it.**

## Important fixes already implemented

- SDK/iPadOS 27 requires scenes. Rust `PhotoCraftSceneDelegate` registers a single
  UIWindowScene and attaches winit's existing window. Picker/Pencil window lookup
  is scene-owned; multiple windows are disabled. No winit fork or dependency added.
- iOS winit ignores OS cursor setters. Shared cursor code now chooses the existing
  painted fallback on iOS, retaining desktop bitmap cursors. Fourteen focused cursor
  tests pass (one visual preview ignored); hardware hover retest passed.
- Managed document snapshots and destination-bookmark sidecars remain authoritative.
  Container relocation repair is implemented; unused Linux token API is removed.

## Remaining work, in order

1. **INT-1:** reconcile outdated ticket statuses/prerequisites with actual merged
   work; final integration checks and commit/PR preparation. Linux-assist already
   integrated at `669635ca`; no redundant merge needed. Review uncommitted changes
   and coordinate ownership of concurrent UI edits before committing.
2. **Tooling gap:** device CLI currently assumes product bundle ID and generic
   destination. Actual deployment needed a development bundle override and the
   specific connected device for provisioning. Add regression-covered configurable
   bundle ID propagated through build/launch, and device-registration handling.
   Keep canonical repository product ID unchanged. Xcode 27 uses Device Hub;
   `--show-simulator` still targets older Simulator.app. Default CLI works.
3. **INT-2:** physical pinch zoom, two-finger pan, rotation; resizing/multitasking
   on installed iPadOS 27; no crash, lost document, or reset zoom. Test file-picker
   cancellation, layered PSD import, actual provider write-back and persisted
   bookmarks after app restart. Test background/termination recovery separately
   from saved-file persistence. Clipboard from another app remains unverified.
   Ask user to save before terminating/reinstalling; do not discard their work.
4. **INT-3:** update hardware parity, roadmap, target parity, scorecard and devlog
   using only demonstrated capabilities. Ticket's proposed blanket 'yes' for
   barrel rotation/eraser is not established. Record double-tap deferred and
   provisional evidence accurately. Do not claim full hardware sign-off yet.
5. Release builds, app icon, scene reconnect/safe-area behavior remain open.

## Build/install on this Mac

Reuse `CARGO_TARGET_DIR=target/ios-xcode`; disk is tight. Avoid independent cold
target directories or profile overrides. Remove only inactive task-owned
incremental caches when needed, after verifying no Cargo process uses them.

Physical device UDID: `00008122-0002309901E8401C`.
Development team: `T5CCKBXFS8`.
Installed development bundle ID: `ai.storyteller.photocraft.dev.t5cckbxfs8`.
The canonical `ai.storyteller.photocraft` could not be registered to this team.
Do not change it globally to solve local provisioning.

```sh
xcodebuild -project packaging/ios/PhotoCraft.xcodeproj -scheme PhotoCraft \
  -configuration Debug -destination 'id=00008122-0002309901E8401C' \
  -derivedDataPath target/ios-derived \
  -allowProvisioningUpdates -allowProvisioningDeviceRegistration \
  DEVELOPMENT_TEAM=T5CCKBXFS8 \
  PRODUCT_BUNDLE_IDENTIFIER=ai.storyteller.photocraft.dev.t5cckbxfs8 build
xcrun devicectl device install app --device 00008122-0002309901E8401C \
  target/ios-derived/Build/Products/Debug-iphoneos/PhotoCraft.app
xcrun devicectl device process launch --device 00008122-0002309901E8401C \
  ai.storyteller.photocraft.dev.t5cckbxfs8
```

Simulator: `78F2F043-0690-4026-B4EF-4EAE1EDF694E`, Air 11-inch M3 / iPadOS 26.2.
`cargo xtask ios --sim` builds/installs/launches the canonical bundle.

## Latest checks and evidence

- Signed device hover build/install/launch pass; user confirms repaired cursor.
- Device runner Clippy `--lib -- -D warnings`, layers (30 crates), and full
  `cargo xtask wasm` pass after the hover repair.
- Six packaging tests pass after scene migration. Device and simulator scene
  builds pass; simulator editor visually inspected.
- Cursor module rustfmt and diff checks pass. Whole UI package format check
  encountered concurrent unrelated `links.rs` formatting; coordinate that edit.
- Earlier baseline: full engine/UI package tests, 47 runner tests, 62 tooling
  tests, four gesture integration tests; see validation report for scope.
- Recent logs: `/tmp/photocraft-hover-{device-build,clippy,layers,wasm,fmt}.log`.
  Device runner log is inside app data at
  `Library/Application Support/ai.storyteller.photocraft/logs/runner.log`.

Follow repository final gates for changed crates. Do not treat tests alone as
physical acceptance or mark all INT-2 scenarios passed from partial evidence.
