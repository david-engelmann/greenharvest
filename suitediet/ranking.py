"""The value rule and the ranking.

A defect counts in full to a test that catches it alone and counts
divided among its catchers when several tests catch it. A test's value
is its weighted catches divided by its runtime. Every share and every
value is an exact rational, and the ranking compares the exact values
even where the printed rendering of two values agrees.
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import ROUND_HALF_UP, Decimal, localcontext
from fractions import Fraction

from .ingest import RecordSet, TestRecord

_SIX_PLACES = Decimal("0.000001")


def derive_catchers(record_set: RecordSet) -> dict[str, tuple[str, ...]]:
    """Map each defect to the sorted names of the tests that catch it.

    The catchers are derived from the records alone, so the defects a
    test caught and the tests that caught a defect stay two readings of
    the same evidence.
    """
    catchers: dict[str, list[str]] = {}
    for test in record_set.tests:
        for defect in test.caught:
            catchers.setdefault(defect, []).append(test.name)
    return {defect: tuple(sorted(names)) for defect, names in sorted(catchers.items())}


@dataclass(frozen=True)
class RankedTest:
    record: TestRecord
    weighted: Fraction
    value: Fraction
    unique_catches: tuple[str, ...]

    @property
    def name(self) -> str:
        return self.record.name


def rank_tests(record_set: RecordSet) -> list[RankedTest]:
    """Order every test by the stated rule.

    Value descending, then weighted catches descending, then runtime
    ascending, then name ascending. The order is total and
    deterministic, so two tests holding identical records are ordered
    by name and nothing in the order depends on input order.
    """
    catchers = derive_catchers(record_set)
    ranked = []
    for test in record_set.tests:
        weighted = sum(
            (Fraction(1, len(catchers[defect])) for defect in test.caught),
            Fraction(0),
        )
        unique = tuple(defect for defect in test.caught if len(catchers[defect]) == 1)
        ranked.append(
            RankedTest(
                record=test,
                weighted=weighted,
                value=weighted / test.runtime_ms,
                unique_catches=unique,
            )
        )
    ranked.sort(
        key=lambda entry: (-entry.value, -entry.weighted, entry.record.runtime_ms, entry.record.name)
    )
    return ranked


def render_value(value: Fraction) -> str:
    """Render an exact rational at six decimal places, rounding half up."""
    with localcontext() as context:
        context.prec = 40
        rendered = (Decimal(value.numerator) / Decimal(value.denominator)).quantize(
            _SIX_PLACES, rounding=ROUND_HALF_UP
        )
    return format(rendered, "f")
