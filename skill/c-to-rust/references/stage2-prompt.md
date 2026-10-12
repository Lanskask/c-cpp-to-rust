# Stage 2 prompt (Idiomatic pass, one Part)

Prerequisites, checked by the governor before the first Stage 2 Part (SKILL.md step 4):
rulebook §6 (Stage 2 API) is filled; the Stage 2 Judge (adapter, Rust driver, translated tests) is
committed and validated on the Faithful port, including the planted-mutant check (`judge.md` §5).

Slots (same sources as `stage1-prompt.md` unless stated):

| Slot                         | Source                                                                                           |
|------------------------------|--------------------------------------------------------------------------------------------------|
| `{part_id}`, `{target_path}` | `workqueue.py --parts migration/parts-stage2.tsv next`                                           |
| `{faithful}`                 | the Part's Faithful-port Rust at its Stage 1 commit                                              |
| `{source}`                   | the original C source of the Part (for intent; behaviour is defined by `{faithful}` + the Judge) |
| `{rulebook}`                 | `RULEBOOK.md` §1, §2, §4, §6, §7, §8, §9, §10, §11                                               |
| `{idiom_rows}`               | matched idiom rows, Stage 2 column                                                               |
| `{adapter}`                  | `rust/tests/adapter/mod.rs` (Stage 1 version)                                                    |
| `{judge}`                    | one paragraph: what the driver prints, which modes it runs, the feature matrix, corpus size      |
| `{previous}`, `{tm_hits}`    | as in Stage 1                                                                                    |

---

```text
You are rewriting one Part of a Rust crate from a Faithful port of C into idiomatic Rust.
Stage 2: Idiomatic pass.

Part: {part_id}   files you may write: {target_path} and rust/tests/adapter/mod.rs

GOAL
Idiomatic, safe Rust with the public API in rulebook §6 and Same behaviour: for every input and every
feature set, the adapter must report exactly the values the Faithful port reports today (return codes,
parser/object state, every field of every output record), including on error paths.

ACCEPTANCE (the gate checks all of it, per feature set)
- zero `unsafe` under rust/src/;
- cargo clippy -- -D warnings clean; cargo fmt --check clean;
- no unwrap()/expect() on input-driven paths; elsewhere only with a one-line invariant comment;
- translated tests and the Differential harness pass unchanged.

HARD RULES
1. Public API exactly as rulebook §6. Do not add public items beyond it without a rulebook amendment.
2. Every sentinel becomes the type §6 names (Option, Result, enum). The adapter converts back to the C
   values; that conversion is the only place C sentinels appear.
3. Kept quirks (§9) stay, each site marked QUIRK(port): Q<n>. Deviations (§8) are implemented exactly
   as their "Rust behaviour" column says, each site marked DEVIATION(port): D<n>. Anything else that
   would change an observable value is a bug in your rewrite.
4. Cargo features stay compile-time features, guarded where the Faithful port guards them.
5. Change rust/tests/adapter/mod.rs only to call the new API and convert its results to the adapter's
   existing types. Its public types and function signatures stay identical. Every other test, the
   driver, the harness, the rulebook and parts files are read-only for you.
6. Inner loop: cargo check / cargo test --manifest-path <crate>/Cargo.toml --features <set> for each
   feature set. Acceptance is decided by the gate.
7. End {target_path} with the PORT STATUS trailer (stage=2).
8. A private helper your rewrite needs that the Faithful port did not have is yours: add it to {target_path} and
   implement it fully; never leave it as `todo!()`.

RULEBOOK
{rulebook}

IDIOM ROWS (Stage 2 column)
{idiom_rows}

JUDGE
{judge}

CURRENT ADAPTER
{adapter}

FAITHFUL PORT (behaviour reference)
{faithful}

ORIGINAL C (intent only)
{source}

PREVIOUS ATTEMPT
{previous}

SIMILAR PAST FIXES
{tm_hits}

When done, reply with three lines only:
STATUS: done | blocked
QUIRKS/DEVIATIONS: <ids marked in code>
NOTE: <one line>
```

---

## Governor notes

- Queue: Stage 2 uses `migration/parts-stage2.tsv` (copy of `parts.tsv` with every Part `todo`,
  `attempts=0`, `commit` empty), so Stage 1 history stays intact.
- Snapshot with `--stage 2` before each attempt; gate with `make pilot-<name> STAGE=2` (or `gate.py
  --stage 2 --harness-cmd ".../run.py --stage 2 {features}"`).
- An attempt may change two paths (target + adapter); rollback must restore both. `gate.py` restores
  only `target_path` today (review gap G1 in `docs/E3-SCRIPT-REVIEW.md`): close that gap before the first
  Stage 2 attempt.
- After a pass, run the review (`review-prompt.md`) **before** committing; the commit includes the target
  and the adapter only.
