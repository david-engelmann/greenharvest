//! Suitediet ranks tests by measured value and diets a suite
//! safely.
//!
//! This crate is the Rust port of the Python package of the same
//! name, standing under the port obligations of the contract.
//! Slice 1 carries the records reading with its duplicate key
//! refusal, the ranking on exact rationals, the plan and the
//! quarantine and the restore, and the report as a document and as
//! markdown, and it proves each against recorded conformance drawn
//! from the landed Python. The command line stays in the Python
//! package in this slice, as the contract records.

pub mod bigint;
pub mod budget;
pub mod errors;
pub mod ingest;
pub mod jsonvalue;
pub mod plan;
pub mod pytext;
pub mod quarantine;
pub mod ranking;
pub mod rational;
pub mod render;
pub mod report;
pub mod serialize;
pub mod tables;

pub use budget::parse_budget;
pub use errors::SuitedietError;
pub use ingest::{
    read_quarantine, read_quarantine_text, read_records, read_records_text, read_remaining,
    read_remaining_text, Quarantine, RecordSet, TestRecord,
};
pub use jsonvalue::JsonValue;
pub use plan::{build_plan, Plan, RemovalPack};
pub use quarantine::{build_quarantine, remaining_records, restore_records};
pub use ranking::{derive_catchers, rank_tests, RankedTest};
pub use render::canonical_text;
pub use report::{markdown_text, report_document};
