//! The cut line and the removal decisions.
//!
//! The survivors are the longest prefix of the ranking whose
//! combined runtime fits inside the budget. Removals are decided
//! from below the line, worst first, against the set of tests
//! still standing, so two tests that only cover each other split
//! one cleared and one blocked rather than both clearing and
//! losing the catch between them.

use std::collections::BTreeSet;

use crate::bigint::BigInt;
use crate::errors::GreenharvestError;
use crate::ingest::{RecordSet, TestRecord};
use crate::ranking::{catchers_of, derive_catchers, rank_tests, RankedTest};

pub const DISPOSITION_QUARANTINE: &str = "quarantine";
pub const DISPOSITION_BLOCKED: &str = "blocked";

/// The removal pack of one test below the line.
#[derive(Debug, Clone)]
pub struct RemovalPack {
    pub ranked: RankedTest,
    pub rank: usize,
    pub disposition: String,
    pub overlap: Vec<(String, Vec<String>)>,
    pub blocking_defects: Vec<String>,
}

/// A plan whole.
#[derive(Debug, Clone)]
pub struct Plan {
    pub record_set: RecordSet,
    pub budget_ms: BigInt,
    pub ranking: Vec<RankedTest>,
    pub survivors: Vec<RankedTest>,
    pub below_the_line: Vec<RankedTest>,
    pub packs: Vec<RemovalPack>,
    pub remaining_names: Vec<String>,
    pub defects_before: Vec<String>,
    pub defects_after: Vec<String>,
    pub lost_defects: Vec<String>,
    pub survivor_runtime_ms: BigInt,
}

/// Build the plan for a record set under a budget in whole
/// milliseconds.
pub fn build_plan(record_set: &RecordSet, budget_ms: &BigInt) -> Result<Plan, GreenharvestError> {
    if !budget_ms.is_positive() {
        return Err(GreenharvestError::new(
            "the budget is not a positive whole number of milliseconds",
        ));
    }
    let ranking = rank_tests(record_set);
    let rank_of = |name: &str| -> usize {
        ranking
            .iter()
            .position(|entry| entry.name() == name)
            .expect("every ranked test stands in the ranking")
            + 1
    };

    let mut survivors: Vec<RankedTest> = Vec::new();
    let mut spent = BigInt::zero();
    for entry in &ranking {
        let next = spent.add(&entry.record.runtime_ms);
        if next.cmp(budget_ms) == std::cmp::Ordering::Greater {
            break;
        }
        survivors.push(entry.clone());
        spent = next;
    }
    let survivor_names: BTreeSet<&str> = survivors.iter().map(|entry| entry.name()).collect();
    let below: Vec<RankedTest> = ranking
        .iter()
        .filter(|entry| !survivor_names.contains(entry.name()))
        .cloned()
        .collect();

    let catchers = derive_catchers(record_set);

    let mut cleared: BTreeSet<String> = BTreeSet::new();
    let mut decisions: Vec<(String, String, Vec<String>)> = Vec::new();
    for entry in below.iter().rev() {
        let mut blocking: Vec<String> = entry
            .record
            .caught
            .iter()
            .filter(|defect| {
                catchers_of(&catchers, defect)
                    .iter()
                    .all(|name| cleared.contains(name) || name == entry.name())
            })
            .cloned()
            .collect();
        blocking.sort();
        if blocking.is_empty() {
            decisions.push((
                entry.name().to_string(),
                DISPOSITION_QUARANTINE.to_string(),
                blocking,
            ));
            cleared.insert(entry.name().to_string());
        } else {
            decisions.push((
                entry.name().to_string(),
                DISPOSITION_BLOCKED.to_string(),
                blocking,
            ));
        }
    }

    let remaining: Vec<String> = record_set
        .tests
        .iter()
        .map(|test| test.name.clone())
        .filter(|name| !cleared.contains(name))
        .collect();
    let remaining_set: BTreeSet<&str> = remaining.iter().map(String::as_str).collect();

    let mut packs: Vec<RemovalPack> = Vec::new();
    for entry in below.iter().rev() {
        let (_, disposition, blocking) = decisions
            .iter()
            .find(|(name, _, _)| name == entry.name())
            .expect("every test below the line carries a decision")
            .clone();
        let overlap: Vec<(String, Vec<String>)> = entry
            .record
            .caught
            .iter()
            .map(|defect| {
                let names: Vec<String> = catchers_of(&catchers, defect)
                    .iter()
                    .filter(|name| {
                        remaining_set.contains(name.as_str()) && name.as_str() != entry.name()
                    })
                    .cloned()
                    .collect();
                (defect.clone(), names)
            })
            .collect();
        packs.push(RemovalPack {
            ranked: entry.clone(),
            rank: rank_of(entry.name()),
            disposition,
            overlap,
            blocking_defects: blocking,
        });
    }

    let record_named = |name: &str| -> &TestRecord {
        record_set
            .record_named(name)
            .expect("every standing name stands in the records")
    };
    let mut before_set: BTreeSet<&str> = BTreeSet::new();
    for test in &record_set.tests {
        for defect in &test.caught {
            before_set.insert(defect.as_str());
        }
    }
    let mut after_set: BTreeSet<&str> = BTreeSet::new();
    for name in &remaining {
        for defect in &record_named(name).caught {
            after_set.insert(defect.as_str());
        }
    }
    let defects_before: Vec<String> = before_set.iter().map(|defect| defect.to_string()).collect();
    let defects_after: Vec<String> = after_set.iter().map(|defect| defect.to_string()).collect();
    let lost: Vec<String> = defects_before
        .iter()
        .filter(|defect| !after_set.contains(defect.as_str()))
        .cloned()
        .collect();

    Ok(Plan {
        record_set: record_set.clone(),
        budget_ms: budget_ms.clone(),
        ranking,
        survivors,
        below_the_line: below,
        packs,
        remaining_names: remaining,
        defects_before,
        defects_after,
        lost_defects: lost,
        survivor_runtime_ms: spent,
    })
}
