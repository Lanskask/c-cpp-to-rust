# 07: Report (cjson)

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

- migration/REPORT.md: Parts per stage, 22 Deviations, 27 quirks, 2 reviews, tokens by model (448,656 output)
- TODO(port) 0, zero unsafe, 10/10 mutants; interventions table; next step: tinyxml2 mapping run
