//! Rendering values the way the Python renders them.
//!
//! The canonical written form spells a document with sorted keys
//! and two space indentation and one trailing newline, with every
//! character outside the printable ASCII range escaped, as the
//! Python encoder spells it by default. The representation
//! spelling follows the Python `repr` of a value, which the
//! refusal messages carry.

use crate::jsonvalue::JsonValue;
use crate::tables::{in_ranges, NON_PRINTABLE_RANGES};

/// Spell a finite float as Python spells it in its shortest round
/// trip form, fixed notation inside the exponent window the
/// language uses and scientific notation outside it.
fn spell_decimal(value: f64) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0.0".to_string()
        } else {
            "0.0".to_string()
        };
    }
    let negative = value < 0.0;
    let magnitude = value.abs();
    // The shortest digits that round trip, with the decimal
    // exponent of the first digit, read off the scientific
    // spelling.
    let scientific = format!("{:e}", magnitude);
    let (mantissa, exponent_text) = scientific
        .split_once('e')
        .expect("the scientific spelling carries an exponent");
    let exponent: i32 = exponent_text
        .parse()
        .expect("the exponent is a whole number");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let mut spelled = String::new();
    if negative {
        spelled.push('-');
    }
    if (-4..16).contains(&exponent) {
        if exponent >= 0 {
            let point = (exponent + 1) as usize;
            if digits.len() > point {
                spelled.push_str(&digits[..point]);
                spelled.push('.');
                spelled.push_str(&digits[point..]);
            } else {
                spelled.push_str(&digits);
                for _ in 0..(point - digits.len()) {
                    spelled.push('0');
                }
                spelled.push_str(".0");
            }
        } else {
            spelled.push_str("0.");
            for _ in 0..(-exponent - 1) {
                spelled.push('0');
            }
            spelled.push_str(&digits);
        }
    } else {
        spelled.push_str(&digits[..1]);
        if digits.len() > 1 {
            spelled.push('.');
            spelled.push_str(&digits[1..]);
        }
        spelled.push('e');
        if exponent < 0 {
            spelled.push('-');
        } else {
            spelled.push('+');
        }
        let width = format!("{:02}", exponent.abs());
        spelled.push_str(&width);
    }
    spelled
}

/// The spelling Python gives a float in a written JSON value, as
/// the encoder writes it.
pub fn json_float_text(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value.is_infinite() {
        if value > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        }
    } else {
        spell_decimal(value)
    }
}

/// The spelling Python gives a float as its representation.
pub fn repr_float_text(value: f64) -> String {
    if value.is_nan() {
        "nan".to_string()
    } else if value.is_infinite() {
        if value > 0.0 {
            "inf".to_string()
        } else {
            "-inf".to_string()
        }
    } else {
        spell_decimal(value)
    }
}

/// Whether a character stands raw in a Python representation, read
/// off the generated printable table.
fn python_printable(ch: char) -> bool {
    !in_ranges(NON_PRINTABLE_RANGES, ch as u32)
}

/// The spelling Python gives a text as its representation, single
/// quotes standing unless the text carries a single quote and no
/// double quote, with the backslash and the standing quote and the
/// line and tab characters escaped and every character outside the
/// printable set spelled by its code point.
pub fn repr_string(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            standing if standing == quote => {
                out.push('\\');
                out.push(standing);
            }
            standing if python_printable(standing) => out.push(standing),
            standing => {
                let code = standing as u32;
                if code < 0x100 {
                    out.push_str(&format!("\\x{:02x}", code));
                } else if code < 0x10000 {
                    out.push_str(&format!("\\u{:04x}", code));
                } else {
                    out.push_str(&format!("\\U{:08x}", code));
                }
            }
        }
    }
    out.push(quote);
    out
}

/// The spelling Python gives a value as its representation, a list
/// in brackets and an object in braces with its fields in the
/// order they stand, and the parts joined as the language joins
/// them.
pub fn repr_value(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "None".to_string(),
        JsonValue::Bool(word) => {
            if *word {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        JsonValue::Int(whole) => whole.to_string(),
        JsonValue::BigInt(digits) => digits.clone(),
        JsonValue::Float(decimal) => repr_float_text(*decimal),
        JsonValue::Text(text) => repr_string(text),
        JsonValue::List(items) => {
            let parts: Vec<String> = items.iter().map(repr_value).collect();
            format!("[{}]", parts.join(", "))
        }
        JsonValue::Object(fields) => {
            let parts: Vec<String> = fields
                .iter()
                .map(|(key, held)| format!("{}: {}", repr_string(key), repr_value(held)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
    }
}

/// Escape a text as the Python encoder escapes one in a written
/// document, every character outside the printable ASCII range
/// spelled by its code point, with the short escapes standing for
/// the characters that carry them.
fn escape_text(text: &str, out: &mut String) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            other => {
                let code = other as u32;
                if (0x20..0x7F).contains(&code) {
                    out.push(other);
                } else if code < 0x10000 {
                    out.push_str(&format!("\\u{:04x}", code));
                } else {
                    // An astral character stands as a surrogate
                    // pair of escapes.
                    let shifted = code - 0x1_0000;
                    let high = 0xD800 + (shifted >> 10);
                    let low = 0xDC00 + (shifted & 0x3FF);
                    out.push_str(&format!("\\u{:04x}\\u{:04x}", high, low));
                }
            }
        }
    }
    out.push('"');
}

fn render_value(value: &JsonValue, depth: usize, out: &mut String) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(word) => out.push_str(if *word { "true" } else { "false" }),
        JsonValue::Int(whole) => out.push_str(&whole.to_string()),
        JsonValue::BigInt(digits) => out.push_str(digits),
        JsonValue::Float(decimal) => out.push_str(&json_float_text(*decimal)),
        JsonValue::Text(text) => escape_text(text, out),
        JsonValue::List(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            let inner_pad = "  ".repeat(depth + 1);
            let outer_pad = "  ".repeat(depth);
            out.push_str("[\n");
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                out.push_str(&inner_pad);
                render_value(item, depth + 1, out);
            }
            out.push('\n');
            out.push_str(&outer_pad);
            out.push(']');
        }
        JsonValue::Object(fields) => {
            if fields.is_empty() {
                out.push_str("{}");
                return;
            }
            let mut sorted: Vec<&(String, JsonValue)> = fields.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            let inner_pad = "  ".repeat(depth + 1);
            let outer_pad = "  ".repeat(depth);
            out.push_str("{\n");
            for (index, (key, item)) in sorted.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                out.push_str(&inner_pad);
                escape_text(key, out);
                out.push_str(": ");
                render_value(item, depth + 1, out);
            }
            out.push('\n');
            out.push_str(&outer_pad);
            out.push('}');
        }
    }
}

/// Render a document in the canonical written form the Python
/// `canonical_text` writes, sorted keys and two space indentation
/// and one trailing newline.
pub fn canonical_text(document: &JsonValue) -> String {
    let mut out = String::new();
    render_value(document, 0, &mut out);
    out.push('\n');
    out
}
