//! Sandbox access tokens for documents (LINUX-3).
//!
//! On sandboxed platforms (iPadOS, macOS App Sandbox) a path alone does not grant access to a file
//! the user picked from iCloud Drive or a USB drive: the platform hands back a security-scoped
//! bookmark that must be persisted to reopen the file after a restart. The engine treats the
//! bookmark as opaque text; only the platform shell creates and resolves it. Desktop and web
//! shells never produce one.

use serde::{Deserialize, Serialize};

/// Longest `bookmark_data` kept, in bytes. Real bookmarks are a few KiB; this bounds a hand-edited
/// or hostile preferences file.
pub const MAX_BOOKMARK_BYTES: usize = 64 * 1024;
/// Longest `display_name` or `original_url` kept, in bytes.
pub const MAX_TOKEN_TEXT_BYTES: usize = 4096;

/// A persisted, platform-neutral handle to a document the app may only reach through the sandbox.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DocumentSecurityToken {
    /// Opaque base64 or identifier for the security-scoped resource on iOS/macOS.
    pub bookmark_data: Option<String>,
    pub display_name: String,
    pub original_url: Option<String>,
}

impl DocumentSecurityToken {
    /// Whether [`sanitized`](Self::sanitized) would return the token unchanged (no clone).
    pub fn is_sane(&self) -> bool {
        self.bookmark_data.as_ref().is_none_or(|b| !b.is_empty() && b.len() <= MAX_BOOKMARK_BYTES)
            && self.original_url.as_ref().is_none_or(|u| !u.is_empty() && u.len() <= MAX_TOKEN_TEXT_BYTES)
            && (self.bookmark_data.is_some() || self.original_url.is_some())
            && self.display_name.len() <= MAX_TOKEN_TEXT_BYTES
    }

    /// The token with every field within its size cap (names cut at a char boundary), or `None`
    /// when it carries nothing to rehydrate (no bookmark and no URL). An oversized bookmark or URL
    /// is dropped rather than truncated: a cut bookmark would never resolve.
    pub fn sanitized(mut self) -> Option<Self> {
        self.bookmark_data = self.bookmark_data.filter(|b| !b.is_empty() && b.len() <= MAX_BOOKMARK_BYTES);
        self.original_url = self.original_url.filter(|u| !u.is_empty() && u.len() <= MAX_TOKEN_TEXT_BYTES);
        if self.bookmark_data.is_none() && self.original_url.is_none() {
            return None;
        }
        if self.display_name.len() > MAX_TOKEN_TEXT_BYTES {
            let mut end = MAX_TOKEN_TEXT_BYTES;
            while end > 0 && !self.display_name.is_char_boundary(end) {
                end -= 1;
            }
            self.display_name.truncate(end);
        }
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(b: Option<&str>) -> DocumentSecurityToken {
        DocumentSecurityToken { bookmark_data: b.map(Into::into), display_name: "a.psd".into(), original_url: None }
    }

    #[test]
    fn empty_token_is_dropped() {
        assert_eq!(tok(None).sanitized(), None);
        assert_eq!(tok(Some("")).sanitized(), None);
    }

    #[test]
    fn oversized_bookmark_is_dropped_not_truncated() {
        let big = "A".repeat(MAX_BOOKMARK_BYTES + 1);
        assert_eq!(tok(Some(&big)).sanitized(), None);
        let ok = "A".repeat(MAX_BOOKMARK_BYTES);
        assert_eq!(tok(Some(&ok)).sanitized().and_then(|t| t.bookmark_data).map(|b| b.len()), Some(MAX_BOOKMARK_BYTES));
    }

    #[test]
    fn long_name_is_cut_at_a_char_boundary() {
        let mut t = tok(Some("x"));
        t.display_name = "é".repeat(MAX_TOKEN_TEXT_BYTES);
        let t = t.sanitized().unwrap();
        assert!(t.display_name.len() <= MAX_TOKEN_TEXT_BYTES);
        assert!(t.display_name.chars().all(|c| c == 'é'));
    }

    #[test]
    fn serde_round_trip_and_missing_fields() {
        let t = DocumentSecurityToken { bookmark_data: Some("QUJD".into()), display_name: "p.png".into(), original_url: Some("file:///p.png".into()) };
        let j = serde_json::to_string(&t).unwrap();
        assert_eq!(serde_json::from_str::<DocumentSecurityToken>(&j).unwrap(), t);
        assert_eq!(serde_json::from_str::<DocumentSecurityToken>("{}").unwrap(), DocumentSecurityToken::default());
    }
}
