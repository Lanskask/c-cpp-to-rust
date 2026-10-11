# 03: Rulebook stress test (cjson)

**What to build:** The hardest Parts are translated twice with the Stage 1 prompt and the differences become rules.

**Blocked by:** 01 Mapping (Stage 0), 02 Stage 1 Judge

**Status:** done

## Tasks

- [x] Translate the hardest Parts twice (skipped: user decision, c2rust engine)
  - pick the 3 hardest `judgment` Parts of `parts.tsv` (fewer if fewer exist)
  - each twice with `references/stage1-prompt.md`, into scratch copies, never the accepted target
- [x] Turn every disagreement into a rule proposal (skipped: user decision, c2rust engine)
  - diff the two translations of each Part; each difference two agents decided differently is a missing rule
  - write them under `## Rule proposals` in RUN-NOTES.md
- [x] Apply the accepted proposals (Opus) (skipped: user decision, c2rust engine)
  - set the rulebook status line to `stress-tested (<date>)`; if the user skips the ticket, record why

## Acceptance

- [x] each difference is a rule proposal in RUN-NOTES.md (skipped: user decision, c2rust engine)
- [x] accepted proposals applied by Opus (skipped: user decision, c2rust engine)
- [x] a same-model rerun agrees, or the remaining differences are classified (skipped: user decision, c2rust engine)
- [x] status line says stress-tested (skipped: user decision, c2rust engine)

-----------
## Done

- Skipped by user decision 2026-10-10: the Stage 1 engine is c2rust, so two LLM translations would only
- compare polished output; the polish prompt got a dry run on cJSON_Utils inside ticket 04 (e693e53)
