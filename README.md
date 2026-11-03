# suitediet

Suite dieting with a safety proof, tests ranked by measured value, a proposed cut line, and every removal shipped with its canary and mutation record, its overlap with the tests still standing after the plan, and a reversible quarantine before deletion.

## What this is

Suites grow in one direction forever, because deletion feels riskier than bloat and selection systems decide what runs without ever asking what deserves to exist. Suitediet is the deletion half with holdout discipline. It ranks tests by measured value, proposes the cut line, and ships every removal with the evidence that the suite still catches what the removed test caught. Removals sit in quarantine before deletion, so the diet never produces a fast, green, blind suite by accident. The boundary with its siblings is one sentence. Cantfail asks whether a test can fail, suitediet asks which tests earn their runtime.

## Status

- Slice 2 is built and proven on synthetic targets only
- Slice 2 reads one measured record set per suite in the format suitediet-records/1, holding each test's runtime in whole milliseconds and the defects it caught, and refuses a record set it cannot read exactly
- Slice 2 ranks tests by measured value under the stated rule, where a defect counts in full to its only catcher and divided among its catchers otherwise and value is weighted catches per millisecond compared as exact rationals
- Slice 2 draws the cut line at a runtime budget and ships a removal pack for every test below it, and a defect whose catchers would all be removed blocks the removal and is printed as the block, so the plan loses no catch
- Slice 2 moves every proposed removal into a quarantine record in the format suitediet-quarantine/1 carrying the full records, deletes nothing, and restores the suite record exactly from the remaining records and the quarantine record
- Slice 2 is the Round 1 work increment and is built on a local branch, and the round closes only after audit and two-axis review and red team and owner approval
- Rust port slice 1 is built on a local branch and carries the records reading and the ranking and the plan and the quarantine and the restore and the report behind two doors, proven by recorded conformance drawn from the Python, and its audit and two axis review and red team stand landed with the fix pass their findings ordered
- Closed rounds stand at 2 of 100
- Seeded in October 2026. Part of the impatient programming portfolio, in the Gate Truth arc

## Licence

MIT. See LICENSE.
