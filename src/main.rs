//! The thin suitediet binary, carrying two doors over the ported
//! core. The plan door reads a records document and a budget
//! argument and writes the diet report as JSON and as markdown
//! with the quarantine record and the record set that remains.
//! The restore door reads the remaining record set and a
//! quarantine record and writes the suite record set as it stood.
//! The Python command line remains the full command line of the
//! tool in this slice, as the contract states.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use suitediet::ingest::{
    quarantine_document, read_quarantine, read_records, read_remaining, records_document,
};
use suitediet::pytext::bytes_text;
use suitediet::{
    build_plan, build_quarantine, canonical_text, markdown_text, parse_budget, remaining_records,
    report_document, restore_records,
};

fn print_line(text: &str) {
    let _ = std::io::stderr().write_all(text.as_bytes());
    let _ = std::io::stderr().write_all(b"\n");
}

fn fail(message: &str) -> ExitCode {
    print_line(&format!("error: {message}"));
    ExitCode::from(2)
}

fn usage() -> ExitCode {
    print_line("usage: suitediet plan RECORDS --budget-ms BUDGET --out-dir OUT_DIR");
    print_line("       suitediet restore REMAINING QUARANTINE --out OUT");
    ExitCode::from(2)
}

/// An argument as text, its bytes decoded as UTF-8 with the
/// replacement character standing for undecodable bytes, as the
/// contract states among the limits of the port.
fn arg_text(arg: &OsString) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        bytes_text(arg.as_os_str().as_bytes())
    }
    #[cfg(not(unix))]
    {
        arg.to_string_lossy().into_owned()
    }
}

struct Flags {
    positionals: Vec<OsString>,
    budget_ms: Option<String>,
    out_dir: Option<OsString>,
    out: Option<OsString>,
}

fn parse_flags(args: &[OsString]) -> Option<Flags> {
    let mut flags = Flags {
        positionals: Vec::new(),
        budget_ms: None,
        out_dir: None,
        out: None,
    };
    let mut index = 0;
    while index < args.len() {
        let token = arg_text(&args[index]);
        match token.as_str() {
            "--budget-ms" => {
                index += 1;
                flags.budget_ms = Some(arg_text(args.get(index)?));
            }
            "--out-dir" => {
                index += 1;
                flags.out_dir = Some(args.get(index)?.clone());
            }
            "--out" => {
                index += 1;
                flags.out = Some(args.get(index)?.clone());
            }
            _ => {
                if token.starts_with("--") {
                    return None;
                }
                flags.positionals.push(args[index].clone());
            }
        }
        index += 1;
    }
    Some(flags)
}

fn write_output(path: &Path, display: &str, text: &str) -> Result<(), ExitCode> {
    match std::fs::write(path, text) {
        Ok(()) => Ok(()),
        Err(_) => Err(fail(&format!("the output at {display} cannot be written"))),
    }
}

fn plan_door(flags: Flags) -> ExitCode {
    if flags.positionals.len() != 1 {
        return usage();
    }
    let (budget_raw, out_dir) = match (flags.budget_ms, flags.out_dir) {
        (Some(budget), Some(out_dir)) => (budget, out_dir),
        _ => return usage(),
    };
    let records_path = PathBuf::from(&flags.positionals[0]);
    let record_set = match read_records(&records_path) {
        Ok(record_set) => record_set,
        Err(error) => return fail(error.message()),
    };
    let budget = match parse_budget(&budget_raw) {
        Ok(budget) => budget,
        Err(error) => return fail(error.message()),
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
    let out_dir_path = PathBuf::from(&out_dir);
    let out_dir_display = arg_text(&out_dir);
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

fn restore_door(flags: Flags) -> ExitCode {
    if flags.positionals.len() != 2 {
        return usage();
    }
    let out = match flags.out {
        Some(out) => out,
        None => return usage(),
    };
    let remaining = match read_remaining(&PathBuf::from(&flags.positionals[0])) {
        Ok(remaining) => remaining,
        Err(error) => return fail(error.message()),
    };
    let quarantine = match read_quarantine(&PathBuf::from(&flags.positionals[1])) {
        Ok(quarantine) => quarantine,
        Err(error) => return fail(error.message()),
    };
    let restored = match restore_records(&remaining, &quarantine) {
        Ok(restored) => restored,
        Err(error) => return fail(error.message()),
    };
    let text = canonical_text(&records_document(&restored));
    let out_path = PathBuf::from(&out);
    let display = arg_text(&out);
    match write_output(&out_path, &display, &text) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => code,
    }
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.is_empty() {
        return usage();
    }
    let command = arg_text(&args[0]);
    let flags = match parse_flags(&args[1..]) {
        Some(flags) => flags,
        None => return usage(),
    };
    match command.as_str() {
        "plan" => plan_door(flags),
        "restore" => restore_door(flags),
        _ => usage(),
    }
}
