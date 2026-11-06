//! The suitediet binary, carrying the command line whole over
//! the ported core.
//!
//! The shell reads its arguments the way the Python argument
//! parser reads them, two doors named plan and restore, each
//! with its own flags and positionals, and it carries argument
//! bytes end to end as OS strings so a value it hands to the
//! filesystem is the value it was given. The texts the Python
//! parser generates stand outside parity, as the contract
//! records, so a usage refusal and a help answer print the
//! short usage line of the door that answers. Every line the
//! tool itself prints is carried exactly.

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use suitediet::bigint::BigInt;
use suitediet::ingest::{
    quarantine_document, read_quarantine, read_records, read_remaining, records_document,
};
use suitediet::pytext::bytes_text;
use suitediet::tables::{in_ranges, NON_PRINTABLE_RANGES};
use suitediet::{
    build_plan, build_quarantine, canonical_text, markdown_text, parse_budget, remaining_records,
    report_document, restore_records,
};

const USAGE_TOP: &str = "usage: suitediet {plan,restore} ...";
const USAGE_PLAN: &str = "usage: suitediet plan RECORDS --budget-ms BUDGET --out-dir OUT_DIR";
const USAGE_RESTORE: &str = "usage: suitediet restore REMAINING QUARANTINE --out OUT";

/// The code points of the Unicode decimal number category, as
/// inclusive ranges, generated from the CPython runtime on the
/// building machine, unicodedata 15.0.0. The argument reader
/// matches a word that looks like a negative number against
/// this category, as the Python reader does.
static ND_RANGES: &[(u32, u32)] = &[
    (0x0030, 0x0039),
    (0x0660, 0x0669),
    (0x06F0, 0x06F9),
    (0x07C0, 0x07C9),
    (0x0966, 0x096F),
    (0x09E6, 0x09EF),
    (0x0A66, 0x0A6F),
    (0x0AE6, 0x0AEF),
    (0x0B66, 0x0B6F),
    (0x0BE6, 0x0BEF),
    (0x0C66, 0x0C6F),
    (0x0CE6, 0x0CEF),
    (0x0D66, 0x0D6F),
    (0x0DE6, 0x0DEF),
    (0x0E50, 0x0E59),
    (0x0ED0, 0x0ED9),
    (0x0F20, 0x0F29),
    (0x1040, 0x1049),
    (0x1090, 0x1099),
    (0x17E0, 0x17E9),
    (0x1810, 0x1819),
    (0x1946, 0x194F),
    (0x19D0, 0x19D9),
    (0x1A80, 0x1A89),
    (0x1A90, 0x1A99),
    (0x1B50, 0x1B59),
    (0x1BB0, 0x1BB9),
    (0x1C40, 0x1C49),
    (0x1C50, 0x1C59),
    (0xA620, 0xA629),
    (0xA8D0, 0xA8D9),
    (0xA900, 0xA909),
    (0xA9D0, 0xA9D9),
    (0xA9F0, 0xA9F9),
    (0xAA50, 0xAA59),
    (0xABF0, 0xABF9),
    (0xFF10, 0xFF19),
    (0x104A0, 0x104A9),
    (0x10D30, 0x10D39),
    (0x11066, 0x1106F),
    (0x110F0, 0x110F9),
    (0x11136, 0x1113F),
    (0x111D0, 0x111D9),
    (0x112F0, 0x112F9),
    (0x11450, 0x11459),
    (0x114D0, 0x114D9),
    (0x11650, 0x11659),
    (0x116C0, 0x116C9),
    (0x11730, 0x11739),
    (0x118E0, 0x118E9),
    (0x11950, 0x11959),
    (0x11C50, 0x11C59),
    (0x11D50, 0x11D59),
    (0x11DA0, 0x11DA9),
    (0x11F50, 0x11F59),
    (0x16A60, 0x16A69),
    (0x16AC0, 0x16AC9),
    (0x16B50, 0x16B59),
    (0x1D7CE, 0x1D7FF),
    (0x1E140, 0x1E149),
    (0x1E2F0, 0x1E2F9),
    (0x1E4F0, 0x1E4F9),
    (0x1E950, 0x1E959),
    (0x1FBF0, 0x1FBF9),
];

