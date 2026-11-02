"""The quarantine record and the restore path.

A proposed removal moves the test into a quarantine record carrying
its full record, and nothing is deleted. Restoring reads the record
set that remained and the quarantine record and returns the suite
record exactly as it stood before the plan.
"""

from __future__ import annotations

from .errors import SuitedietError
from .ingest import Quarantine, RecordSet
from .plan import DISPOSITION_QUARANTINE, Plan


def build_quarantine(record_set: RecordSet, plan: Plan) -> Quarantine:
    cleared = {
        pack.ranked.name for pack in plan.packs if pack.disposition == DISPOSITION_QUARANTINE
    }
    tests = tuple(test for test in record_set.tests if test.name in cleared)
    return Quarantine(suite=record_set.suite, tests=tests)


def remaining_records(record_set: RecordSet, plan: Plan) -> RecordSet:
    remaining = set(plan.remaining_names)
    tests = tuple(test for test in record_set.tests if test.name in remaining)
    return RecordSet(suite=record_set.suite, tests=tests)


def restore_records(remaining: RecordSet, quarantine: Quarantine) -> RecordSet:
    if remaining.suite != quarantine.suite:
        raise SuitedietError(
            f"the quarantine record belongs to the suite {quarantine.suite!r} "
            f"and the remaining records to {remaining.suite!r}"
        )
    standing = {test.name for test in remaining.tests}
    for test in quarantine.tests:
        if test.name in standing:
            raise SuitedietError(
                f"the quarantined test {test.name} is still standing in the remaining records"
            )
    merged = tuple(sorted(remaining.tests + quarantine.tests, key=lambda test: test.name))
    return RecordSet(suite=remaining.suite, tests=merged)
