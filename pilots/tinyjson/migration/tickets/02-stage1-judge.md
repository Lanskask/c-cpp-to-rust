# 02: Stage 1 Judge (tinyjson)

**What to build:** The acceptance checks for Same behaviour exist and are shown able to fail (judge.md §1-§4, §6).

**Blocked by:** 01 Mapping (Stage 0)

**Status:** in_progress

## Tasks

- [ ] Write the driver and the corpus generator
  - one canonical dump of every observable, in every API mode (`references/judge.md` §2)
  - `make_inputs.py` with a fixed seed writes the git-ignored corpus; drop non-deterministic inputs (§3)
- [ ] Write run.py for every feature set
  - build the staticlib, link the original tests (C only), build the driver twice, run `harness.py --orig --rust
    --inputs --out` (§4)
  - outputs go under `migration/harness/work/`
  - the crate does not exist yet: validate on a scratch crate in `<ws>/experiments/scratch-crate/` (§4a)
- [ ] Prove the Judge can fail
  - plant a one-character mutant in the scratch crate, run, restore (§6)
  - write `<ws>/experiments/e<N>_<pilot>_judge_sensitivity.py`, docstring: which piece caught it
  - time the first run and record the numbers in RUN-NOTES.md

## Acceptance

- [ ] original tests link to the Rust crate (C only)
- [ ] driver and deterministic corpus per feature set
- [ ] a planted mutant makes the Judge fail in every feature set
- [ ] numbers recorded in RUN-NOTES.md
