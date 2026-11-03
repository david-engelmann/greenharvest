//! The diet report, as a JSON document and as a markdown
//! rendering.
//!
//! Every number in the report is read from the plan it reports.
//! The totals are computed from the defect sets themselves, never
//! copied from one another, so the after total can never claim a
//! catch the plan loses.

use crate::ingest::{bigint_json, record_document};
use crate::jsonvalue::JsonValue;
use crate::plan::{Plan, DISPOSITION_BLOCKED, DISPOSITION_QUARANTINE};
use crate::ranking::render_value;

pub const REPORT_FORMAT: &str = "suitediet-report/1";

fn text(value: &str) -> JsonValue {
    JsonValue::Text(value.to_string())
}

fn texts(values: &[String]) -> JsonValue {
    JsonValue::List(values.iter().map(|value| text(value)).collect())
}

fn count(value: usize) -> JsonValue {
    JsonValue::Int(value as i64)
}

/// The report of a plan as a document value.
pub fn report_document(plan: &Plan) -> JsonValue {
    let ranking_rows: Vec<JsonValue> = plan
        .ranking
        .iter()
        .enumerate()
        .map(|(position, entry)| {
            JsonValue::object(vec![
                ("rank", count(position + 1)),
                ("name", text(entry.name())),
                ("runtime_ms", bigint_json(&entry.record.runtime_ms)),
                ("weighted_catches", text(&render_value(&entry.weighted))),
                ("value", text(&render_value(&entry.value))),
                ("catches", count(entry.record.caught.len())),
                ("unique_catches", count(entry.unique_catches.len())),
            ])
        })
        .collect();
    let pack_rows: Vec<JsonValue> = plan
        .packs
        .iter()
        .map(|pack| {
            let overlap = JsonValue::Object(
                pack.overlap
                    .iter()
                    .map(|(defect, names)| (defect.clone(), texts(names)))
                    .collect(),
            );
            JsonValue::object(vec![
                ("name", text(pack.ranked.name())),
                ("disposition", text(&pack.disposition)),
                ("rank", count(pack.rank)),
                ("runtime_ms", bigint_json(&pack.ranked.record.runtime_ms)),
                (
                    "weighted_catches",
                    text(&render_value(&pack.ranked.weighted)),
                ),
                ("value", text(&render_value(&pack.ranked.value))),
                ("record", record_document(&pack.ranked.record)),
                ("overlap", overlap),
                ("blocking_defects", texts(&pack.blocking_defects)),
            ])
        })
        .collect();
    let quarantined: Vec<String> = {
        let mut names: Vec<String> = plan
            .packs
            .iter()
            .filter(|pack| pack.disposition == DISPOSITION_QUARANTINE)
            .map(|pack| pack.ranked.name().to_string())
            .collect();
        names.sort();
        names
    };
    let blocked: Vec<String> = {
        let mut names: Vec<String> = plan
            .packs
            .iter()
            .filter(|pack| pack.disposition == DISPOSITION_BLOCKED)
            .map(|pack| pack.ranked.name().to_string())
            .collect();
        names.sort();
        names
    };
    JsonValue::object(vec![
        ("format", text(REPORT_FORMAT)),
        ("suite", text(&plan.record_set.suite)),
        ("budget_ms", bigint_json(&plan.budget_ms)),
        ("ranking", JsonValue::List(ranking_rows)),
        (
            "survivors",
            texts(
                &plan
                    .survivors
                    .iter()
                    .map(|entry| entry.name().to_string())
                    .collect::<Vec<_>>(),
            ),
        ),
        (
            "survivor_runtime_ms",
            bigint_json(&plan.survivor_runtime_ms),
        ),
        (
            "below_the_line",
            texts(
                &plan
                    .below_the_line
                    .iter()
                    .map(|entry| entry.name().to_string())
                    .collect::<Vec<_>>(),
            ),
        ),
        ("removal_packs", JsonValue::List(pack_rows)),
        (
            "totals",
            JsonValue::object(vec![
                ("defects_before", count(plan.defects_before.len())),
                ("defects_after", count(plan.defects_after.len())),
                ("defects_before_list", texts(&plan.defects_before)),
                ("defects_after_list", texts(&plan.defects_after)),
                ("lost_defects", texts(&plan.lost_defects)),
            ]),
        ),
        ("quarantined", texts(&quarantined)),
        ("blocked", texts(&blocked)),
    ])
}

