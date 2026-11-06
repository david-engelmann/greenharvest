//! The quarantine record and the restore path.
//!
//! A proposed removal moves the test into a quarantine record
//! carrying its full record, and nothing is deleted. Restoring
//! reads the record set that remained and the quarantine record
//! and returns the suite record exactly as it stood before the
//! plan.

use crate::errors::GreenharvestError;
use crate::ingest::{Quarantine, RecordSet};
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
