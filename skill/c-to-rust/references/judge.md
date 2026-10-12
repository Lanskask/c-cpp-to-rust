# Building the Judge

The Judge decides Same behaviour for every Part (CONTEXT.md): original tests where they can link to the Rust
crate, a Differential harness, and tests translated into Rust. **No Judge, no translation**: build it,
then prove it can fail, before any Part is accepted. Worked example: `pilots/jsmn/migration/harness/`.

## 1. Pieces and when each applies

| Piece                                                                                | Stage 1, C | Stage 1, C++    | Stage 2                |
|--------------------------------------------------------------------------------------|------------|-----------------|------------------------|
| Original tests linked to the crate's C ABI (staticlib, test-only: decision 20)       | yes        | no (decision 5) | no (the C ABI is gone) |
| Differential harness, C driver built twice (original source vs Rust staticlib)       | yes        | —               | —                      |
| Differential harness, Rust driver (`rust/examples/dump.rs`) vs original C/C++ driver | optional   | yes             | yes                    |
| Translated tests `rust/tests/*.rs`                                                   | optional   | yes             | yes (decision 6)       |

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
   primitive, as a string character, after a backslash). Decision 28: without this the E2 mutant (`~` rejected) passed a
   164-input hand-picked corpus and all 16 upstream tests.
4. Seeded mutations (flip, cut, insert, duplicate) of the valid inputs.
5. **Every narrowing site of the original** (`as u8`, `(char)`, `(unsigned char)`, a `switch` on a narrowed value): an
   input with a code point above 0xFF (and a NUL, and a byte 0x80..0xFF) at that exact position, not only at the
   position where the first such bug was found. A corpus that has the case in one place does not cover the same cast
   at another (tinyjson ticket 04: `bool`/`null` tokens covered, the same cast at the value start was not; the review
   found four defects the corpus could not see). Add the gap before the Part is first accepted, not after.
6. Only deterministic inputs: run the original driver twice and drop inputs whose output differs (`rand`, `time`,
   uninitialised memory). Record dropped inputs in `RUN-NOTES.md`.

## 4. Stage 1 runner (`migration/harness/run.py [FEATURES]`, exit 0 = pass)

Per feature set, in a per-feature work dir under `migration/harness/work/` (git-ignored):

1. `cargo build --manifest-path <crate> --target-dir <build>/cargo-target --features <set>` (staticlib).
2. C only: compile the upstream tests with the header in declarations-only mode (jsmn: `-DJSMN_HEADER`)
   plus the config defines; link the staticlib; require the upstream success line (`FAILED: 0`).
3. Build the driver twice from one source: against the original implementation, and declarations-only
   linked to the staticlib. Run `scripts/harness.py --orig <a> --rust <b> --inputs <dir> --out <tsv>`.

The gate calls it as `--harness-cmd "python3 -I <pilot>/migration/harness/run.py {features}"`.

## 4a. Validating the Stage 1 Judge before the crate exists

Ticket 02 builds the Stage 1 Judge, but its crate is the output of ticket 04, which 02 blocks. The Judge needs
something Rust-shaped to run green against, and a planted mutant to run red against. Use a **scratch crate** in
`<ws>/experiments/scratch-crate/` (never committed as the target; ticket 04's done-commit must still change the
real crate):

- **C, engine c2rust:** a raw `c2rust transpile` per feature set (config defines on the command line), unpolished,
  one crate per set.
- **C, no c2rust:** one hand-merged minimal port of a slice big enough to exercise the driver.
- **C++:** a stub crate with real signatures and `todo!()` bodies. Original tests cannot link (decision 5).

Acceptance for the Judge (ticket 02):

1. Green on the raw transpile in **every** feature set (C routes only). A red run here is a Judge bug or a c2rust
   defect; find out which before going on, and record it in RUN-NOTES.
2. Red on a planted mutant of the scratch crate (§6).
3. Stub route: red on the stub, and the runner reports a diff, not a build error.

The scratch crate proves the Judge, not the port. Ticket 04's Gate runs the same Judge against the real crate.

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

**Validating the adapter before the Stage 2 crate exists** (tinyjson ticket 05). The validation above assumes the
Stage 2 adapter can run against the Faithful port, which holds only when Stage 1 already exposes the §6 names. For a
C++ Pilot, or any Pilot whose Stage 1 names differ from §6, it cannot compile. Write a validation shim:

- `stage2_shim.rs` implements the whole §6 API (same names, signatures, derives) on top of the Stage 1 crate. The
  Stage 2 adapter starts with `mod stage2_shim; use stage2_shim as <crate>;`. `run.py --stage 2` installs the adapter
  and the shim; Stage 1 runs delete the shim copy.
