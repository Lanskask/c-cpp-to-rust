# 03: Rulebook stress test (tinyjson)

**What to build:** The hardest Parts are translated twice with the Stage 1 prompt and the differences become rules.

**Blocked by:** 01 Mapping (Stage 0), 02 Stage 1 Judge

**Status:** in_progress

## Tasks

- [x] Translate the hardest Parts twice
  - pick the 3 hardest `judgment` Parts of `parts.tsv` (fewer if fewer exist)
  - each twice with `references/stage1-prompt.md`, into scratch copies, never the accepted target
- [x] Turn every disagreement into a rule proposal
  - diff the two translations of each Part; each difference two agents decided differently is a missing rule
  - write them under `## Rule proposals` in RUN-NOTES.md
- [ ] Apply the accepted proposals (Opus)
  - set the rulebook status line to `stress-tested (<date>)`; if the user skips the ticket, record why

## Acceptance

- [ ] each difference is a rule proposal in RUN-NOTES.md
- [ ] accepted proposals applied by Opus
- [ ] a same-model rerun agrees, or the remaining differences are classified
- [ ] status line says stress-tested
