//! How the doors carry paths and argument bytes as text.
//!
//! The Python runtime carries undecodable bytes in paths and
//! arguments as the bytes themselves through its surrogateescape
//! handling. The Rust text type holds no such string, so the port
//! shows the replacement character in their place, as the
//! contract states among its limits. The reading and the refusal
//! stand the same on both sides, and only the echo of those bytes
//! in a message differs.

use std::path::Path;

/// The display text of a path, the lossy reading the doors use.
pub fn path_display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The text of raw argument or path bytes, decoded as UTF-8 with
/// the replacement character standing for undecodable bytes.
pub fn bytes_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}