- When the Stage 2 crate lands, those two lines become `use <crate> as <alias>;` and the shim is deleted (the
  adapter stays the only test-side file Stage 2 may change).
- The shim has no `unsafe`, and every method calls the Stage 1 function and takes Ok/Err and the error text from it,
  so the verdict stays the Faithful port's. Where §6 returns a borrow and Stage 1 returns a clone, hold the data in
  the §6 value and convert one level for scalar getters: a fully converting shim made a depth-400 corpus input time
  out.
- Validate as above: green on the Faithful port at both stages, red on a planted mutant, and record which Judge piece
  caught it. In tinyjson the D5 window mutant (29 -> 30) passed 22/22 translated tests and was caught only by 13
  corpus diffs.

## 5a. What the corpus never feeds (Stage 2 review lesson, cJSON)

A corpus of parsed files only reaches states the parser builds. The cJSON Stage 2 review found five BLOCKs on paths
the Judge never fed (WORK-LOG 39). For every Pilot, list each class below in RUN-NOTES as covered (where) or
not testable (why), before the Stage 2 implementer starts:

| Class                           | Why the corpus misses it                          | Cover with                                           |
|---------------------------------|---------------------------------------------------|------------------------------------------------------|
| Allocation failure              | the driver's allocator never fails                | allocator hooks: counting trace, then fail-the-Nth   |
| Absent members of built values  | the parser never makes a NULL string or key       | translated tests that build the value through the API |
| Input without a trailing NUL    | the adapter appends a NUL, so C and Rust both stop | an adapter entry that passes the slice as it is      |

Where C writes through NULL (UB) the abort cannot be compared: record it as a Deviation row and pin only the
C-defined neighbours. Run the allocator probes on a subset of inputs and time them (§6).

## 6. Proving a Judge can fail (every new Pilot)

A Judge that passes everything is indistinguishable from a broken one. For each Pilot, write one
`<ws>/experiments/<eN>_<pilot>_judge_sensitivity.py` (decision 45) that plants at least one one-character mutant
in a byte class or boundary check, runs the Judge, restores the file, and states in its docstring which Judge
piece caught it. If nothing catches it, extend the corpus (§3) until something does.

### Keeping Judge runs cheap

Measured on cJSON Stage 2 (WORK-LOG 36): one Judge run is about 60 s, of which the corpus differential is about 50 s
and the translated tests plus incremental cargo about 4 s.

- **One run at a time.** Never start a second Judge run or experiment while one is running (8 cores are shared and a
  stray run doubled a subagent's wall time).
- **One feature set per mutant.** Run each mutant on the first feature set; run the others only for a mutant that
  survived there, and for the final all-sets confirmation. A mutant the upstream/translated tests catch never
  reaches the corpus diff.
- **Time the first Gate run of a stage** (`time make ...`) and put the number in RUN-NOTES before planning more runs.
- **A crash is red too.** A translated test that aborts (SIGABRT from an allocator probe) stops the whole test binary,
  so you see a crash, not a list. Confirm the rest with `cargo test -- --skip <names>`.
- Do not run the Judge under PyPy: the corpus step is process spawning, and PyPy was slower (76 s against 50 s).

## 8. C++ Pilots: the reference build (tinyjson lesson)

The C branch assumes the original builds with the default compiler. A C++ original may not (tinyjson's header needs
another standard library and an `-include`, and its upstream test file needs two patches), and its behaviour on
undefined behaviour changes with the optimisation level. So, in ticket 01 and before any Judge piece is written:

1. Build the original (header or sources, plus the upstream test file) with the candidate compiler. On failure try
   the next one and keep the error text: it is the "Why this build" row.
2. Record the result in the RULEBOOK slot `Reference build` (`rulebook-template-cpp.md`): compiler, standard library,
   `-std`, extra flags, optimisation level of the **driver**, and every patch to the upstream tests.
3. A patch to an upstream test may make it compile; it may not change an expected value. A changed expectation is a
   Deviation proposal.
4. Every piece that runs the original (the original driver, the upstream tests, the planted mutant of §6) is built with
   exactly that row. `run.py` prints the row's compiler version first, so a changed toolchain shows in the log.
5. The reference build's results (for example "17 of 22 upstream tests pass") are a **measurement**: run them
   yourself, record the command and the output in RUN-NOTES. A number reported by a subagent is unverified until then.

If no available compiler builds the original, stop and tell the user: there is no reference behaviour to port.

## 7. Rules for untrusted Pilot code

Upstream clones are untrusted. Never use a Pilot directory as the working directory of a build tool or
interpreter; pass paths as arguments (`cargo --manifest-path`, `cc <path>`); run Python with `-I`.
Build outputs go under `migration/harness/work/` (git-ignored).
