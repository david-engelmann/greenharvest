"""The diet report, as a JSON document and as a markdown rendering.

Every number in the report is read from the plan it reports. The
totals are computed from the defect sets themselves, never copied from
one another, so the after total can never claim a catch the plan
loses.
"""

from __future__ import annotations

from .ingest import record_document
from .plan import DISPOSITION_BLOCKED, DISPOSITION_QUARANTINE, Plan
from .ranking import render_value

REPORT_FORMAT = "suitediet-report/1"


def report_document(plan: Plan) -> dict:
    ranking_rows = []
    for position, entry in enumerate(plan.ranking):
        ranking_rows.append(
            {
                "rank": position + 1,
                "name": entry.name,
                "runtime_ms": entry.record.runtime_ms,
                "weighted_catches": render_value(entry.weighted),
                "value": render_value(entry.value),
                "catches": len(entry.record.caught),
                "unique_catches": len(entry.unique_catches),
            }
        )
    pack_rows = []
    for pack in plan.packs:
        pack_rows.append(
            {
                "name": pack.ranked.name,
                "disposition": pack.disposition,
                "rank": pack.rank,
                "runtime_ms": pack.ranked.record.runtime_ms,
                "weighted_catches": render_value(pack.ranked.weighted),
                "value": render_value(pack.ranked.value),
                "record": record_document(pack.ranked.record),
                "overlap": {defect: list(names) for defect, names in pack.overlap},
                "blocking_defects": list(pack.blocking_defects),
            }
        )
    return {
        "format": REPORT_FORMAT,
        "suite": plan.record_set.suite,
        "budget_ms": plan.budget_ms,
        "ranking": ranking_rows,
        "survivors": [entry.name for entry in plan.survivors],
        "survivor_runtime_ms": plan.survivor_runtime_ms,
        "below_the_line": [entry.name for entry in plan.below_the_line],
        "removal_packs": pack_rows,
        "totals": {
            "defects_before": len(plan.defects_before),
            "defects_after": len(plan.defects_after),
            "defects_before_list": list(plan.defects_before),
            "defects_after_list": list(plan.defects_after),
            "lost_defects": list(plan.lost_defects),
        },
        "quarantined": sorted(
            pack.ranked.name for pack in plan.packs if pack.disposition == DISPOSITION_QUARANTINE
        ),
        "blocked": sorted(
            pack.ranked.name for pack in plan.packs if pack.disposition == DISPOSITION_BLOCKED
        ),
    }


def markdown_text(report: dict) -> str:
    lines = []
    lines.append("# suitediet diet report")
    lines.append("")
    lines.append(
        f"The suite {report['suite']} under a budget of {report['budget_ms']} milliseconds. "
        f"The survivors stand at {report['survivor_runtime_ms']} milliseconds of runtime."
    )
    lines.append("")
    lines.append("## Ranking")
    lines.append("")
    lines.append("| Rank | Test | Runtime ms | Weighted catches | Value | Catches | Unique catches |")
    lines.append("| --- | --- | --- | --- | --- | --- | --- |")
    for row in report["ranking"]:
        lines.append(
            f"| {row['rank']} | {row['name']} | {row['runtime_ms']} | "
            f"{row['weighted_catches']} | {row['value']} | {row['catches']} | {row['unique_catches']} |"
        )
    lines.append("")
    lines.append("## Removal packs")
    lines.append("")
    if not report["removal_packs"]:
        lines.append("No test stands below the cut line, so no removal is proposed.")
    for pack in report["removal_packs"]:
        if pack["disposition"] == DISPOSITION_BLOCKED:
            blocked = " and ".join(pack["blocking_defects"])
            line = (
                f"- {pack['name']} is held in the suite. Its removal would lose {blocked}."
            )
        else:
            line = f"- {pack['name']} is proposed for quarantine."
        overlap_sentences = []
        for defect, names in pack["overlap"].items():
            if names:
                overlap_sentences.append(
                    f"{defect} is also caught by {' and '.join(names)}"
                )
            else:
                overlap_sentences.append(
                    f"{defect} is also caught by no other test still standing"
                )
        if overlap_sentences:
            line = (
                f"{line} Overlap with the tests still standing after the plan. "
                f"{'. '.join(overlap_sentences)}."
            )
        lines.append(line)
    lines.append("")
    lines.append("## Totals")
    lines.append("")
    totals = report["totals"]
    lines.append(
        f"The suite catches {totals['defects_before']} defects before the plan and "
        f"{totals['defects_after']} after the proposed removals."
    )
    if totals["lost_defects"]:
        lines.append(f"The plan loses {' and '.join(totals['lost_defects'])}.")
    else:
        lines.append("The plan loses no defect.")
    lines.append("")
    return "\n".join(lines)
