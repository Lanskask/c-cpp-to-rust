# 04: Stage 1 Faithful port (tinyjson)

**What to build:** Every Part is done or stubbed, each accepted by the Gate and committed.

**Blocked by:** 03 Rulebook stress test

**Status:** done

## Tasks

- [x] Protect the Rulebook and the Judge, then start the fan-out
  - session started with `claude --settings skill/c-to-rust/assets/settings.json` (SKILL.md step 3.1)
  - `tickets.py mark 04 in_progress`, then an `Edit` of RULEBOOK.md must be refused; if not, stop
  - write `rust/src/lib.rs` (`pub mod <stem>;` per Part) and the cargo features of rulebook §3
- [x] Port every Part in batches
  - `workqueue.py --parts <ws>/migration/parts.tsv next --batch N`
  - per Part: `gate.py --snapshot` -> implementer (`references/stage1-prompt.md`) -> `gate.py` -> review
    (`references/review-prompt.md`) -> commit -> `workqueue.py mark`
  - up to 5 attempts, then `stubbed` (real signatures, `todo!()` bodies, RUN-NOTES entry)
- [x] Log every batch
  - `make spend PHASE=<phase> SINCE=<timestamp> NOTE=...` and one WORK-LOG entry per batch

## Acceptance

- [x] workqueue.py verify passes
- [x] gate pass recorded for every done Part
- [x] SPEND.md has the batch rows

-----------
## Done

- src/tinyjson ported, 5 attempts. Part 558bb65, ledger 911ef8e, Judge b9d53af, rulebook 619b412.
- Re-gate PASS (check, test, harness): 22/22 tests, 0/5755 corpus diffs vs g++-16.
- Tier split: Sonnet scanners right first time; Haiku body needed fixes, written by the governor.
- Review found 4 behaviour defects the Judge could not see (P2 cast, D13 leak, Q5 key, D4).
- Not shown: red run of the new inputs on the pre-review Part (permission classifier denied it).