fn write_stderr(text: &str) {
    let _ = std::io::stderr().write_all(text.as_bytes());
    let _ = std::io::stderr().write_all(b"\n");
}

fn write_stdout(text: &str) {
    let _ = std::io::stdout().write_all(text.as_bytes());
    let _ = std::io::stdout().write_all(b"\n");
}

/// A refusal of the library, printed in the tool's own error
/// line and standing at the refusal code.
fn fail(message: &str) -> ExitCode {
    write_stderr(&format!("error: {message}"));
    ExitCode::from(2)
}

/// A usage refusal, the usage line of the parser that answers
/// standing in place of the generated text it would print.
fn usage_exit(line: &str) -> ExitCode {
    write_stderr(line);
    ExitCode::from(2)
}

/// A help answer, the usage line standing in place of the
/// generated help text, at the clean code.
fn help_exit(line: &str) -> ExitCode {
    write_stdout(line);
    ExitCode::SUCCESS
}

/// Decode bytes the way the Python runtime decodes its argument
/// strings, valid UTF-8 standing as its characters and every
/// byte of an undecodable span standing as the surrogate code
/// point of its own value, U+DC00 plus the byte.
fn decode_units(bytes: &[u8]) -> Vec<u32> {
    fn continuation(byte: u8) -> bool {
        (0x80..=0xBF).contains(&byte)
    }
    let mut units = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let lead = bytes[index];
        if lead < 0x80 {
            units.push(lead as u32);
            index += 1;
            continue;
        }
        // The length of the sequence the lead byte opens and
        // the range its first continuation byte must stand in
        // for the sequence to spell a scalar. A lead byte that
        // opens no valid sequence stands alone.
        let (length, first_range): (usize, Option<(u8, u8)>) = match lead {
            0xC2..=0xDF => (2, None),
            0xE0 => (3, Some((0xA0, 0xBF))),
            0xE1..=0xEC | 0xEE..=0xEF => (3, None),
            0xED => (3, Some((0x80, 0x9F))),
            0xF0 => (4, Some((0x90, 0xBF))),
            0xF1..=0xF3 => (4, None),
            0xF4 => (4, Some((0x80, 0x8F))),
            _ => (0, None),
        };
        if length == 0 {
            units.push(0xDC00 + lead as u32);
            index += 1;
            continue;
        }
        let mut scalar: Option<u32> = None;
        if index + length <= bytes.len() {
            let rest = &bytes[index + 1..index + length];
            let first_stands = match first_range {
                Some((low, high)) => (low..=high).contains(&rest[0]),
                None => continuation(rest[0]),
            };
            let rest_stand = rest[1..].iter().all(|byte| continuation(*byte));
            if first_stands && rest_stand {
                let mut value = (lead as u32) & (0x7F >> length);
                for byte in rest {
                    value = (value << 6) | (byte & 0x3F) as u32;
                }
                scalar = Some(value);
            }
        }
        match scalar {
            Some(value) => {
                units.push(value);
                index += length;
            }
            None => {
                // The span the decoder reports is the lead byte
                // and the continuation bytes that follow it up
                // to the length of the sequence it opened, each
                // standing as its own surrogate.
                let mut consumed = 1;
                while consumed < length
                    && index + consumed < bytes.len()
                    && continuation(bytes[index + consumed])
                {
                    consumed += 1;
                }
                for offset in 0..consumed {
                    units.push(0xDC00 + bytes[index + offset] as u32);
                }
                index += consumed;
            }
        }
    }
    units
}

/// One argument word, carried as the OS string it arrived as
/// and as the code points the Python runtime would read it as.
#[derive(Clone)]
struct Word {
    raw: OsString,
    bytes: Vec<u8>,
    units: Vec<u32>,
}

