# 06: Stage 2 Idiomatic pass (tinyjson)

**What to build:** Every Part is idiomatic Rust that passes the Gate: zero unsafe, clippy and rustfmt clean, no
unjustified unwrap, same Corpus output.

**Blocked by:** 05 Stage 2 Judge

**Status:** in_progress

## Tasks

- [x] Start the fan-out with the guard active
  - `tickets.py mark 06 in_progress`, then an `Edit` of RULEBOOK.md must be refused (SKILL.md 3.1)
- [ ] Rewrite every Part in batches
  - `workqueue.py --parts <ws>/migration/parts-stage2.tsv next --batch N`
  - per Part: snapshot -> implementer (`references/stage2-prompt.md`) -> `gate.py --stage 2` -> review -> commit ->
    mark; the Rust stays as close to the C behaviour as the rulebook says
  - a behaviour change is a Deviation row, not a silent edit
- [ ] Log every batch and count the open ports
  - `make spend ...` and a WORK-LOG entry per batch
  - `grep -rn "TODO(port)" <ws>/rust/src` for the report

## Acceptance

- [ ] all Stage 2 rows done
- [ ] Gate passes in every feature set with the full corpus
- [ ] TODO(port) count reported
