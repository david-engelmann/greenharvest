//! The diet report, as a JSON document and as a markdown
//! rendering.
//!
//! Every number in the report is read from the plan it reports.
//! The totals are computed from the defect sets themselves, never
//! copied from one another, so the after total can never claim a
//! catch the plan loses.

use std::path::Path;

use crate::bigint::BigInt;
use crate::errors::GreenharvestError;
use crate::ingest::{
    bigint_json, read_file_text, record_document, translate_line_endings, RecordSet,
};
use crate::jsonvalue::{parse_json, JsonValue, ParseFailure};
use crate::plan::{build_plan, Plan, DISPOSITION_BLOCKED, DISPOSITION_QUARANTINE};
use crate::quarantine::remaining_records;
use crate::ranking::render_value;
use crate::render::{canonical_text, repr_string, repr_value};

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

/// The fields of the report document, in the sorted order the
/// exactness refusal spells.
const REPORT_FIELDS: [&str; 11] = [
    "below_the_line",
    "blocked",
    "budget_ms",
    "format",
    "quarantined",
    "ranking",
    "removal_packs",
    "suite",
    "survivor_runtime_ms",
    "survivors",
    "totals",
];

/// The fields of one ranking entry of the report.
const RANKING_FIELDS: [&str; 7] = [
    "catches",
    "name",
    "rank",
    "runtime_ms",
    "unique_catches",
    "value",
    "weighted_catches",
];

/// The fields of one removal pack of the report.
const PACK_FIELDS: [&str; 9] = [
    "blocking_defects",
    "disposition",
    "name",
    "overlap",
    "rank",
    "record",
    "runtime_ms",
    "value",
    "weighted_catches",
];

/// The fields of the test record a removal pack carries.
const PACK_RECORD_FIELDS: [&str; 3] = ["caught", "name", "runtime_ms"];

/// The fields of the totals member of the report.
const TOTALS_FIELDS: [&str; 5] = [
    "defects_after",
    "defects_after_list",
    "defects_before",
    "defects_before_list",
    "lost_defects",
];

fn read_json(text: &str, what: &str) -> Result<JsonValue, GreenharvestError> {
    parse_json(text).map_err(|failure| match failure {
        ParseFailure::Invalid(message) => {
            GreenharvestError::new(format!("the {what} is not valid JSON: {message}"))
        }
        ParseFailure::Boundary => GreenharvestError::new(format!("the {what} is not valid JSON")),
        ParseFailure::DuplicateKey(key) => GreenharvestError::new(format!(
            "a JSON object carries the key {} twice",
            repr_string(&key)
        )),
    })
}

/// The list of a document's field names as the refusal spells
/// it, the sorted names in a list representation.
fn fields_spelling(keys: &[&str]) -> String {
    let mut sorted: Vec<&str> = keys.to_vec();
    sorted.sort();
    let items: Vec<JsonValue> = sorted
        .iter()
        .map(|key| JsonValue::Text((*key).to_string()))
        .collect();
    repr_value(&JsonValue::List(items))
}

fn expected_fields_spelling(fields: &[&str]) -> String {
    let items: Vec<JsonValue> = fields
        .iter()
        .map(|key| JsonValue::Text((*key).to_string()))
        .collect();
    repr_value(&JsonValue::List(items))
}

/// Refuse unless an object's fields are exactly the named set,
/// in the standing exactness wording of the records reading.
fn exact_fields(
    value: &JsonValue,
    expected: &[&str],
    wording: &str,
) -> Result<(), GreenharvestError> {
    let fields = match value.as_object() {
        Some(fields) => fields,
        None => {
            return Err(GreenharvestError::new(format!(
                "{wording} is not an object"
            )));
        }
    };
    let keys: Vec<&str> = fields.iter().map(|(key, _)| key.as_str()).collect();
    let mut sorted_keys = keys.clone();
    sorted_keys.sort();
    if sorted_keys.as_slice() != expected {
        return Err(GreenharvestError::new(format!(
            "{wording} carries fields {} rather than {}",
            fields_spelling(&keys),
            expected_fields_spelling(expected),
        )));
    }
    Ok(())
}

