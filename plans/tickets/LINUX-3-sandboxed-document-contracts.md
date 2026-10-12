# Ticket LINUX-3: Sandboxed Document State & Persistence Contracts

- **Assigned Session:** Claude Code on Linux Dev Server
- **Branch:** `feature/ipados-core-contracts`
- **Recommended Model:** `Claude Sonnet 5.5`
- **Layer:** L5 (`photocraft-engine`), L6 (`photocraft-ui-egui`)
- **Status:** READY TO PICK UP

---

## 1. Context & Objective
On desktop platforms, PhotoCraft opens files from absolute filesystem paths (e.g. `/Users/user/photo.psd`).
On iPadOS, apps operate in a strict sandbox:
- Files from iCloud Drive or external USB drives are granted via **Security-Scoped Bookmarks** (`NSURL` security scope).
- If the app terminates, access to the external file is lost unless a security-scoped bookmark token was persisted.
- Crash recovery and autosave snapshots must live in the app's sandboxed container (`Library/Application Support/` or `Documents/`).

This ticket refines PhotoCraft's `Services` and engine contracts to cleanly support persistent bookmark tokens and sandboxed storage paths without breaking desktop or web runners.

---

## 2. Technical Specification

### A. Security-Scoped Token Contract (`crates/ui-egui/src/services.rs`)
Define a platform-neutral bookmark token abstraction:

```rust
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DocumentSecurityToken {
    /// Opaque base64 or identifier for security-scoped resource on iOS/macOS
    pub bookmark_data: Option<String>,
    pub display_name: String,
    pub original_url: Option<String>,
}
```

### B. Recent Documents with Security Tokens
- Ensure `Session` and `Preferences` recent file lists can store and rehydrate `DocumentSecurityToken`.
- Update `Services::file_dialog` reply to accept optional bookmark metadata.
- Ensure autosave recovery snapshots can be associated with external bookmark tokens.

---

## 3. Files to Touch
- `[MODIFY] crates/ui-egui/src/services.rs`
- `[MODIFY] crates/ui-egui/src/file_dialog.rs`
- `[MODIFY] crates/engine/src/prefs.rs`

---

## 4. Verification & Acceptance Criteria
- [ ] No `unwrap()` or `expect()` in non-test code.
- [ ] Serialization of recent file entries with and without security tokens is backward-compatible.
- [ ] All existing desktop and web tests pass without regression:
  ```bash
  cargo test -p photocraft-ui-egui
  cargo test -p photocraft-engine
  ```
