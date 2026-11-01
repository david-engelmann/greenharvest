# suitediet

Suite dieting with a safety proof, tests ranked by measured value, a proposed cut line, and every removal shipped with its canary and mutation record, its overlap with surviving tests, and a reversible quarantine before deletion.

## What this is

Suites grow in one direction forever, because deletion feels riskier than bloat and selection systems decide what runs without ever asking what deserves to exist. Suitediet is the deletion half with holdout discipline. It ranks tests by measured value, proposes the cut line, and ships every removal with the evidence that the suite still catches what the removed test caught. Removals sit in quarantine before deletion, so the diet never produces a fast, green, blind suite by accident. The boundary with its siblings is one sentence. Cantfail asks whether a test can fail, suitediet asks which tests earn their runtime.

## Status

Seeded in October 2026. The build has not started. Part of the impatient programming portfolio, in the Gate Truth arc.

## Licence

MIT. See LICENSE.
