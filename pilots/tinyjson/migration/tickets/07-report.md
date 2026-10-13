# 07: Report (tinyjson)

**What to build:** One page: Parts per stage, Deviations with statuses, open TODO(port), tokens by model, interventions
the user had to make, one recommended next step.

**Blocked by:** 06 Stage 2 Idiomatic pass

**Status:** done

## Tasks

- [x] Collect the numbers
  - `workqueue.py status` for both stages; the §8 Deviations table with statuses
  - tokens for this Pilot from `SPEND.md` / `SPEND-MODELS.tsv`; open `TODO(port)` count
- [x] List the interventions
  - every time the user had to step in; each one not on the allowed list is a Skill bug in WORK-LOG
- [x] Write the report and recommend one next step

## Acceptance

- [x] SPEND-MODELS.tsv rows for the run
- [x] every intervention not in the allowed list is a Skill bug in WORK-LOG
- [x] next step recommended

-----------
## Done

- Report in pilots/tinyjson/migration/REPORT.md. 1 Part, Stage 1 (558bb65, 5 attempts) and Stage 2 (aa15689, 1 attempt).
- Gate 3 of 3 / 8 of 8; 22/22 tests; corpus 0 DIFF of 5755; zero unsafe; TODO(port) 0.
- About 506,800 output tokens over 33 phase rows; Sonnet 5.5 413k, Haiku 5.5 86k, Opus 3k.
- Interventions outside the allowed list are Skill bugs in WORK-LOG 58-64.