impl Word {
    fn new(raw: &OsString) -> Word {
        let bytes = word_bytes(raw);
        let units = decode_units(&bytes);
        Word {
            raw: raw.clone(),
            bytes,
            units,
        }
    }

    fn from_bytes(bytes: &[u8]) -> Word {
        let raw = word_from_bytes(bytes);
        Word {
            raw,
            bytes: bytes.to_vec(),
            units: decode_units(bytes),
        }
    }

    fn is_text(&self, text: &str) -> bool {
        self.units.len() == text.chars().count()
            && self
                .units
                .iter()
                .zip(text.chars())
                .all(|(unit, ch)| *unit == ch as u32)
    }

    /// Whether the word carries a byte that is not UTF-8 text,
    /// standing in its units as a surrogate.
    fn carries_surrogate(&self) -> bool {
        self.units
            .iter()
            .any(|unit| (0xDC80..=0xDCFF).contains(unit))
    }
}

fn word_bytes(raw: &OsString) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        raw.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        raw.to_string_lossy().into_owned().into_bytes()
    }
}

fn word_from_bytes(bytes: &[u8]) -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(bytes.to_vec())
    }
    #[cfg(not(unix))]
    {
        OsString::from(String::from_utf8_lossy(bytes).into_owned())
    }
}

/// The spelling the Python representation gives a word, over
/// the word's code points, so a byte that is not UTF-8 text
/// spells as the escape of its surrogate exactly as the landed
/// command line spells it in its budget refusal.
fn repr_units(units: &[u32]) -> String {
    let quote = if units.contains(&(b'\'' as u32)) && !units.contains(&(b'"' as u32)) {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for unit in units {
        let escaped = match *unit {
            unit if unit == b'\\' as u32 => Some("\\\\".to_string()),
            unit if unit == b'\n' as u32 => Some("\\n".to_string()),
            unit if unit == b'\r' as u32 => Some("\\r".to_string()),
            unit if unit == b'\t' as u32 => Some("\\t".to_string()),
            unit if unit == quote as u32 => Some(format!("\\{quote}")),
            _ => None,
        };
        if let Some(text) = escaped {
            out.push_str(&text);
            continue;
        }
        let printable =
            !(0xD800..=0xDFFF).contains(unit) && !in_ranges(NON_PRINTABLE_RANGES, *unit);
        if printable {
            if let Some(ch) = char::from_u32(*unit) {
                out.push(ch);
                continue;
            }
        }
        if *unit < 0x100 {
            out.push_str(&format!("\\x{:02x}", unit));
        } else if *unit < 0x10000 {
            out.push_str(&format!("\\u{:04x}", unit));
        } else {
            out.push_str(&format!("\\U{:08x}", unit));
        }
    }
    out.push(quote);
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FlagId {
    Help,
    BudgetMs,
    OutDir,
    Out,
}

#[derive(Clone, Copy)]
struct OptionSpec {
    text: &'static str,
    flag: FlagId,
}

struct Spec {
    options: &'static [OptionSpec],
    positional_count: usize,
    required: &'static [FlagId],
}

const TOP_SPEC: Spec = Spec {
    options: &[
        OptionSpec {
            text: "-h",
            flag: FlagId::Help,
        },
        OptionSpec {
            text: "--help",
            flag: FlagId::Help,
        },
    ],
    positional_count: 1,
    required: &[],
};

const PLAN_SPEC: Spec = Spec {
    options: &[
        OptionSpec {
            text: "-h",
            flag: FlagId::Help,
        },
        OptionSpec {
            text: "--help",
            flag: FlagId::Help,
        },
        OptionSpec {
            text: "--budget-ms",
            flag: FlagId::BudgetMs,
        },
        OptionSpec {
            text: "--out-dir",
            flag: FlagId::OutDir,
        },
    ],
    positional_count: 1,
    required: &[FlagId::BudgetMs, FlagId::OutDir],
};

const RESTORE_SPEC: Spec = Spec {
    options: &[
        OptionSpec {
            text: "-h",
            flag: FlagId::Help,
        },
        OptionSpec {
            text: "--help",
            flag: FlagId::Help,
        },
        OptionSpec {
            text: "--out",
            flag: FlagId::Out,
        },
    ],
    positional_count: 2,
    required: &[FlagId::Out],
};

/// The standing of one word under a parser's option spellings,
/// as its classification pass reads it.
enum Class {
    /// Pattern A, a word the parser reads as a positional.
    Argument,
    /// Pattern O with no option behind it, an extra.
    Unknown,
    /// The help option with no explicit argument standing.
    HelpPlain,
    /// The help option with an explicit argument standing,
    /// refused when the word is consumed.
    HelpValued,
    /// The help option heading a cluster whose tail does not
    /// open with a prefix character, answered as plain help.
    HelpCluster,
    /// The help option heading a cluster whose tail opens
    /// with a prefix character, refused when consumed.
    HelpClusterValued,
    /// A flag that takes a value, with the value bytes where
    /// the word carries them after an equals sign.
    ValueFlag(FlagId, Option<Vec<u8>>),
}

/// Whether a word reads as a negative number to the argument
/// reader, a minus and a run of decimal digits, or a minus and
/// digits around one point with at least one digit after it.
fn is_negative_number(units: &[u32]) -> bool {
    fn digit(unit: u32) -> bool {
        in_ranges(ND_RANGES, unit)
    }
    if units.first() != Some(&(b'-' as u32)) || units.len() < 2 {
        return false;
    }
    let rest = &units[1..];
    if rest.iter().all(|unit| digit(*unit)) {
        return true;
    }
    match rest.iter().position(|unit| *unit == b'.' as u32) {
        Some(point) => {
            rest[..point].iter().all(|unit| digit(*unit))
                && !rest[point + 1..].is_empty()
                && rest[point + 1..].iter().all(|unit| digit(*unit))
        }
        None => false,
    }
}

fn option_units(text: &str) -> Vec<u32> {
    text.chars().map(|ch| ch as u32).collect()
}

/// The value bytes a word carries after its first equals sign.
fn explicit_bytes(word: &Word) -> Vec<u8> {
    match word.bytes.iter().position(|byte| *byte == b'=') {
        Some(index) => word.bytes[index + 1..].to_vec(),
        None => Vec::new(),
    }
}

/// Classify one word under a parser, following the Python
/// reader step for step. An ambiguous long prefix fails the
/// classification pass itself, ahead of any word's consumption.
fn classify(word: &Word, spec: &Spec) -> Result<Class, ()> {
    if word.units.is_empty() || word.units[0] != b'-' as u32 {
        return Ok(Class::Argument);
    }
    for option in spec.options {
        if word.units == option_units(option.text) {
            return Ok(match option.flag {
                FlagId::Help => Class::HelpPlain,
                flag => Class::ValueFlag(flag, None),
            });
        }
    }
    if word.units.len() == 1 {
        return Ok(Class::Argument);
    }
    let equals = word.units.iter().position(|unit| *unit == b'=' as u32);
    if let Some(point) = equals {
        let head = &word.units[..point];
        for option in spec.options {
            if head == option_units(option.text).as_slice() {
                return Ok(match option.flag {
                    FlagId::Help => Class::HelpValued,
                    flag => Class::ValueFlag(flag, Some(explicit_bytes(word))),
                });
            }
        }
    }
    if word.units[1] == b'-' as u32 {
        let prefix = match equals {
            Some(point) => &word.units[..point],
            None => &word.units[..],
        };
        let mut matched: Option<FlagId> = None;
        let mut matches = 0;
        for option in spec.options {
            if !option.text.starts_with("--") {
                continue;
            }
            let standing = option_units(option.text);
            if standing.len() >= prefix.len() && standing[..prefix.len()] == *prefix {
                if matched != Some(option.flag) {
                    matches += 1;
                    matched = Some(option.flag);
                }
            }
        }
        if matches > 1 {
            return Err(());
        }
        if let Some(flag) = matched {
            return Ok(match flag {
                FlagId::Help => {
                    if equals.is_some() {
                        Class::HelpValued
                    } else {
                        Class::HelpPlain
                    }
                }
                flag => Class::ValueFlag(
                    flag,
                    if equals.is_some() {
                        Some(explicit_bytes(word))
                    } else {
                        None
                    },
                ),
            });
        }
    } else if word.units[1] == b'h' as u32
        && spec
            .options
            .iter()
            .any(|option| option.text == "-h" && option.flag == FlagId::Help)
    {
        // The one short option of these parsers heads a
        // cluster. Its tail answers as help unless the tail
        // opens with a prefix character, which the reader
        // refuses as an explicit argument it ignores. The
        // equals forms of the short option were read above.
        let tail = &word.units[2..];
        if !tail.is_empty() {
            return Ok(if tail[0] == b'-' as u32 {
                Class::HelpClusterValued
            } else {
                Class::HelpCluster
            });
        }
    }
    if is_negative_number(&word.units) {
        return Ok(Class::Argument);
    }
    if word.units.contains(&(b' ' as u32)) {
        return Ok(Class::Argument);
    }
    Ok(Class::Unknown)
}

/// The pattern of one parser pass over a word list, A for a
/// positional word, O for an option word, and the separator
/// standing for the first double dash, after which every word
/// reads as a positional.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pat {
    A,
    O,
    Dash,
}

fn pattern_pass(words: &[Word], spec: &Spec) -> Result<(Vec<Pat>, Vec<Option<Class>>), ()> {
    let mut pats = Vec::with_capacity(words.len());
    let mut classes = Vec::with_capacity(words.len());
    let mut flipped = false;
    for word in words {
        if !flipped && word.bytes == b"--" {
            pats.push(Pat::Dash);
            classes.push(None);
            flipped = true;
        } else if flipped {
            pats.push(Pat::A);
            classes.push(None);
        } else {
            match classify(word, spec)? {
                Class::Argument => {
                    pats.push(Pat::A);
                    classes.push(None);
                }
                class => {
                    pats.push(Pat::O);
                    classes.push(Some(class));
                }
            }
        }
    }
    Ok((pats, classes))
}

/// Match as many positional groups as the reader's pattern
/// match matches at a start index, largest count first, and
/// give the length in words of each group that matched. A
/// group is any run of separator words, one positional word,
/// and any run of separator words.
fn match_groups(pats: &[Pat], start: usize, count: usize) -> Option<Vec<usize>> {
    'attempt: for wanted in (1..=count).rev() {
        let mut pos = start;
        let mut lengths = Vec::with_capacity(wanted);
        for _ in 0..wanted {
            let group_start = pos;
            while pos < pats.len() && pats[pos] == Pat::Dash {
                pos += 1;
            }
            if pos < pats.len() && pats[pos] == Pat::A {
                pos += 1;
            } else {
                continue 'attempt;
            }
            while pos < pats.len() && pats[pos] == Pat::Dash {
                pos += 1;
            }
            lengths.push(pos - group_start);
        }
        return Some(lengths);
    }
    None
}

