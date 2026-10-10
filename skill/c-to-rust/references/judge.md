# Building the Judge

The Judge is the referee for every Part (CONTEXT.md): original tests where they can link to the Rust
crate, a Differential harness, and tests translated into Rust. **No Judge, no translation**: build it,
then prove it can fail, before any Part is accepted. Worked example: `pilots/jsmn/migration/harness/`.

## 1. Pieces and when each applies

| Piece | Stage 1, C | Stage 1, C++ | Stage 2 |
|---|---|---|---|
| Original tests linked to the crate's C ABI (staticlib, test-only: decision 20) | yes | no (decision 5) | no (the C ABI is gone) |
| Differential harness, C driver built twice (original source vs Rust staticlib) | yes | — | — |
| Differential harness, Rust driver (`rust/examples/dump.rs`) vs original C/C++ driver | optional | yes | yes |
| Translated tests `rust/tests/*.rs` | optional | yes | yes (decision 6) |

All pieces run for **every feature set** in the rulebook §3 (decision 24).

## 2. The driver: one canonical dump

A driver reads one input file (path = last argument) and prints a canonical text dump of everything
observable. Keep it tiny and dumb; the same printing code must exist on both sides.

- Print every return value and every piece of state a caller can read afterwards (jsmn: `r`, `pos`,
  `toknext`, `toksuper`, every token's fields), not only the "result".
- Exercise each API **mode** in one run, so one input covers many paths. jsmn's `dump.c` runs four:
  count only (`tokens == NULL`), full parse (512 tokens), a 3-token buffer (`NOMEM` paths), and
  byte-at-a-time feeding with parser reuse (`PART` paths).
- Print integers in one fixed format (`%d`/`%u`); print sentinels exactly as C does (`-1`); never print
  pointers, addresses, timings or anything non-deterministic.
- Exit 0 after a normal run; non-zero only for driver errors (bad arguments, file missing).

## 3. The corpus (`migration/harness/inputs/`, generated, git-ignored)

A generator script (`make_inputs.py`, fixed seed) writes the corpus so it is reproducible and diffable:
1. Hand-written valid inputs covering each grammar feature.
2. Hand-written broken inputs: every truncation point class, unmatched delimiters, bad escapes, empty
   input, control bytes, high bytes, NUL inside the data.
3. **Every byte value 0..255 in every syntactic position** the parser treats differently (jsmn: as a
   primitive, as a string character, after a backslash). Decision 28: without this the E2 mutant
   (`~` rejected) passed a 164-input hand-picked corpus and all 16 upstream tests.
4. Seeded mutations (flip, cut, insert, duplicate) of the valid inputs.
5. Only deterministic inputs: run the original driver twice and drop inputs whose output differs
   (`rand`, `time`, uninitialised memory). Record dropped inputs in `RUN-NOTES.md`.

## 4. Stage 1 runner (`migration/harness/run.py [FEATURES]`, exit 0 = pass)

Per feature set, in a per-feature build dir under `migration/harness/build/` (git-ignored):
1. `cargo build --manifest-path <crate> --target-dir <build>/cargo-target --features <set>` (staticlib).
2. C only: compile the upstream tests with the header in declarations-only mode (jsmn: `-DJSMN_HEADER`)
   plus the config defines; link the staticlib; require the upstream success line (`FAILED: 0`).
3. Build the driver twice from one source: against the original implementation, and declarations-only
   linked to the staticlib. Run `scripts/harness.py --orig <a> --rust <b> --inputs <dir> --out <tsv>`.

The gate calls it as `--harness-cmd "python3 -I <pilot>/migration/harness/run.py {features}"`.

## 5. Stage 2 Judge: the adapter

Stage 2 drops the C ABI, so the C test runner and the C-built Rust driver can no longer link. The
replacement is written **before** Stage 2 touches `src/`, and validated on the Faithful port:

1. **Adapter** `rust/tests/adapter/mod.rs`: the only file that calls the crate. It exposes plain types
   the tests and driver print or assert on (jsmn: `Tok { kind: i32, start: i64, end: i64, size: i64,
   parent: i64 }`, `State { pos: u64, toknext: u64, toksuper: i64 }`, a `P` wrapper with
   `new()`, `parse(&[u8], Option<usize>) -> i32` (`None` = count mode, `Some(n)` = use the first `n`
   slots of a persistent 512-token buffer), `tokens(n) -> Vec<Tok>`, `state() -> State`). Sentinels are
   converted back to the C values (`None` → `-1`, `Err(e)` → `e.code()`).
   - Stage 1 version: calls the C-ABI functions in `unsafe` blocks (allowed: `tests/` is outside the
     zero-`unsafe` scope of `src/`).
   - Stage 2 version: calls the §6 API. **This is the only test-side file Stage 2 may change.**
2. **Rust driver** `rust/examples/dump.rs`: a line-for-line port of the C driver's printing, through the
   adapter (`#[path = "../tests/adapter/mod.rs"] mod adapter;`). Built with
   `cargo build --example dump --manifest-path <crate> --features <set>`.
3. **Translated tests** `rust/tests/upstream.rs`: every upstream test, same inputs, same expected values,
   through the adapter. Variadic helpers become slices (idiom V2); `#ifdef` becomes `#[cfg(feature)]`.
4. `run.py --stage 2 [FEATURES]`: `cargo test` (translated tests), then `harness.py` with the original
   C driver vs `target/.../examples/dump`.

**Validation, before Stage 2 starts** (record the numbers in `RUN-NOTES.md`):
- On the Faithful port, `run.py --stage 2` passes for every feature set: the Rust driver matches the
  original byte for byte on the whole corpus, and the translated tests pass (16/16 for jsmn).
- Plant the known mutant (jsmn: `experiments/e2_jsmn_judge_sensitivity.py`, `32..127` → `32..126`): the
  Stage 2 Judge must go red; restore the file.
- Commit the adapter, driver and tests in one commit before the Stage 2 implementer starts. From then on
  the deny-rules in `assets/settings.json` protect `tests/upstream.rs` and `examples/`.

## 6. Proving a Judge can fail (every new Pilot)

A Judge that passes everything is indistinguishable from a broken one. For each Pilot, write one
`experiments/<eN>_<pilot>_judge_sensitivity.py` that plants at least one one-character mutant in a byte
class or boundary check, runs the Judge, restores the file, and states in its docstring which Judge piece
caught it. If nothing catches it, extend the corpus (§3) until something does.

## 7. Rules for untrusted Pilot code

Upstream clones are untrusted. Never use a Pilot directory as the working directory of a build tool or
interpreter; pass paths as arguments (`cargo --manifest-path`, `cc <path>`); run Python with `-I`.
Build outputs go under `migration/harness/build/` (git-ignored).