/// The budget the report carries, under the standing budget
/// rule, a positive whole number of milliseconds.
fn report_budget(report: &JsonValue) -> Result<BigInt, GreenharvestError> {
    let budget = match report.get("budget_ms") {
        Some(JsonValue::Int(value)) => BigInt::from_i64(*value),
        Some(JsonValue::BigInt(text)) => {
            BigInt::from_decimal(text).expect("a stored integer token reads back")
        }
        _ => {
            return Err(GreenharvestError::new(
                "the report carries a budget that is not a positive whole number of milliseconds",
            ));
        }
    };
    if !budget.is_positive() {
        return Err(GreenharvestError::new(
            "the report carries a budget that is not a positive whole number of milliseconds",
        ));
    }
    Ok(budget)
}

/// Read a report document from a local file.
pub fn read_report(path: &Path) -> Result<JsonValue, GreenharvestError> {
    let text = read_file_text(path, "report")?;
    read_report_text(&text)
}

/// Read a report document under the exact reading of VR-2.
/// The document stands as read, its fields exactly the fields
/// the builder writes at every level it carries.
pub fn read_report_text(text: &str) -> Result<JsonValue, GreenharvestError> {
    let document = read_json(text, "report")?;
    exact_fields(&document, &REPORT_FIELDS, "the report")?;
    let format_value = document.get("format").expect("the field stands");
    let format_matches = matches!(
        format_value,
        JsonValue::Text(text) if text == REPORT_FORMAT
    );
    if !format_matches {
        return Err(GreenharvestError::new(format!(
            "the report carries the format name {} rather than {}",
            repr_value(format_value),
            repr_string(REPORT_FORMAT),
        )));
    }
    match document.get("ranking") {
        Some(JsonValue::List(entries)) => {
            for entry in entries {
                exact_fields(entry, &RANKING_FIELDS, "a ranking entry in the report")?;
            }
        }
        _ => {
            return Err(GreenharvestError::new(
                "the report carries a ranking field that is not a list",
            ));
        }
    }
    match document.get("removal_packs") {
        Some(JsonValue::List(packs)) => {
            for pack in packs {
                exact_fields(pack, &PACK_FIELDS, "a removal pack in the report")?;
                let record = pack.get("record").expect("the field stands");
                exact_fields(
                    record,
                    &PACK_RECORD_FIELDS,
                    "a test record in a removal pack in the report",
                )?;
            }
        }
        _ => {
            return Err(GreenharvestError::new(
                "the report carries a removal packs field that is not a list",
            ));
        }
    }
    let totals = document.get("totals").expect("the field stands");
    exact_fields(totals, &TOTALS_FIELDS, "the totals in the report")?;
    report_budget(&document)?;
    Ok(document)
}

/// Validate a report against the record set it was planned
/// from (VR-3). The plan is rebuilt from the records under
/// the budget the report carries, by the same building the
/// plan door performs, and the report of that plan must
/// equal the report as read in every field, the comparison
/// standing over the documents as values in their canonical
/// written form.
pub fn validate_report(
    record_set: &RecordSet,
    report: &JsonValue,
) -> Result<(), GreenharvestError> {
    let budget = report_budget(report)?;
    let plan = build_plan(record_set, &budget)?;
    let rebuilt = report_document(&plan);
    if canonical_text(&rebuilt) != canonical_text(report) {
        return Err(GreenharvestError::new(
            "the report disagrees with the records it was planned from",
        ));
    }
    Ok(())
}

/// Read a markdown rendering from a local file, as the text
/// the standing file reading reads it, its line endings
/// translated as that reading translates them.
pub fn read_markdown(path: &Path) -> Result<String, GreenharvestError> {
    read_file_text(path, "markdown")
}

/// Whether a value stands as an integer in either of its
/// stored forms, the kind the rendering reads a count in.
fn stands_as_int(value: Option<&JsonValue>) -> bool {
    matches!(value, Some(JsonValue::Int(_)) | Some(JsonValue::BigInt(_)))
}

/// Whether a value stands as a text, the kind the rendering
/// reads a name or a rendered figure in.
fn stands_as_text(value: Option<&JsonValue>) -> bool {
    matches!(value, Some(JsonValue::Text(_)))
}