/// The value a positional group carries. The reader strips
/// the first double dash word of the group, so a group reduced
/// to nothing leaves the positional standing empty, a standing
/// the doors refuse as a usage refusal after the parse.
fn positional_value(group: &[Word]) -> Option<Word> {
    let mut held: Vec<Word> = group.to_vec();
    if let Some(index) = held.iter().position(|word| word.bytes == b"--") {
        held.remove(index);
    }
    held.into_iter().next()
}

struct DoorParse {
    positionals: Vec<Option<Word>>,
    flags: Vec<(FlagId, Word)>,
    seen: Vec<FlagId>,
    extras: Vec<Word>,
}

enum DoorOutcome {
    Parsed(DoorParse),
    Help,
    Usage,
}

/// Consume the positionals standing at a start index, as the
/// reader's consume step does, and return the index it stops
/// at.
fn consume_positionals(
    words: &[Word],
    pats: &[Pat],
    spec: &Spec,
    parse: &mut DoorParse,
    start: usize,
) -> usize {
    let remaining = spec.positional_count - parse.positionals.len();
    if remaining == 0 {
        return start;
    }
    match match_groups(pats, start, remaining) {
        Some(lengths) => {
            let mut pos = start;
            for length in lengths {
                parse
                    .positionals
                    .push(positional_value(&words[pos..pos + length]));
                pos += length;
            }
            pos
        }
        None => start,
    }
}

