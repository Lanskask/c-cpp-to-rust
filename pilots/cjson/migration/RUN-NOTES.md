# RUN-NOTES: cJSON

## Stage 1 Judge validation (ticket 02, 2026-10-10)

Everything below was measured by running `pilots/cjson/migration/harness/run.py --scratch [FEATURES]` on macOS
aarch64 (clang from Xcode, rustc stable, c2rust 0.22.1). The Judge was validated on SCRATCH crates produced by
`harness/make_scratch.py` (one c2rust run per feature set, under `harness/build/scratch/`, git-ignored). They are not
the accepted target `pilots/cjson/rust/`, which does not exist yet (ticket 04).

### Files

| Path                                                  | What                                                                               |
|-------------------------------------------------------|------------------------------------------------------------------------------------|
| `harness/run.py [--crate DIR \| --scratch] [FEATURES]` | the Judge, exit 0 = pass; `--skip-tests` runs the Differential harness alone       |
| `harness/make_scratch.py`                              | c2rust per feature set and the hand tweaks below                                   |
| `harness/shim/common.h`                                | replaces upstream `tests/common.h` (public headers only, internals as `extern`)    |
| `harness/dump.c`                                       | the canonical dump driver (parse, print, lookups, hooks, Utils, locale, constructors) |
| `harness/make_inputs.py`                               | seeded corpus generator (seed 20261010), writes `harness/inputs/` (git-ignored)    |
| `harness/e4_cjson_judge_sensitivity.py`                | mutation experiment (see "Judge sensitivity"; location note below)                 |

### Upstream tests, per file and feature set

Both columns are `tests / failures / ignored`. The C original is the upstream test built with the real `common.h`
(it includes `cJSON.c`); the Rust column is the same test source with the shim, linked to the staticlib built with
`test-internals`. All 21 test programs linked; no test is C-original-only.

| Test file          | `""` C original | `""` Rust | `locales` C original | `locales` Rust |
|--------------------|-----------------|-----------|----------------------|----------------|
| `parse_examples`   | 15 / 0 / 0      | 15 / 0 / 0 | 15 / 0 / 0           | 15 / 0 / 0     |
| `parse_number`     | 6 / 0 / 0       | 6 / 0 / 0  | 6 / 0 / 0            | 6 / 0 / 0      |
| `parse_hex4`       | 2 / 0 / 0       | 2 / 0 / 0  | 2 / 0 / 0            | 2 / 0 / 0      |
| `parse_string`     | 6 / 0 / 0       | 6 / 0 / 0  | 6 / 0 / 0            | 6 / 0 / 0      |
| `parse_array`      | 4 / 0 / 0       | 4 / 0 / 0  | 4 / 0 / 0            | 4 / 0 / 0      |
| `parse_object`     | 4 / 0 / 0       | 4 / 0 / 0  | 4 / 0 / 0            | 4 / 0 / 0      |
| `parse_value`      | 7 / 0 / 0       | 7 / 0 / 0  | 7 / 0 / 0            | 7 / 0 / 0      |
| `print_string`     | 3 / 0 / 0       | 3 / 0 / 0  | 3 / 0 / 0            | 3 / 0 / 0      |
| `print_number`     | 6 / 0 / 1       | 6 / 0 / 1  | 6 / 0 / 1            | 6 / 0 / 1      |
| `print_array`      | 3 / 0 / 0       | 3 / 0 / 0  | 3 / 0 / 0            | 3 / 0 / 0      |
| `print_object`     | 3 / 0 / 0       | 3 / 0 / 0  | 3 / 0 / 0            | 3 / 0 / 0      |
| `print_value`      | 7 / 0 / 0       | 7 / 0 / 0  | 7 / 0 / 0            | 7 / 0 / 0      |
| `misc_tests`       | 30 / 0 / 0      | 30 / 0 / 0 | 30 / 0 / 0           | 30 / 0 / 0     |
| `parse_with_opts`  | 6 / 0 / 0       | 6 / 0 / 0  | 6 / 0 / 0            | 6 / 0 / 0      |
| `compare_tests`    | 10 / 0 / 0      | 10 / 0 / 0 | 10 / 0 / 0           | 10 / 0 / 0     |
| `cjson_add`        | 31 / 0 / 0      | 31 / 0 / 0 | 31 / 0 / 0           | 31 / 0 / 0     |
| `readme_examples`  | 3 / 0 / 0       | 3 / 0 / 0  | 3 / 0 / 0            | 3 / 0 / 0      |
| `minify_tests`     | 7 / 0 / 0       | 7 / 0 / 0  | 7 / 0 / 0            | 7 / 0 / 0      |
| `json_patch_tests` | 3 / 0 / 0       | 3 / 0 / 0  | 3 / 0 / 0            | 3 / 0 / 0      |
| `old_utils_tests`  | 6 / 0 / 0       | 6 / 0 / 0  | 6 / 0 / 0            | 6 / 0 / 0      |
| `misc_utils_tests` | 1 / 0 / 0       | 1 / 0 / 0  | 1 / 0 / 0            | 1 / 0 / 0      |
| total              | 163 / 0 / 1     | 163 / 0 / 1 | 163 / 0 / 1         | 163 / 0 / 1    |

