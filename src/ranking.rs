//! The value rule and the ranking.
//!
//! A defect counts in full to a test that catches it alone and
//! counts divided among its catchers when several tests catch it.
//! A test's value is its weighted catches divided by its runtime.
//! Every share and every value is an exact rational, and the
//! ranking compares the exact values even where the printed
//! rendering of two values agrees.

use std::collections::HashMap;

use crate::bigint::BigInt;
use crate::ingest::{RecordSet, TestRecord};
use crate::rational::Rational;

/// Derive the catchers of every defect from the records alone, as
/// defect and sorted catcher names pairs in defect order, so the
/// defects a test caught and the tests that caught a defect stay
/// two readings of the same evidence. The derivation is indexed by
/// defect as it is built, so a record set with many tests sharing
/// many defects costs one lookup per caught entry rather than a
/// scan of the table per entry.
pub fn derive_catchers(record_set: &RecordSet) -> Vec<(String, Vec<String>)> {
    let mut index: HashMap<&str, usize> = HashMap::new();
    let mut catchers: Vec<(String, Vec<String>)> = Vec::new();
    for test in &record_set.tests {
        for defect in &test.caught {
            match index.get(defect.as_str()) {
                Some(&slot) => catchers[slot].1.push(test.name.clone()),
                None => {
                    index.insert(defect.as_str(), catchers.len());
                    catchers.push((defect.clone(), vec![test.name.clone()]));
                }
            }
        }
    }
    for (_, names) in &mut catchers {
        names.sort();
    }
    catchers.sort_by(|a, b| a.0.cmp(&b.0));
    catchers
}

/// Look up the catchers of one defect in the derived table, which
/// derive_catchers returns in defect order, so the lookup is a
/// binary search over the table.
pub fn catchers_of<'a>(catchers: &'a [(String, Vec<String>)], defect: &str) -> &'a [String] {
    let slot = catchers
        .binary_search_by(|(name, _)| name.as_str().cmp(defect))
        .expect("every caught defect stands in the table");
    catchers[slot].1.as_slice()
}

/// The weighted catches of one test from the catcher counts of its
/// defects, the exact sum of one divided by the count over every
/// defect it caught. The sum is taken over a common denominator,
/// the least common multiple of the counts, and reduced once at
/// the end, which stands as the same rational the step by step sum
/// of shares reduced at every step would reach, at a small
/// fraction of the cost when a test catches many defects.
fn weighted_catches(counts: &[u64]) -> Rational {
    if counts.is_empty() {
        return Rational::from_int(BigInt::zero());
    }
    let mut common = BigInt::from_i64(1);
    for &count in counts {
        let count = BigInt::from_u64(count);
        let gcd = common.gcd_abs(&count);
        let (reduced, _) = common.divmod_abs(&gcd);
        common = reduced.mul(&count);
    }
    let mut numerator = BigInt::zero();
    for &count in counts {
        let (share, _) = common.divmod_abs(&BigInt::from_u64(count));
        numerator = numerator.add(&share);
    }
    Rational::new(numerator, common)
}

/// One test under the value rule.
#[derive(Debug, Clone)]
pub struct RankedTest {
    pub record: TestRecord,
    pub weighted: Rational,
    pub value: Rational,
    pub unique_catches: Vec<String>,
}

impl RankedTest {
    pub fn name(&self) -> &str {
        &self.record.name
    }
}

/// Order every test by the stated rule.
///
/// Value descending, then weighted catches descending, then
/// runtime ascending, then name ascending. The order is total and
/// deterministic, so two tests holding identical records are
/// ordered by name and nothing in the order depends on input
/// order.
pub fn rank_tests(record_set: &RecordSet) -> Vec<RankedTest> {
    let catchers = derive_catchers(record_set);
    let mut ranked: Vec<RankedTest> = Vec::new();
    for test in &record_set.tests {
        let counts: Vec<u64> = test
            .caught
            .iter()
            .map(|defect| catchers_of(&catchers, defect).len() as u64)
            .collect();
        let weighted = weighted_catches(&counts);
        let unique: Vec<String> = test
            .caught
            .iter()
            .filter(|defect| catchers_of(&catchers, defect).len() == 1)
            .cloned()
            .collect();
        let value = weighted.div_int(&test.runtime_ms);
        ranked.push(RankedTest {
            record: test.clone(),
            weighted,
            value,
            unique_catches: unique,
        });
    }
    ranked.sort_by(|a, b| {
        b.value
            .cmp(&a.value)
            .then_with(|| b.weighted.cmp(&a.weighted))
            .then_with(|| a.record.runtime_ms.cmp(&b.record.runtime_ms))
            .then_with(|| a.record.name.cmp(&b.record.name))
    });
    ranked
}

/// Render an exact rational at six decimal places, rounding half
/// up, as the Python rendering renders one.
pub fn render_value(value: &Rational) -> String {
    crate::rational::render_value(value)
}
