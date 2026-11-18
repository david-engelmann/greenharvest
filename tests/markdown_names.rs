//! The pins of slice 12, the markdown rendering of
//! names, over synthetic records the suite writes
//! itself. Each pin plans a record set carrying the
//! name it names and asserts the markdown the plan
//! writes, the ranking row keeping the table's columns
//! and every sentence and bullet standing on one line.
//! Six pins fail on the head this slice branches from,
//! where the rendering writes every name exactly as it
//! stands, and three stand on both heads at the lines
//! MN-3 and MN-4 draw.

use greenharvest::{
    build_plan, canonical_text, markdown_text, parse_budget, read_records_text, report_document,
    validate_markdown,
};

fn rendered(records: &str) -> String {
    let set = read_records_text(records).expect("the records read");
    let plan = build_plan(&set, &parse_budget("100000").expect("the budget parses"))
        .expect("the plan builds");
    markdown_text(&report_document(&plan))
}

fn records_with_name(name_json: &str) -> String {
    format!(
        "{{\"format\": \"suitediet-records/1\", \"suite\": \"probe\", \"tests\": [
          {{\"name\": \"alpha\", \"runtime_ms\": 100, \"caught\": [\"d1\"]}},
          {{\"name\": {}, \"runtime_ms\": 50, \"caught\": [\"d1\"]}}
        ]}}",
        name_json
    )
}

fn line_count(text: &str) -> usize {
    text.lines().count()
}

#[test]
fn test_a_pipe_in_a_ranking_name_stands_escaped_in_its_cell() {
    let md = rendered(&records_with_name("\"test_a|b\""));
    let row = md
        .lines()
        .find(|line| line.contains("test_a"))
        .expect("the row stands");
    assert!(
        row.contains("| test_a\\|b |"),
        "the cell carries the escaped pipe: {row}"
    );
    assert_eq!(
        row.matches('|').count(),
        9,
        "the row's pipes stand at the table's eight plus the escaped one: {row}"
    );
}

#[test]
fn test_a_line_break_in_a_ranking_name_stands_as_a_space() {
    let plain = rendered(&records_with_name("\"test_ab\""));
    let md = rendered(&records_with_name("\"test_a\\nb\""));
    assert_eq!(line_count(&md), line_count(&plain), "no line is added");
    let row = md
        .lines()
        .find(|line| line.contains("test_a b"))
        .expect("the row stands on one line");
    assert!(row.starts_with("| "), "the row stands whole: {row}");
}

#[test]
fn test_a_pipe_in_a_suite_name_stands_as_itself_in_the_prose() {
    let records = "{\"format\": \"suitediet-records/1\", \"suite\": \"suite|x\", \"tests\": [{\"name\": \"alpha\", \"runtime_ms\": 100, \"caught\": [\"d1\"]}]}";
    let md = rendered(records);
    assert!(
        md.contains("The suite suite|x under a budget"),
        "the prose carries the pipe as it stands: {md}"
    );
}

#[test]
fn test_a_line_break_in_a_suite_name_stands_as_a_space() {
    let records = "{\"format\": \"suitediet-records/1\", \"suite\": \"suite\\nx\", \"tests\": [{\"name\": \"alpha\", \"runtime_ms\": 100, \"caught\": [\"d1\"]}]}";
    let md = rendered(records);
    assert!(
        md.contains("The suite suite x under a budget"),
        "the sentence stands on one line: {md}"
    );
}

#[test]
fn test_a_line_break_in_a_pack_name_stands_as_a_space() {
    // The budget cuts the second test, which catches
    // nothing, so a removal pack names it.
    let records = "{\"format\": \"suitediet-records/1\", \"suite\": \"probe\", \"tests\": [{\"name\": \"alpha\", \"runtime_ms\": 100, \"caught\": [\"d1\"]}, {\"name\": \"idle\\none\", \"runtime_ms\": 50, \"caught\": []}]}";
    let plain = "{\"format\": \"suitediet-records/1\", \"suite\": \"probe\", \"tests\": [{\"name\": \"alpha\", \"runtime_ms\": 100, \"caught\": [\"d1\"]}, {\"name\": \"idle one\", \"runtime_ms\": 50, \"caught\": []}]}";
    let render_at = |text: &str, budget: &str| {
        let set = read_records_text(text).expect("the records read");
        let plan = build_plan(&set, &parse_budget(budget).expect("the budget parses"))
            .expect("the plan builds");
        markdown_text(&report_document(&plan))
    };
    let md = render_at(records, "100");
    assert_eq!(
        line_count(&md),
        line_count(&render_at(plain, "100")),
        "no line is added"
    );
    assert!(
        md.contains("- idle one is proposed for quarantine."),
        "the bullet stands on one line: {md}"
    );
}

#[test]
fn test_a_line_break_in_a_defect_name_stands_as_a_space() {
    // The budget cuts the slower test, whose only defect
    // the standing test also catches, so the pack's
    // overlap sentence names the defect.
    let records = "{\"format\": \"suitediet-records/1\", \"suite\": \"probe\", \"tests\": [{\"name\": \"alpha\", \"runtime_ms\": 100, \"caught\": [\"d\\n1\"]}, {\"name\": \"beta\", \"runtime_ms\": 900, \"caught\": [\"d\\n1\"]}]}";
    let set = read_records_text(records).expect("the records read");
    let plan = build_plan(&set, &parse_budget("100").expect("the budget parses"))
        .expect("the plan builds");
    let md = markdown_text(&report_document(&plan));
    assert!(
        md.contains("d 1 is also caught by alpha"),
        "the overlap sentence stands on one line: {md}"
    );
}

#[test]
fn test_the_report_document_keeps_a_piped_name_exactly() {
    let set = read_records_text(&records_with_name("\"test_a|b\"")).expect("the records read");
    let plan = build_plan(&set, &parse_budget("100000").expect("the budget parses"))
        .expect("the plan builds");
    let report = report_document(&plan);
    let text = canonical_text(&report);
    assert!(
        text.contains("\"test_a|b\""),
        "the report document carries the name untouched: {text}"
    );
}

#[test]
fn test_the_markdown_of_a_piped_name_validates_against_its_report() {
    let set = read_records_text(&records_with_name("\"test_a|b\"")).expect("the records read");
    let plan = build_plan(&set, &parse_budget("100000").expect("the budget parses"))
        .expect("the plan builds");
    let report = report_document(&plan);
    let md = markdown_text(&report);
    validate_markdown(&report, &md).expect("the written markdown validates");
}

#[test]
fn test_a_markdown_carrying_the_raw_pipe_refuses() {
    let set = read_records_text(&records_with_name("\"test_a|b\"")).expect("the records read");
    let plan = build_plan(&set, &parse_budget("100000").expect("the budget parses"))
        .expect("the plan builds");
    let report = report_document(&plan);
    let raw = markdown_text(&report).replace("test_a\\|b", "test_a|b");
    let message = validate_markdown(&report, &raw)
        .expect_err("the raw form refuses")
        .message()
        .to_string();
    assert_eq!(
        message,
        "the markdown disagrees with the report it was rendered from"
    );
}
