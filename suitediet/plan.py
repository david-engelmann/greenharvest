"""The cut line and the removal decisions.

The survivors are the longest prefix of the ranking whose combined
runtime fits inside the budget. Removals are decided from below the
line, worst first, against the set of tests still standing, so two
tests that only cover each other split one cleared and one blocked
rather than both clearing and losing the catch between them.
"""

from __future__ import annotations

from dataclasses import dataclass

from .errors import SuitedietError
from .ingest import RecordSet, TestRecord
from .ranking import RankedTest, derive_catchers, rank_tests

DISPOSITION_QUARANTINE = "quarantine"
DISPOSITION_BLOCKED = "blocked"


@dataclass(frozen=True)
class RemovalPack:
    ranked: RankedTest
    rank: int
    disposition: str
    overlap: tuple[tuple[str, tuple[str, ...]], ...]
    blocking_defects: tuple[str, ...]


@dataclass(frozen=True)
class Plan:
    record_set: RecordSet
    budget_ms: int
    ranking: tuple[RankedTest, ...]
    survivors: tuple[RankedTest, ...]
    below_the_line: tuple[RankedTest, ...]
    packs: tuple[RemovalPack, ...]
    remaining_names: tuple[str, ...]
    defects_before: tuple[str, ...]
    defects_after: tuple[str, ...]
    lost_defects: tuple[str, ...]
    survivor_runtime_ms: int


def build_plan(record_set: RecordSet, budget_ms: int) -> Plan:
    if isinstance(budget_ms, bool) or not isinstance(budget_ms, int) or budget_ms <= 0:
        raise SuitedietError("the budget is not a positive whole number of milliseconds")
    ranking = rank_tests(record_set)
    rank_of = {entry.name: position + 1 for position, entry in enumerate(ranking)}

    survivors = []
    spent = 0
    for entry in ranking:
        if spent + entry.record.runtime_ms > budget_ms:
            break
        survivors.append(entry)
        spent += entry.record.runtime_ms
    survivor_names = {entry.name for entry in survivors}
    below = tuple(entry for entry in ranking if entry.name not in survivor_names)

    catchers = derive_catchers(record_set)
    catchers_of = {defect: set(names) for defect, names in catchers.items()}

    cleared: set[str] = set()
    decisions: dict[str, tuple[str, tuple[str, ...]]] = {}
    for entry in reversed(below):
        blocking = tuple(
            sorted(
                defect
                for defect in entry.record.caught
                if catchers_of[defect] <= (cleared | {entry.name})
            )
        )
        if blocking:
            decisions[entry.name] = (DISPOSITION_BLOCKED, blocking)
        else:
            decisions[entry.name] = (DISPOSITION_QUARANTINE, ())
            cleared.add(entry.name)

    remaining = tuple(
        sorted(test.name for test in record_set.tests if test.name not in cleared)
    )
    remaining_set = set(remaining)

    packs = []
    for entry in reversed(below):
        disposition, blocking = decisions[entry.name]
        overlap = tuple(
            (
                defect,
                tuple(
                    name
                    for name in catchers[defect]
                    if name in remaining_set and name != entry.name
                ),
            )
            for defect in entry.record.caught
        )
        packs.append(
            RemovalPack(
                ranked=entry,
                rank=rank_of[entry.name],
                disposition=disposition,
                overlap=overlap,
                blocking_defects=blocking,
            )
        )

    by_name = record_set.by_name()
    defects_before = tuple(
        sorted({defect for test in record_set.tests for defect in test.caught})
    )
    defects_after = tuple(
        sorted({defect for name in remaining for defect in by_name[name].caught})
    )
    lost = tuple(defect for defect in defects_before if defect not in set(defects_after))

    return Plan(
        record_set=record_set,
        budget_ms=budget_ms,
        ranking=tuple(ranking),
        survivors=tuple(survivors),
        below_the_line=below,
        packs=tuple(packs),
        remaining_names=remaining,
        defects_before=defects_before,
        defects_after=defects_after,
        lost_defects=lost,
        survivor_runtime_ms=spent,
    )