/// Parse one door's argument words under its own parser, the
/// reader's standings carried step for step.
fn parse_door(words: &[Word], spec: &Spec) -> DoorOutcome {
    let (pats, classes) = match pattern_pass(words, spec) {
        Ok(standing) => standing,
        Err(()) => return DoorOutcome::Usage,
    };
    let option_indices: Vec<usize> = pats
        .iter()
        .enumerate()
        .filter(|(_, pat)| **pat == Pat::O)
        .map(|(index, _)| index)
        .collect();
    let max_option = option_indices
        .last()
        .map(|index| *index as i64)
        .unwrap_or(-1);
    let mut parse = DoorParse {
        positionals: Vec::new(),
        flags: Vec::new(),
        seen: Vec::new(),
        extras: Vec::new(),
    };
    let mut start = 0usize;
    while (start as i64) <= max_option {
        let next_option = option_indices
            .iter()
            .copied()
            .filter(|index| *index >= start)
            .min()
            .expect("an option index stands at or after the start");
        if start != next_option {
            let end = consume_positionals(words, &pats, spec, &mut parse, start);
            if end > start {
                start = end;
                continue;
            }
        }
        if !option_indices.contains(&start) {
            parse.extras.extend_from_slice(&words[start..next_option]);
            start = next_option;
        }
        match &classes[start] {
            Some(Class::Unknown) => {
                parse.extras.push(words[start].clone());
                start += 1;
            }
            Some(Class::HelpPlain) | Some(Class::HelpCluster) => return DoorOutcome::Help,
            Some(Class::HelpValued) | Some(Class::HelpClusterValued) => return DoorOutcome::Usage,
            Some(Class::ValueFlag(flag, explicit)) => {
                let flag = *flag;
                if !parse.seen.contains(&flag) {
                    parse.seen.push(flag);
                }
                match explicit {
                    Some(bytes) => {
                        parse.flags.push((flag, Word::from_bytes(bytes)));
                        start += 1;
                    }
                    None => {
                        if start + 1 < words.len() && pats[start + 1] == Pat::A {
                            parse.flags.push((flag, words[start + 1].clone()));
                            start += 2;
                        } else {
                            return DoorOutcome::Usage;
                        }
                    }
                }
            }
            _ => unreachable!("an option index carries its class"),
        }
    }
    let stop = consume_positionals(words, &pats, spec, &mut parse, start);
    parse.extras.extend_from_slice(&words[stop..]);
    if parse.positionals.len() < spec.positional_count {
        return DoorOutcome::Usage;
    }
    for flag in spec.required {
        if !parse.seen.contains(flag) {
            return DoorOutcome::Usage;
        }
    }
    DoorOutcome::Parsed(parse)
}

