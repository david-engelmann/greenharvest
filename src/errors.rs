//! The one refusal type every greenharvest reading raises.
//!
//! Mirrors the Python error type.

use std::fmt;

/// Raised when an input is refused rather than guessed.
///
/// Mirrors the Python error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreenharvestError {
    message: String,
}

impl GreenharvestError {
    pub fn new(message: impl Into<String>) -> Self {
        GreenharvestError {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for GreenharvestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for GreenharvestError {}
