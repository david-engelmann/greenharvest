//! The quarantine record and the restore path.
//!
//! A proposed removal moves the test into a quarantine record
//! carrying its full record, and nothing is deleted. Restoring
//! reads the record set that remained and the quarantine record
//! and returns the suite record exactly as it stood before the
//! plan.

use crate::bigint::BigInt;
use crate::errors::GreenharvestError;
use crate::ingest::{Quarantine, RecordSet};
use crate::jsonvalue::JsonValue;
use crate::plan::{Plan, DISPOSITION_QUARANTINE};
use crate::render::repr_string;

/// Build the quarantine record of a plan, carrying the full
/// record of every cleared test and of no other test.
pub fn build_quarantine(record_set: &RecordSet, plan: &Plan) -> Quarantine {
    let cleared: Vec<&str> = plan
        .packs
        .iter()
        .filter(|pack| pack.disposition == DISPOSITION_QUARANTINE)
        .map(|pack| pack.ranked.name())
        .collect();
    let tests = record_set
        .tests
        .iter()
        .filter(|test| cleared.contains(&test.name.as_str()))
        .cloned()
        .collect();
    Quarantine {
        suite: record_set.suite.clone(),
        tests,
    }
}

/// The record set that remains after a plan, the survivors and
/// the blocked tests.
pub fn remaining_records(record_set: &RecordSet, plan: &Plan) -> RecordSet {
    let tests = record_set
        .tests
        .iter()
        .filter(|test| plan.remaining_names.contains(&test.name))
        .cloned()
        .collect();
    RecordSet {
        suite: record_set.suite.clone(),
        tests,
    }
}

/// Restore the suite record set from the record set that remained
/// and the quarantine record, refusing a quarantine record whose
/// suite disagrees or whose tests collide with tests still
/// standing.
pub fn restore_records(
    remaining: &RecordSet,
    quarantine: &Quarantine,
) -> Result<RecordSet, GreenharvestError> {
    if remaining.suite != quarantine.suite {
        return Err(GreenharvestError::new(format!(
            "the quarantine record belongs to the suite {} and the remaining records to {}",
            repr_string(&quarantine.suite),
            repr_string(&remaining.suite),
        )));
    }
    for test in &quarantine.tests {
        if remaining.record_named(&test.name).is_some() {
            return Err(GreenharvestError::new(format!(
                "the quarantined test {} is still standing in the remaining records",
                test.name
            )));
        }
    }
    let mut merged: Vec<crate::ingest::TestRecord> = Vec::new();
    merged.extend(remaining.tests.iter().cloned());
    merged.extend(quarantine.tests.iter().cloned());
    merged.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(RecordSet {
        suite: remaining.suite.clone(),
        tests: merged,
    })
}

/// Validate a restored record set against the remaining
/// records and the quarantine record it was restored from.
/// The expected restored set is the standing restore
/// computation's own output, its refusals standing as this
/// reading's refusals, and any field of the restored set
/// differing from it refuses in the disagreement words.
pub fn validate_restore(
    remaining: &RecordSet,
    quarantine: &Quarantine,
    restored: &RecordSet,
) -> Result<(), GreenharvestError> {
    let expected = restore_records(remaining, quarantine)?;
    if expected != *restored {
        return Err(GreenharvestError::new(
            "the restored record set disagrees with the remaining records and the quarantine record",
        ));
    }
    Ok(())
}

/// One removal pack's record read in the held form, as the
/// records reading holds a record, its caught list sorted.
/// A record standing in a shape that reading refuses cannot
/// be the record of a plan this tool states, and the caller
/// reads that standing as a disagreement.
fn pack_record(record: &JsonValue) -> Option<crate::ingest::TestRecord> {
    let name = record.get("name")?.as_text()?;
    if name.is_empty() {
        return None;
    }
    let runtime = match record.get("runtime_ms")? {
        JsonValue::Int(value) => BigInt::from_i64(*value),
        JsonValue::BigInt(text) => BigInt::from_decimal(text)?,
        _ => return None,
    };
    if !runtime.is_positive() {
        return None;
    }
    let mut caught: Vec<String> = Vec::new();
    for defect in record.get("caught")?.as_list()? {
        let text = defect.as_text()?;
        if text.is_empty() {
            return None;
        }
        caught.push(text.to_string());
    }
    caught.sort();
    let mut deduped = caught.clone();
    deduped.dedup();
    if deduped.len() != caught.len() {
        return None;
    }
    Some(crate::ingest::TestRecord {
        name: name.to_string(),
        runtime_ms: runtime,
        caught,
    })
}

/// Validate a quarantine record against the report of its
/// plan (VQ-2 and VQ-3). The expected quarantine record is
/// built from the report as read, carrying the suite the
/// report carries and the full record of every removal pack
/// whose disposition is quarantine and of no other pack, its
/// tests standing in the name order the held form stands in.
/// Every field of the quarantine record as read must equal
/// it, the comparison standing over the held forms as values,
/// so the spelling of either file, its key order and its
/// whitespace, does not stand in the comparison.
pub fn validate_quarantine(
    report: &JsonValue,
    quarantine: &Quarantine,
) -> Result<(), GreenharvestError> {
    let disagreement =
        || GreenharvestError::new("the quarantine record disagrees with the report of its plan");
    let suite = report
        .get("suite")
        .and_then(JsonValue::as_text)
        .ok_or_else(disagreement)?;
    let packs = report
        .get("removal_packs")
        .and_then(JsonValue::as_list)
        .ok_or_else(disagreement)?;
    let mut tests: Vec<crate::ingest::TestRecord> = Vec::new();
    for pack in packs {
        let cleared = matches!(
            pack.get("disposition"),
            Some(JsonValue::Text(text)) if text == DISPOSITION_QUARANTINE
        );
        if cleared {
            let record = pack.get("record").ok_or_else(disagreement)?;
            tests.push(pack_record(record).ok_or_else(disagreement)?);
        }
    }
    tests.sort_by(|a, b| a.name.cmp(&b.name));
    let expected = Quarantine {
        suite: suite.to_string(),
        tests,
    };
    if expected != *quarantine {
        return Err(disagreement());
    }
    Ok(())
}
