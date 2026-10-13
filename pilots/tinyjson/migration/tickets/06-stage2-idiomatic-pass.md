# 06: Stage 2 Idiomatic pass (tinyjson)

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

- src/tinyjson Stage 2 done in 1 attempt: Part aa15689. Gate 8 of 8; 22/22 tests; corpus 0 DIFF of 5755.
- Zero unsafe, TODO(port) count 0. Haiku reported done on a file that did not compile; the governor repaired it.
- Review fixed lost D8/Q5 markers and a D4 gap. Open GAPs in RUN-NOTES: const get_key/get_index unexercised.
