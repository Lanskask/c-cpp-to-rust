# 06: Stage 2 Idiomatic pass (cjson)

**What to build:** Every Part is idiomatic Rust that passes the Gate: zero unsafe, clippy and rustfmt clean, no
unjustified unwrap, same Corpus output.

**Blocked by:** 05 Stage 2 Judge

**Status:** done

## Tasks

- [x] Start the fan-out with the guard active
  - `tickets.py mark 06 in_progress`, then an `Edit` of RULEBOOK.md must be refused (SKILL.md 3.1)
- [x] Rewrite every Part in batches
  - `workqueue.py --parts <ws>/migration/parts-stage2.tsv next --batch N`
  - per Part: snapshot -> implementer (`references/stage2-prompt.md`) -> `gate.py --stage 2` -> review -> commit ->
    mark; the Rust stays as close to the C behaviour as the rulebook says
  - a behaviour change is a Deviation row, not a silent edit
- [x] Log every batch and count the open ports
  - `make spend ...` and a WORK-LOG entry per batch
  - `grep -rn "TODO(port)" <ws>/rust/src` for the report

## Acceptance

- [x] all Stage 2 rows done
- [x] Gate passes in every feature set with the full corpus
- [x] TODO(port) count reported

-----------
## Done

- Stage 2 idiomatic port of cJSON and cJSON_Utils: zero unsafe, TODO(port) 0, gate PASS both Parts, both sets (3e23a08)
- 174 translated tests and harness 3886/3886 per set; 10/10 mutants detected (set default)
- Two Opus reviews: 5 BLOCK fixed, markers added; RULEBOOK Q27, D22, D8, D2, D20, Q18 rows
