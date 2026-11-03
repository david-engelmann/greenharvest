//! The one refusal type every suitediet reading raises.
//!
//! Mirrors the Python `SuitedietError`.

use std::fmt;

/// Raised when an input is refused rather than guessed.
///
/// Mirrors the Python `SuitedietError`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuitedietError {
    message: String,
}

impl SuitedietError {
    pub fn new(message: impl Into<String>) -> Self {
        SuitedietError {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for SuitedietError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SuitedietError {}