The one ignored test is upstream's (`print_number.c`, `TEST_IGNORE()` at `print_number.c:105`).

### Differential harness

| Feature set | Inputs | Match | DIFF | Unstable (dropped) |
|-------------|--------|-------|------|--------------------|
| `""`        | 3849   | 3849  | 0    | 0                  |
| `locales`   | 3849   | 3849  | 0    | 0                  |

`harness.py --repeat 2` runs the original driver twice per input; no input was unstable, so none was dropped as
non-deterministic.

Corpus by group (`make_inputs.py`): valid 61, broken 99, prefix 74, byte 2816 (11 syntactic positions x 256 byte
values: top level, in a string, after a backslash, between number characters, in a key, before a value, as a `\u`
hex digit, inside `true`, in a patch path, as a case-insensitive key, after `-`), mutation 400, random 160 (120
document pairs, 40 identical pairs), patch 77, upstream data 142 (21 `tests/inputs/*`, 121 cases from
`json-patch-tests/*.json`), judge 20. Total 3849.

User-approved Judge cases and where they are exercised by `dump.c`:

| Case | Where                                                                                                            |
|------|------------------------------------------------------------------------------------------------------------------|
| D3   | locale pass: `LC_NUMERIC` set to the first comma locale the OS has; parse and print of `1.5`, arrays, the input  |
| D6   | constructors: `cJSON_CreateNumber(NAN)`, `cJSON_SetNumberHelper(NAN)`, Inf, -Inf                                  |
| D15  | every parsed document is printed with `cJSON_PrintBuffered(item, 0, fmt)` for both `fmt` values                  |
| D19  | locale pass: `LC_CTYPE` set to a Latin-1 locale; keys with bytes >= 0x80 looked up, compared and sorted           |
| Q15  | `judge-q15-*` inputs; every input also goes through `cJSON_Minify`                                                |
| Q20  | `judge-q20-*` and `patch-*` inputs with `~0` / `~1` paths                                                         |

- D3: this OS has comma locales (`locale -a`). The driver picked `de_DE.UTF-8` (decimal point `,`). With
  `ENABLE_LOCALES` the parse of `1.5` works and prints `1.5`; without it `1.5` parses as `1` and
  `cJSON_CreateNumber(1.5)` prints `1,5`. Both programs agree in both feature sets (0 DIFF).
- D6: on this machine (aarch64, clang -O0) the C original also prints `vi=0` for `(int)NaN`; the dump line is
  `NaN vi=0`. The comparison with C is therefore a real equality here, but C leaves it undefined (RULEBOOK D6), so
  treat a future difference on another platform as informational.
- D19: the driver picked `de_DE.ISO8859-1` (`tolower(0xC9) = 233`). The C original and the crate agree.
- `cJSON_Compare` on objects is exponential in nesting depth (it recurses a to b and b to a at every level): a
  999-deep object did not finish in 100 s with the C original. The driver therefore calls `cJSON_Compare` only when
  the tree is at most 14 deep and prints `compare skipped`. This is upstream behaviour, not a port defect; it is
  not in the RULEBOOK (candidate for a new quirk row, deliberately not edited here).

### Judge sensitivity

`harness/e4_cjson_judge_sensitivity.py` plants one one-character mutant in the scratch crate source, runs the full
Judge, restores the file, and then runs the Differential harness alone (`--skip-tests`). Each mutant changes one
integer literal inside one function. All 8 mutants were detected (exit 1). "Tests" is the first piece to go red.