#[derive(Clone, Copy)]
enum Door {
    Plan,
    Restore,
}

impl Door {
    fn spec(&self) -> &'static Spec {
        match self {
            Door::Plan => &PLAN_SPEC,
            Door::Restore => &RESTORE_SPEC,
        }
    }

    fn usage(&self) -> &'static str {
        match self {
            Door::Plan => USAGE_PLAN,
            Door::Restore => USAGE_RESTORE,
        }
    }
}

enum CommandStep {
    /// No command group matches at the start index.
    NoMatch,
    /// A command group matches and its first word names no
    /// door, the reader's invalid choice standing.
    Invalid,
    /// The door word was taken and its parser answered.
    Found(Door, DoorOutcome),
}

/// Read the command group standing at a start index, the
/// reader's subcommand pattern, a run of separator words and
/// one positional word and then every word that follows.
fn try_command(words: &[Word], pats: &[Pat], start: usize) -> CommandStep {
    let mut pos = start;
    while pos < pats.len() && pats[pos] == Pat::Dash {
        pos += 1;
    }
    if pos >= pats.len() || pats[pos] != Pat::A {
        return CommandStep::NoMatch;
    }
    let first = &words[start];
    let door = if first.is_text("plan") {
        Door::Plan
    } else if first.is_text("restore") {
        Door::Restore
    } else {
        return CommandStep::Invalid;
    };
    let outcome = parse_door(&words[start + 1..], door.spec());
    CommandStep::Found(door, outcome)
}

