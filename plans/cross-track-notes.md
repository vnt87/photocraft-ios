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

## 2. Palm rejection (updated after the Linux-assist MAC-4 work)

Resolved on the Linux side, see `plans/HANDOFF-linux-to-mac.md`. winit delivers the Pencil as an
ordinary `WindowEvent::Touch` (fingers and pen share one device id; the touch id is the `UITouch`
address), so `apps/photocraft-ios/src/pencil/native.rs` records the pen's touch ids and
`StylusFeed` remembers them. `GestureRecognizer::feed_frame_with` keeps pen touches out of the
finger count, which also fixes a lone pen counting as a palm.

One case is still open: a palm that lands *before* the pen owns egui's emulated pointer, so
painting stays blocked until the palm lifts. Fixing it needs egui-winit to hand the pointer to
the pen, or the palm touch cancelled natively.

## 3. Untested on hardware

`gestures.rs` is tested with synthetic touches only. On the simulator, please try a two-finger
pinch, pan and rotate and report anything odd (zoom drift, sign of rotation on a flipped view,
fighting with the status-bar or safe-area insets).
