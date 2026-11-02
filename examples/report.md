# suitediet diet report

The suite synthetic-suite under a budget of 600 milliseconds. The survivors stand at 465 milliseconds of runtime.

## Ranking

| Rank | Test | Runtime ms | Weighted catches | Value | Catches | Unique catches |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | test_gamma | 45 | 1.000000 | 0.022222 | 1 | 1 |
| 2 | test_alpha | 120 | 2.000000 | 0.016667 | 3 | 1 |
| 3 | test_beta | 300 | 1.000000 | 0.003333 | 2 | 0 |
| 4 | test_zeta | 240 | 0.500000 | 0.002083 | 1 | 0 |
| 5 | test_delta | 500 | 1.000000 | 0.002000 | 1 | 1 |
| 6 | test_epsilon | 260 | 0.500000 | 0.001923 | 1 | 0 |
| 7 | test_eta | 90 | 0.000000 | 0.000000 | 0 | 0 |

## Removal packs

- test_eta is proposed for quarantine.
- test_epsilon is proposed for quarantine. Overlap with the tests still standing after the plan. d6 is also caught by test_zeta.
- test_delta is held in the suite. Its removal would lose d5. Overlap with the tests still standing after the plan. d5 is also caught by no other test still standing.
- test_zeta is held in the suite. Its removal would lose d6. Overlap with the tests still standing after the plan. d6 is also caught by no other test still standing.

## Totals

The suite catches 6 defects before the plan and 6 after the proposed removals.
The plan loses no defect.
