# 05: Stage 2 Judge (tinyjson)

**What to build:** The adapter, Rust driver and translated tests pass on the Faithful port and fail on the mutant
(judge.md §5), in their own commit.

**Blocked by:** 04 Stage 1 Faithful port

**Status:** todo

## Tasks

- [ ] Check that rulebook §6 (Stage 2 API) is filled
  - if empty, stop and ask the user for it (Opus writes it)
- [ ] Write the adapter, the Rust driver and the translated tests
  - `rust/tests/adapter/mod.rs` (the only file that calls the crate), `rust/examples/dump.rs`, `rust/tests/upstream.rs`
    (`references/judge.md` §5)
  - `run.py --stage 2` runs `cargo test`, then `harness.py` with the C driver against the Rust one
  - list allocation failure, NULL members and unterminated input as covered or not testable (`references/judge.md` §5a)
- [ ] Validate on the Faithful port and on the mutant
  - green in every feature set on the Faithful port; red with the planted mutant, then restore
  - one commit for the adapter, driver and tests, before the Stage 2 implementer starts
  - copy `parts.tsv` to `parts-stage2.tsv` (all `todo`, attempts 0, no commit)

## Acceptance

- [ ] passes on the Faithful port in every feature set
- [ ] mutant goes red
- [ ] parts-stage2.tsv created
