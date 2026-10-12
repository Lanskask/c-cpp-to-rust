# 04: Stage 1 Faithful port (tinyjson)

**What to build:** Every Part is done or stubbed, each accepted by the Gate and committed.

**Blocked by:** 03 Rulebook stress test

**Status:** in_progress

## Tasks

- [x] Protect the Rulebook and the Judge, then start the fan-out
  - session started with `claude --settings skill/c-to-rust/assets/settings.json` (SKILL.md step 3.1)
  - `tickets.py mark 04 in_progress`, then an `Edit` of RULEBOOK.md must be refused; if not, stop
  - write `rust/src/lib.rs` (`pub mod <stem>;` per Part) and the cargo features of rulebook §3
- [ ] Port every Part in batches
  - `workqueue.py --parts <ws>/migration/parts.tsv next --batch N`
  - per Part: `gate.py --snapshot` -> implementer (`references/stage1-prompt.md`) -> `gate.py` -> review
    (`references/review-prompt.md`) -> commit -> `workqueue.py mark`
  - up to 5 attempts, then `stubbed` (real signatures, `todo!()` bodies, RUN-NOTES entry)
- [x] Log every batch
  - `make spend PHASE=<phase> SINCE=<timestamp> NOTE=...` and one WORK-LOG entry per batch

## Acceptance

- [ ] workqueue.py verify passes
- [ ] gate pass recorded for every done Part
- [ ] SPEND.md has the batch rows