/// Whether a value stands as a list of texts, the kind the
/// rendering reads a defect list in.
fn stands_as_texts(value: Option<&JsonValue>) -> bool {
    match value {
        Some(JsonValue::List(items)) => items.iter().all(|item| matches!(item, JsonValue::Text(_))),
        _ => false,
    }
}

/// Whether the report as read carries every field in the
/// kind the markdown rendering reads it in (MD-2). The
/// report reading settles which fields stand, and this
/// settling settles the kinds they stand in, so the
/// rendering the validation compares against is never built
/// from a report it could not have been made from.
fn markdown_kinds_stand(report: &JsonValue) -> bool {
    if !stands_as_text(report.get("suite"))
        || !stands_as_int(report.get("budget_ms"))
        || !stands_as_int(report.get("survivor_runtime_ms"))
    {
        return false;
    }
    let ranking = match report.get("ranking") {
        Some(JsonValue::List(rows)) => rows,
        _ => return false,
    };
    for row in ranking {
        if !stands_as_int(row.get("rank"))
            || !stands_as_text(row.get("name"))
            || !stands_as_int(row.get("runtime_ms"))
            || !stands_as_text(row.get("weighted_catches"))
            || !stands_as_text(row.get("value"))
            || !stands_as_int(row.get("catches"))
            || !stands_as_int(row.get("unique_catches"))
        {
            return false;
        }
    }
    let packs = match report.get("removal_packs") {
        Some(JsonValue::List(packs)) => packs,
        _ => return false,
    };
    for pack in packs {
        if !stands_as_text(pack.get("name"))
            || !stands_as_text(pack.get("disposition"))
            || !stands_as_texts(pack.get("blocking_defects"))
        {
            return false;
        }
        match pack.get("overlap") {
            Some(JsonValue::Object(entries)) => {
                for (_, names) in entries {
                    if !stands_as_texts(Some(names)) {
                        return false;
                    }
                }
            }
            _ => return false,
        }
    }
    let totals = match report.get("totals") {
        Some(totals) => totals,
        None => return false,
    };
    stands_as_int(totals.get("defects_before"))
        && stands_as_int(totals.get("defects_after"))
        && stands_as_texts(totals.get("lost_defects"))
}

/// Validate a markdown rendering against the report it was
/// rendered from (MD-3). The expected rendering is built
/// from the report as read by the same rendering the plan
/// door writes, and the markdown as read must equal it
/// whole under the translation the standing text reading
/// applies, every other character standing in the
/// comparison, because a rendering carries one canonical
/// spelling. A
/// markdown any character of which differs refuses in the
/// disagreement words, and a report one of whose fields
/// stands in a kind the rendering does not read refuses
/// the same way, the rendering never standing to be built
/// from it.
pub fn validate_markdown(report: &JsonValue, markdown: &str) -> Result<(), GreenharvestError> {
    if !markdown_kinds_stand(report) || translate_line_endings(&markdown_text(report)) != markdown {
        return Err(GreenharvestError::new(
            "the markdown disagrees with the report it was rendered from",
        ));
    }
    Ok(())
}

/// Validate a remaining record set against the records and
/// the report of the plan it was planned from (VM-2 and
/// VM-3). The plan is rebuilt from the records under the
/// budget the report carries, by the same building the plan
/// door performs, and the remaining record set of that plan,
/// built by the standing remaining computation, must equal
/// the remaining set as read in every field, the suite and
/// the tests in their order and every record the tests
/// carry. A remaining set any field of which differs refuses
/// in the disagreement words.
pub fn validate_remaining(
    record_set: &RecordSet,
    report: &JsonValue,
    remaining: &RecordSet,
) -> Result<(), GreenharvestError> {
    let budget = report_budget(report)?;
    let plan = build_plan(record_set, &budget)?;
    let expected = remaining_records(record_set, &plan);
    if expected != *remaining {
        return Err(GreenharvestError::new(
            "the remaining record set disagrees with the records and the report it was planned from",
        ));
    }
    Ok(())
}