fn last_flag<'a>(parse: &'a DoorParse, flag: FlagId) -> &'a Word {
    parse
        .flags
        .iter()
        .rev()
        .find(|(standing, _)| *standing == flag)
        .map(|(_, word)| word)
        .expect("a required flag stands after the parse")
}

/// Read the budget argument. A word carrying a byte that is
/// not UTF-8 text can never stand as a whole number, and its
/// refusal is spelled from the word's own code points, so the
/// surrogate escapes of its bytes stand in the wording exactly
/// as the probe recorded them, never a replacement character.
fn budget_value(word: &Word) -> Result<BigInt, ExitCode> {
    if word.carries_surrogate() {
        return Err(fail(&format!(
            "the budget {} is not a whole number of milliseconds",
            repr_units(&word.units)
        )));
    }
    let text = String::from_utf8(word.bytes.clone())
        .expect("a word with no surrogate units is UTF-8 text");
    match parse_budget(&text) {
        Ok(budget) => Ok(budget),
        Err(error) => Err(fail(error.message())),
    }
}

fn write_output(path: &PathBuf, display: &str, text: &str) -> Result<(), ExitCode> {
    match std::fs::write(path, text) {
        Ok(()) => Ok(()),
        Err(_) => Err(fail(&format!("the output at {display} cannot be written"))),
    }
}

fn plan_door(parse: DoorParse) -> ExitCode {
    let records_word = parse.positionals[0]
        .as_ref()
        .expect("the records positional stands after the parse");
    let budget_word = last_flag(&parse, FlagId::BudgetMs);
    let out_dir_word = last_flag(&parse, FlagId::OutDir);
    let record_set = match read_records(&PathBuf::from(&records_word.raw)) {
        Ok(record_set) => record_set,
        Err(error) => return fail(error.message()),
    };
    let budget = match budget_value(budget_word) {
        Ok(budget) => budget,
        Err(code) => return code,
    };
    let plan = match build_plan(&record_set, &budget) {
        Ok(plan) => plan,
        Err(error) => return fail(error.message()),
    };
    let quarantine = build_quarantine(&record_set, &plan);
    let remaining = remaining_records(&record_set, &plan);
    let report = report_document(&plan);
    let outputs: [(&str, String); 4] = [
        ("report.json", canonical_text(&report)),
        ("report.md", markdown_text(&report)),
        (
            "quarantine.json",
            canonical_text(&quarantine_document(&quarantine)),
        ),
        (
            "remaining.json",
            canonical_text(&records_document(&remaining)),
        ),
    ];
    let out_dir_path = PathBuf::from(&out_dir_word.raw);
    let out_dir_display = bytes_text(&out_dir_word.bytes);
    if std::fs::create_dir_all(&out_dir_path).is_err() {
        return fail(&format!(
            "the output at {out_dir_display} cannot be written"
        ));
    }
    let trimmed = out_dir_display.trim_end_matches('/');
    for (name, text) in &outputs {
        let path = out_dir_path.join(name);
        let display = format!("{trimmed}/{name}");
        if let Err(code) = write_output(&path, &display, text) {
            return code;
        }
    }
    ExitCode::SUCCESS
}

