//! The pins of slice 13, the markdown rendering of
//! names, the markup characters, over synthetic records
//! the suite writes itself. Each pin plans a record set
//! carrying the name it names and asserts the markdown
//! the plan writes, the name reading as itself where the
//! markdown is rendered. Seven pins fail on the head
//! this slice branches from, where the rendering writes
//! an asterisk and a backtick and a less than sign raw,
//! and two stand on both heads at the lines MK-3 and
//! MK-4 draw.

use greenharvest::{
    build_plan, markdown_text, parse_budget, read_records_text, report_document, validate_markdown,
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

fn records_with_suite(suite_json: &str) -> String {
    format!(
        "{{\"format\": \"suitediet-records/1\", \"suite\": {}, \"tests\": [
          {{\"name\": \"alpha\", \"runtime_ms\": 100, \"caught\": [\"d1\"]}}
        ]}}",
        suite_json
    )
}

#[test]
fn test_an_asterisk_in_a_ranking_name_stands_escaped_in_its_cell() {
    let md = rendered(&records_with_name("\"test_a*b\""));
    let row = md
        .lines()
        .find(|line| line.contains("test_a"))
        .expect("the row stands");
    assert!(
        row.contains("| test_a\\*b |"),
        "the cell carries the escaped asterisk: {row}"
    );
}

#[test]
fn test_an_asterisk_in_a_suite_name_stands_escaped_in_the_prose() {
    let md = rendered(&records_with_suite("\"suite*x\""));
    let sentence = md
        .lines()
        .find(|line| line.contains("The suite"))
        .expect("the suite sentence stands");
    assert!(
        sentence.contains("suite\\*x"),
        "the prose carries the escaped asterisk: {sentence}"
    );
}

#[test]
fn test_a_backtick_in_a_ranking_name_stands_escaped_in_its_cell() {
    let md = rendered(&records_with_name("\"test_a`b\""));
    let row = md
        .lines()
        .find(|line| line.contains("test_a"))
        .expect("the row stands");
    assert!(
        row.contains("| test_a\\`b |"),
        "the cell carries the escaped backtick: {row}"
    );
}

#[test]
fn test_a_backtick_in_a_suite_name_stands_escaped_in_the_prose() {
    let md = rendered(&records_with_suite("\"suite`x\""));
    let sentence = md
        .lines()
        .find(|line| line.contains("The suite"))
        .expect("the suite sentence stands");
    assert!(
        sentence.contains("suite\\`x"),
        "the prose carries the escaped backtick: {sentence}"
    );
}

#[test]
fn test_a_less_than_in_a_ranking_name_stands_escaped_in_its_cell() {
    let md = rendered(&records_with_name("\"test_a<b\""));
    let row = md
        .lines()
        .find(|line| line.contains("test_a"))
        .expect("the row stands");
    assert!(
        row.contains("| test_a\\<b |"),
        "the cell carries the escaped less than sign: {row}"
    );
}

#[test]
fn test_a_less_than_in_a_suite_name_stands_escaped_in_the_prose() {
    let md = rendered(&records_with_suite("\"suite<x\""));
    let sentence = md
        .lines()
        .find(|line| line.contains("The suite"))
        .expect("the suite sentence stands");
    assert!(
        sentence.contains("suite\\<x"),
        "the prose carries the escaped less than sign: {sentence}"
    );
}

#[test]
fn test_a_backslash_before_an_asterisk_stands_doubled_and_escaped() {
    let md = rendered(&records_with_name("\"test_a\\\\*b\""));
    let row = md
        .lines()
        .find(|line| line.contains("test_a"))
        .expect("the row stands");
    assert!(
        row.contains("| test_a\\\\\\*b |"),
        "the cell carries the doubled backslash and the escaped asterisk: {row}"
    );
}

#[test]
fn test_an_underscore_inside_a_word_stands_as_itself_in_its_cell() {
    let md = rendered(&records_with_name("\"test_gamma_x\""));
    let row = md
        .lines()
        .find(|line| line.contains("test_gamma_x"))
        .expect("the row stands");
    assert!(
        row.contains("| test_gamma_x |"),
        "the cell carries the name as it stands: {row}"
    );
}

#[test]
fn test_the_markdown_of_markup_names_validates() {
    let set = read_records_text(&records_with_name("\"test_a*b<c\"")).expect("the records read");
    let plan = build_plan(&set, &parse_budget("100000").expect("the budget parses"))
        .expect("the plan builds");
    let report = report_document(&plan);
    let md = markdown_text(&report);
    validate_markdown(&report, &md).expect("the tool's own markdown validates");
}
