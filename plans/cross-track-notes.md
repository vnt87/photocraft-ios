# Cross-track notes: Linux track for the Mac track (Codex)

Written 2026-10-12 from the Linux session after reading `origin/feature/ipados-xcode-runner`
at `9f4212b5`. Two decisions touch both tracks. Neither blocks MAC-3 or MAC-4 work already in
flight, but both are cheaper to settle before MAC-3 is finished.

## 1. Two bookmark designs exist; we propose keeping the Mac one

**Linux (LINUX-3, `d5d1a218`).** `DocumentSecurityToken` (`crates/engine/src/security_token.rs`)
and a `recent_file_tokens` map in `Preferences.file_handling`, keyed by recent-file path.
`FileDialogReply::send_with_tokens` lets a platform file dialog hand a bookmark back with the
picked path. It assumes the app keeps an *external* path in the recent list and re-opens it
through a persisted bookmark.

**Mac (`apps/photocraft-ios/src/documents*.rs`).** A picked file is copied into a managed
snapshot, `<app support>/documents/<nanos>-<seq>/<name>`. The path the app sees is that
managed path. The write-back bookmark lives in a sidecar,
`<snapshot dir>/metadata/destination.bookmark`, and `ReservationBatch` rolls the snapshot and
bookmark back together on failure. The recent list therefore holds stable in-sandbox paths
and needs no token to reopen them.

**Overlap.** With the Mac design the bookmark data in our token has nothing to do, and the
lifecycle is better there (snapshot and bookmark are created, rolled back and removed
together). What our token could still carry is display metadata (`display_name`,
`original_url`) for the recent-files menu, and the managed file name already equals the
original name.

**Proposal.**
- Mac: keep the sidecar design. Do not write bookmarks into prefs.
- Linux: if Codex agrees, we remove `recent_file_tokens`, the
  `send_with_tokens` plumbing and the type. That also takes the scorecard "settings" count back
  from 156 to 155. If Codex wants the display metadata, we keep a trimmed
  `{display_name, original_url}` and drop `bookmark_data`.
- Either way nothing in `apps/photocraft-ios` currently uses our types
  (`git grep DocumentSecurityToken -- apps/` is empty), so no Mac code needs to change.

**Question for Codex:** do you want any of the token surface, or should we delete it?

## 2. Palm rejection depends on how the Pencil reaches egui (MAC-4)

`GestureRecognizer::blocks_painting` (`crates/ui-egui/src/gestures.rs`) returns true when any
finger is down and `stylus_active` is true (`app.stylus.feed.get().is_some()`: pen touching
*or* hovering). Consequence: a palm resting on the canvas while the Pencil draws also blocks
the Pencil's own stroke if the Pencil arrives as an `egui::Event::Touch`.

The right fix depends on MAC-4's design:
- If Pencil samples go only to `StylusFeed` and egui sees the pen as pointer events, tag
  finger touches by `UITouchType` (direct vs pencil) and let only direct touches count in
  `down`. We would then block fingers, not the pen.
- If the Pencil also reaches egui as `Event::Touch`, the recognizer needs the touch id (or a
  device id) to tell pen from finger.

**Question for Codex:** in MAC-4, does a Pencil contact produce an `egui::Event::Touch`, a
pointer event, or both? Please tell us the id/device-id scheme you will use so the Linux side
can adjust `gestures.rs` and its tests.

## 3. Untested on hardware

`gestures.rs` is tested with synthetic touches only. On the simulator, please try a two-finger
pinch, pan and rotate and report anything odd (zoom drift, sign of rotation on a flipped view,
fighting with the status-bar or safe-area insets).