fn restore_door(parse: DoorParse) -> ExitCode {
    let remaining_word = parse.positionals[0]
        .as_ref()
        .expect("the remaining positional stands after the parse");
    let quarantine_word = parse.positionals[1]
        .as_ref()
        .expect("the quarantine positional stands after the parse");
    let out_word = last_flag(&parse, FlagId::Out);
    let remaining = match read_remaining(&PathBuf::from(&remaining_word.raw)) {
        Ok(remaining) => remaining,
        Err(error) => return fail(error.message()),
    };
    let quarantine = match read_quarantine(&PathBuf::from(&quarantine_word.raw)) {
        Ok(quarantine) => quarantine,
        Err(error) => return fail(error.message()),
    };
    let restored = match restore_records(&remaining, &quarantine) {
        Ok(restored) => restored,
        Err(error) => return fail(error.message()),
    };
    let text = canonical_text(&records_document(&restored));
    let out_path = PathBuf::from(&out_word.raw);
    let display = bytes_text(&out_word.bytes);
    match write_output(&out_path, &display, &text) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => code,
    }
}

fn run(parse: DoorParse, door: Door) -> ExitCode {
    match door {
        Door::Plan => plan_door(parse),
        Door::Restore => restore_door(parse),
    }
}

fn main() -> ExitCode {
    let raw_args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if raw_args.is_empty() {
        return usage_exit(USAGE_TOP);
    }
    let words: Vec<Word> = raw_args.iter().map(Word::new).collect();
    let (pats, classes) = match pattern_pass(&words, &TOP_SPEC) {
        Ok(standing) => standing,
        Err(()) => return usage_exit(USAGE_TOP),
    };
    let option_indices: Vec<usize> = pats
        .iter()
        .enumerate()
        .filter(|(_, pat)| **pat == Pat::O)
        .map(|(index, _)| index)
        .collect();
    let max_option = option_indices
        .last()
        .map(|index| *index as i64)
        .unwrap_or(-1);
    let mut extras_top: Vec<Word> = Vec::new();
    let mut found: Option<(Door, DoorOutcome)> = None;
    let mut start = 0usize;
    while (start as i64) <= max_option {
        let next_option = option_indices
            .iter()
            .copied()
            .filter(|index| *index >= start)
            .min()
            .expect("an option index stands at or after the start");
        if start != next_option {
            match try_command(&words, &pats, start) {
                CommandStep::Found(door, outcome) => {
                    found = Some((door, outcome));
                    break;
                }
                CommandStep::Invalid => return usage_exit(USAGE_TOP),
                CommandStep::NoMatch => {}
            }
        }
        if !option_indices.contains(&start) {
            extras_top.extend_from_slice(&words[start..next_option]);
            start = next_option;
        }
        match &classes[start] {
            Some(Class::Unknown) => {
                extras_top.push(words[start].clone());
                start += 1;
            }
            Some(Class::HelpPlain) | Some(Class::HelpCluster) => {
                return help_exit(USAGE_TOP);
            }
            Some(Class::HelpValued) | Some(Class::HelpClusterValued) => {
                return usage_exit(USAGE_TOP);
            }
            _ => unreachable!("the top parser carries no value flags"),
        }
    }
    if found.is_none() {
        match try_command(&words, &pats, start) {
            CommandStep::Found(door, outcome) => {
                found = Some((door, outcome));
            }
            CommandStep::Invalid => return usage_exit(USAGE_TOP),
            CommandStep::NoMatch => {}
        }
    }
    let Some((door, outcome)) = found else {
        return usage_exit(USAGE_TOP);
    };
    match outcome {
        DoorOutcome::Help => help_exit(door.usage()),
        DoorOutcome::Usage => usage_exit(door.usage()),
        DoorOutcome::Parsed(parse) => {
            if !extras_top.is_empty() || !parse.extras.is_empty() {
                return usage_exit(USAGE_TOP);
            }
            if parse.positionals.iter().any(|held| held.is_none()) {
                return usage_exit(door.usage());
            }
            run(parse, door)
        }
    }
}
