# cJSON report (E4 Stage 1, E5 Stage 2), 2026-10-11

Pinned upstream: cJSON 1.7.19 (6d9f244, MIT), 4,691 lines of C in 2 Parts. Numbers below are from runs I executed
or from the ledgers named in each row.

## Result

| Item                       | Stage 1 (Faithful port)               | Stage 2 (Idiomatic pass)                              |
|----------------------------|---------------------------------------|-------------------------------------------------------|
| Parts done                 | 2 of 2 (`parts.tsv`)                  | 2 of 2 (`parts-stage2.tsv`)                           |
| Gate                       | PASS both Parts, both sets (ee90077)  | PASS both Parts, both sets (3e23a08)                  |
| Translated / upstream tests| 163 passed, 1 ignored (C tests)       | 174 passed, 2 ignored per set                         |
| Differential harness       | 3881 / 3881 inputs match              | 3886 / 3886 inputs match, both sets                   |
| `unsafe` in `rust/src`     | c2rust output (C ABI)                 | 0                                                     |
| `TODO(port)` open          | n/a                                   | 0                                                     |
| Judge sensitivity (mutants)| 8 of 8 detected (E4)                  | 10 of 10 detected, set `""` (RUN-NOTES)               |

Feature sets: `""` and `locales`. A Stage 2 crate has no C ABI; the adapter converts to the C values.

## Rules

- Deviations: 22 rows (D1-D22), all approved by the user (D1, D11, D21, D22 and the D2, D8, D20 text revised in
  Stage 2).
- Kept quirks: 27 rows (Q1-Q27); Q27 and the Q18 first-`prev` quirk were found by the Stage 2 review.
- Amendments recorded in RULEBOOK section 12: 8 rows.

## Review

| Review                   | Verdict | Findings                                  | Outcome                                   |
|--------------------------|---------|-------------------------------------------|-------------------------------------------|
| Opus, Stage 2 port (1st) | fix     | 5 BLOCK, 5 GAP, 6 RULE, 5 NIT             | all applied; Judge went red, then green   |
| Opus, same, 2nd pass     | fix     | 2 BLOCK (markers), 2 GAP, 2 RULE, 3 NIT   | markers added, D8 text, 1 GAP test added  |

The four behaviour BLOCKs of the first review sat on paths the corpus never feeds (allocation failure inside Utils,
NULL `valuestring`, input without a trailing NUL). They are now `judge.md` section 5a. A passing translated-test
suite missed a Q26 regression introduced in attempt 2; the corpus caught it (1840 DIFFs).

## Tokens (output tokens, `SPEND-MODELS.tsv`, 2026-10-10 and later)

| Model             | Output tokens | Note                                                         |
|-------------------|---------------|--------------------------------------------------------------|
| claude-sonnet-5-5 | 441,232       | governor session and implementer / Judge subagents           |
| claude-opus-5-5   | 7,424         | rulebook, review and patch-review subagents (cache reads big)|

Total 448,656 output tokens over 10 phase rows (E4-mapping ... E5-stage2-fixes), against the 490k gate of E4/E5.
Cache reads are about 172M tokens; there is no dollar meter, so cost is not stated.

## Time

Calendar time: 2026-10-10 10:49 to 2026-10-11 about 23:00, roughly two days, of which about 12-14 hours were active
work (estimate from commit timestamps, not a clock). Corpus step per Judge run: about 30-50 s with `--jobs`.

## Interventions by the user

| Intervention                                              | On the allowed list (decision 39)? | Logged as a Skill bug          |
|-----------------------------------------------------------|------------------------------------|--------------------------------|
| Start the deny-ruled session                              | yes (a)                            | -                              |
| Approve Deviations D1-D20, later D21, D22, rule rows      | yes (b)                            | -                              |
| Choose D1 seam, D11 node store, D21 views, D3/D19 option A| yes (b), design decisions          | -                              |
| Skip ticket 03 (c2rust engine), skip LLM polish           | no, a choice                       | WORK-LOG 32                    |
| Hand the review patch to the Skill-editing session        | no, subagent spawn was refused     | WORK-LOG 42 (classifier gap)   |
| Revert of reformatted Judge files (`git checkout`) denied | no, permission classifier           | WORK-LOG 42, 43                |
| Budget check                                              | yes (c)                            | -                              |

Skill bugs found in the run and fixed or filed: WORK-LOG 29, 30, 33, 34, 40, 42, 43 (Judge before crate, `build/`
deny rule, unanchored `Edit(experiments/**)`, done with open tasks, `--also` path and rollback wipe, guard blocking
whole Bash lines with `>`, `cargo fmt` rewriting read-only files).

## Recommended next step

Run the same flow on tinyxml2 (about 2,900 lines of C++, one Part), but first decide the Stage 1 route for C++ (the
c2rust route does not apply): a mapping run only (tickets 01-02, no port) costs a fraction of a day and turns the
1-1.5 day estimate into a measured number. Before that, fix the two Skill gaps that cost the most time here: a gate
that runs the corpus step in the implementer's inner loop, and a guard that does not block a `cargo test` line for
naming `Cargo.toml`.