/// Read a text field of a report document.
fn field_text<'a>(document: &'a JsonValue, field: &str) -> &'a str {
    document
        .get(field)
        .and_then(JsonValue::as_text)
        .expect("the report carries the field as text")
}

/// Read an integer field of a report document as its decimal
/// spelling.
fn field_int(document: &JsonValue, field: &str) -> String {
    match document.get(field) {
        Some(JsonValue::Int(value)) => value.to_string(),
        Some(JsonValue::BigInt(text)) => text.clone(),
        _ => panic!("the report carries the field as an integer"),
    }
}

/// Read a text list field of a report document.
fn field_texts<'a>(document: &'a JsonValue, field: &str) -> Vec<&'a str> {
    document
        .get(field)
        .and_then(JsonValue::as_list)
        .expect("the report carries the field as a list")
        .iter()
        .map(|item| item.as_text().expect("the entries stand as text"))
        .collect()
}

/// The markdown rendering of a report document, in the words the
/// Python rendering uses.
pub fn markdown_text(report: &JsonValue) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("# suitediet diet report".to_string());
    lines.push(String::new());
    lines.push(format!(
        "The suite {} under a budget of {} milliseconds. The survivors stand at {} milliseconds of runtime.",
        field_text(report, "suite"),
        field_int(report, "budget_ms"),
        field_int(report, "survivor_runtime_ms"),
    ));
    lines.push(String::new());
    lines.push("## Ranking".to_string());
    lines.push(String::new());
    lines.push(
        "| Rank | Test | Runtime ms | Weighted catches | Value | Catches | Unique catches |"
            .to_string(),
    );
    lines.push("| --- | --- | --- | --- | --- | --- | --- |".to_string());
    if let Some(rows) = report.get("ranking").and_then(JsonValue::as_list) {
        for row in rows {
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} | {} |",
                field_int(row, "rank"),
                field_text(row, "name"),
                field_int(row, "runtime_ms"),
                field_text(row, "weighted_catches"),
                field_text(row, "value"),
                field_int(row, "catches"),
                field_int(row, "unique_catches"),
            ));
        }
    }
    lines.push(String::new());
    lines.push("## Removal packs".to_string());
    lines.push(String::new());
    let packs = report
        .get("removal_packs")
        .and_then(JsonValue::as_list)
        .expect("the report carries its packs");
    if packs.is_empty() {
        lines.push("No test stands below the cut line, so no removal is proposed.".to_string());
    }
    for pack in packs {
        let mut line = if field_text(pack, "disposition") == DISPOSITION_BLOCKED {
            format!(
                "- {} is held in the suite. Its removal would lose {}.",
                field_text(pack, "name"),
                field_texts(pack, "blocking_defects").join(" and "),
            )
        } else {
            format!("- {} is proposed for quarantine.", field_text(pack, "name"))
        };
        let overlap = pack
            .get("overlap")
            .and_then(JsonValue::as_object)
            .expect("the pack carries its overlap");
        let mut sentences: Vec<String> = Vec::new();
        for (defect, names) in overlap {
            let names: Vec<&str> = names
                .as_list()
                .expect("the overlap names stand as a list")
                .iter()
                .map(|item| item.as_text().expect("the names stand as text"))
                .collect();
            if names.is_empty() {
                sentences.push(format!(
                    "{defect} is also caught by no other test still standing"
                ));
            } else {
                sentences.push(format!(
                    "{defect} is also caught by {}",
                    names.join(" and ")
                ));
            }
        }
        if !sentences.is_empty() {
            line = format!(
                "{line} Overlap with the tests still standing after the plan. {}.",
                sentences.join(". ")
            );
        }
        lines.push(line);
    }
    lines.push(String::new());
    lines.push("## Totals".to_string());
    lines.push(String::new());
    let totals = report.get("totals").expect("the report carries its totals");
    lines.push(format!(
        "The suite catches {} defects before the plan and {} after the proposed removals.",
        field_int(totals, "defects_before"),
        field_int(totals, "defects_after"),
    ));
    let lost = field_texts(totals, "lost_defects");
    if lost.is_empty() {
        lines.push("The plan loses no defect.".to_string());
    } else {
        lines.push(format!("The plan loses {}.", lost.join(" and ")));
    }
    lines.push(String::new());
    lines.join("\n")
}