| Feature set | Mutant (C site)                                                  | Upstream tests caught it                    | Harness alone caught it |
|-------------|------------------------------------------------------------------|---------------------------------------------|-------------------------|
| `""`        | whitespace `<= 32` to `<= 31` in `buffer_skip_whitespace`        | yes: 8 files, 24 failures                   | 43 of 3849 DIFF         |
| `""`        | `print_string_ptr` escape `< 32` to `< 31`                       | yes: `print_string` (1 failure)             | 3849 of 3849 DIFF       |
| `""`        | array index digit `'9'` to `'8'` in `decode_array_index_from_pointer` | **no**, 0 failures                      | 2 of 3849 DIFF          |
| `""`        | hex digit `'9'` to `'8'` in `parse_hex4`                         | yes: `parse_hex4` (1 failure)               | 2 of 3849 DIFF          |
| `locales`   | whitespace `<= 32` to `<= 31`                                    | yes: 8 files, 24 failures                   | 43 of 3849 DIFF         |
| `locales`   | array index digit `'9'` to `'8'`                                 | **no**, 0 failures                          | 2 of 3849 DIFF          |
| `locales`   | `46 =>` to `47 =>` ('.' accepted in a number) in `parse_number`  | yes: 5 files, 6 failures                    | 3849 of 3849 DIFF       |
| `locales`   | `decimal_point[0]` to `decimal_point[1]` in `get_decimal_point`  | yes: 5 files, 6 failures                    | 3849 of 3849 DIFF       |

Reading: the array-index mutant is invisible to all 21 upstream test programs and is caught only by the Differential
harness, and only by 2 inputs (`valid-043`, a 14-element array, and `valid-051`, a 300-element array: the pointers `/9`
and above that `pointers()` in `dump.c` looks up). That is thin; a future corpus extension should add pointer
round-trips for every index digit in every position. No mutant survived, so no corpus extension was forced. Whole-corpus
DIFF counts of 3849 come from the fixed constructors section, which prints in every run.

### c2rust workarounds and hand tweaks (scratch crates)

| id | Where                      | What                                                                                                    |
|----|----------------------------|---------------------------------------------------------------------------------------------------------|
| W1 | compile DB                 | named exactly `compile_commands.json`, one entry per `.c` file (decision 23 workaround 1)               |
| W2 | output dir                 | `cjson_c2rust`, never `crate` (decision 23 workaround 2)                                                |
| W3 | toolchain                  | not needed: after T1 to T6 the crate builds with stable cargo; no `cargo +<pin>` (decision 25 unused)   |
| T1 | `lib.rs`, `Cargo.toml`     | rewritten: c2rust's `#![feature(linkage)]`, `#![feature(raw_ref_op)]`, `extern crate f128`, build.rs dropped; features `locales` (no-op) and `test-internals` added; `crate-type = ["staticlib", "rlib"]` |
| T2 | `cJSON.rs`                 | `use ::f128;` removed; `::f128::f128` replaced by `f64` (2 sites) and `::f128::f128::new(x)` by `x` (2 sites). macOS `isnan`/`isinf` expand to `__inline_isnanl(long double)` and on aarch64 macOS `long double` is `f64` |
| T3 | `cJSON.rs`                 | `test-internals`: 27 static functions and 2 statics (`global_hooks`, `global_error`) become `pub` with `#[cfg_attr(feature = "test-internals", no_mangle)]` |
| T5 | `cJSON.rs`, `cJSON_Utils.rs` | one `#[linkage = "external"]` removed per file (nightly-only attribute)                               |
| T6 | `cJSON.rs`, `cJSON_Utils.rs` | `#[no_mangle]` removed from the inline libc helper `tolower` in each file: otherwise both modules define the symbol `tolower` (duplicate symbol) and it would shadow libc |

Counts are identical for both feature sets (c2rust output: 4851 Rust lines for `""`, 4883 for `locales`).
Not applied: any change to function bodies. The behaviour under test is c2rust's own translation.

### Linking the upstream tests (shim)

`harness/shim/common.h` copies the typedefs `internal_hooks`, `parse_buffer`, `printbuffer`, the macros
`can_read`/`can_access_at_index`/`buffer_at_offset`/`static_strlen`, and declares as `extern` the 18 private
functions and `global_hooks` that the tests use (listed in the file), plus upstream's `reset`, `read_file` and the
`assert_*` macros. All 12 internals-using tests linked against the `test-internals` build. The driver is built from the
public headers only and linked to the build without `test-internals`.

### Not done or not run

- The accepted crate `pilots/cjson/rust/` does not exist; `run.py` without `--scratch` has not been run.
- Stage 2 Judge (Rust driver, translated tests, adapter): not part of this ticket.
- Allocation-failure hooks (`cjson_add`, `ensure_should_fail_on_failed_realloc`) ran as upstream tests (inside the
  163); the driver adds a fail-the-Nth-allocation sweep capped at 48 allocations per operation (Parse, Print,
  PrintBuffered, Duplicate). Deeper allocation indices are not swept.
- Only the first 12 root children get pointer round-trips (`pointers()` in `dump.c`).
- Windows, MSVC, Sun: untested (RULEBOOK D18).
- Location note: `e4_cjson_judge_sensitivity.py` should live in `pilots/cjson/experiments/`; writing there was
  refused by a permission rule, so it sits in `harness/` and needs a `git mv` by the governor.
