# 02: Stage 1 Judge (tinyjson)

**What to build:** The acceptance checks for Same behaviour exist and are shown able to fail (judge.md §1-§4, §6).

**Blocked by:** 01 Mapping (Stage 0)

**Status:** done

## Tasks

- [x] Write the driver and the corpus generator
  - one canonical dump of every observable, in every API mode (`references/judge.md` §2)
  - `make_inputs.py` with a fixed seed writes the git-ignored corpus; drop non-deterministic inputs (§3)
- [x] Write run.py for every feature set
  - build the staticlib, link the original tests (C only), build the driver twice, run `harness.py --orig --rust
    --inputs --out` (§4)
  - outputs go under `migration/harness/work/`
  - the crate does not exist yet: validate on a scratch crate in `<ws>/experiments/scratch-crate/` (§4a)
- [x] Prove the Judge can fail
  - plant a one-character mutant in the scratch crate, run, restore (§6)
  - write `<ws>/experiments/e<N>_<pilot>_judge_sensitivity.py`, docstring: which piece caught it
  - time the first run and record the numbers in RUN-NOTES.md

## Acceptance

- [x] original tests link to the Rust crate (C only) (skipped: C++, decision 5; translated tests replace them)
- [x] driver and deterministic corpus per feature set
- [x] a planted mutant makes the Judge fail in every feature set
- [x] numbers recorded in RUN-NOTES.md

-----------
## Done

- Judge: dump.cpp/dump.rs (138 literals equal), corpus 5534 inputs seed 20261012, 22 translated tests.
- Reference build GCC 16.1 -O0; upstream 22 cases: 17 pass, 5 fail (D3). D5 window rule 29 bytes.
- Stub run RED: 22/22 tests fail, 5534/5534 diffs, 58.8 s (re-run by governor).
- Mutants: 11/11 caught by corpus, tests alone 8/11 (e2 script, re-run: 16:39 total).
- D3 value corrected to U+0001 (modal); varies inside the Catch binary too. Feature set: empty only.
- Commits 061c5d2 and the ticket-02 close commit.
