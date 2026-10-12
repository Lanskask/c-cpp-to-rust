# 03: Rulebook stress test (tinyjson)

**What to build:** The hardest Parts are translated twice with the Stage 1 prompt and the differences become rules.

**Blocked by:** 01 Mapping (Stage 0), 02 Stage 1 Judge

**Status:** done (2026-10-12)

## Tasks

- [x] Translate the hardest Parts twice
  - pick the 3 hardest `judgment` Parts of `parts.tsv` (fewer if fewer exist)
  - each twice with `references/stage1-prompt.md`, into scratch copies, never the accepted target
- [x] Turn every disagreement into a rule proposal
  - diff the two translations of each Part; each difference two agents decided differently is a missing rule
  - write them under `## Rule proposals` in RUN-NOTES.md
- [x] Apply the accepted proposals (Opus)
  - set the rulebook status line to `stress-tested (<date>)`; if the user skips the ticket, record why

## Acceptance

- [x] each difference is a rule proposal in RUN-NOTES.md
  - shown by RUN-NOTES `## Rule proposals` P1-P11, each citing its review item in
    `pilots/tinyjson/experiments/stress_test_diff_review.md`
- [x] accepted proposals applied by Opus
  - shown by RULEBOOK §12 rows P1-P11, the §4 marker rule and the round 2b tier and I8/D8 rows
- [x] a same-model rerun agrees, or the remaining differences are classified
  - shown by RUN-NOTES `### Round 2 result`: B3/C3 agree, A3 differences classified, the `stod` gap is agent-tier
    (Sonnet `R2c-A3` 0 of 5257 mismatches)
- [x] status line says stress-tested
  - shown by RULEBOOK line 3: `Status: stress-tested (2026-10-12)`
