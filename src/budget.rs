//! The budget argument reading of the plan door.
//!
//! The argument is stripped of the characters the Python runtime
//! counts as space and must then stand as an optional sign and a
//! run of ASCII digits, exactly the shape gate the command line
//! applies. The Python conversion then reads the argument as
//! written rather than the stripped text, and four of the space
//! characters, the file, group, record, and unit separators
//! U+001C to U+001F, pass the gate at an edge and kill the
//! conversion uncaught, so the door refuses an argument edged
//! with one of them in the same wording, as the contract states
//! among its limits. A digit run past the runtime limit of 4300
//! digits kills the Python conversion the same way, and the door
//! refuses it the same way.

use crate::bigint::BigInt;
use crate::errors::SuitedietError;
use crate::render::repr_string;
use crate::tables::{in_ranges, STRIP_RANGES};

/// The longest digit run the Python conversion accepts.
const MAX_DIGIT_RUN: usize = 4300;

/// Trim the characters of a set from both edges of a text, the set
/// standing as a predicate over characters.
fn trim_edges(raw: &str, in_set: impl Fn(char) -> bool) -> &str {
    let start = raw
        .char_indices()
        .find(|(_, ch)| !in_set(*ch))
        .map(|(index, _)| index)
        .unwrap_or(raw.len());
    let end = raw
        .char_indices()
        .rev()
        .find(|(_, ch)| !in_set(*ch))
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    if start > end {
        ""
    } else {
        &raw[start..end]
    }
}

/// Read a budget argument into an exact integer, refusing an
/// argument that does not stand as a whole number with the
/// wording that names the argument as written.
pub fn parse_budget(raw: &str) -> Result<BigInt, SuitedietError> {
    let refusal = || {
        SuitedietError::new(format!(
            "the budget {} is not a whole number of milliseconds",
            repr_string(raw)
        ))
    };
    let stripped: &str = trim_edges(raw, |ch| in_ranges(STRIP_RANGES, ch as u32));
    // The conversion reads the argument as written and tolerates
    // every space character at an edge except the four separators,
    // so an argument whose edges differ under the two trimmings
    // carries a separator at an edge and is refused, where an
    // interior separator already fails the digit reading below.
    let tolerated: &str = trim_edges(raw, |ch| {
        in_ranges(STRIP_RANGES, ch as u32) && !(0x1C..=0x1F).contains(&(ch as u32))
    });
    if tolerated != stripped {
        return Err(refusal());
    }
    let digits = match stripped.strip_prefix('+') {
        Some(rest) => rest,
        None => stripped.strip_prefix('-').unwrap_or(stripped),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(refusal());
    }
    if digits.len() > MAX_DIGIT_RUN {
        return Err(refusal());
    }
    let signed = if stripped.starts_with('-') {
        format!("-{digits}")
    } else {
        digits.to_string()
    };
    BigInt::from_decimal(&signed).ok_or_else(refusal)
}
