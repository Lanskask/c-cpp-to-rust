# RUN-NOTES: cJSON

## Stage 1 Judge validation (ticket 02, 2026-10-10)

Everything below was measured by running `pilots/cjson/migration/harness/run.py --scratch [FEATURES]` on macOS
aarch64 (clang from Xcode, rustc stable, c2rust 0.22.1). The Judge was validated on SCRATCH crates produced by
`harness/make_scratch.py` (one c2rust run per feature set, under `harness/work/scratch/`, git-ignored). They are not
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
| `""`        | 3881   | 3881  | 0    | 0                  |
| `locales`   | 3881   | 3881  | 0    | 0                  |

`harness.py --repeat 2` runs the original driver twice per input; no input was unstable, so none was dropped as
non-deterministic.

Corpus by group (`make_inputs.py`): valid 61, broken 99, prefix 74, byte 2816 (11 syntactic positions x 256 byte
values: top level, in a string, after a backslash, between number characters, in a key, before a value, as a `\u`
hex digit, inside `true`, in a patch path, as a case-insensitive key, after `-`), mutation 400, random 160 (120
document pairs, 40 identical pairs), patch 77, upstream data 142 (21 `tests/inputs/*`, 121 cases from
`json-patch-tests/*.json`), judge 52 (20 approved-deviation and quirk cases, 32 array-index cases). Total 3881.

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
| `""`        | array index digit `'9'` to `'8'` in `decode_array_index_from_pointer` | **no**, 0 failures                      | 34 of 3881 DIFF         |
| `""`        | hex digit `'9'` to `'8'` in `parse_hex4`                         | yes: `parse_hex4` (1 failure)               | 2 of 3849 DIFF          |
| `locales`   | whitespace `<= 32` to `<= 31`                                    | yes: 8 files, 24 failures                   | 43 of 3849 DIFF         |
| `locales`   | array index digit `'9'` to `'8'`                                 | **no**, 0 failures                          | 34 of 3881 DIFF         |
| `locales`   | `46 =>` to `47 =>` ('.' accepted in a number) in `parse_number`  | yes: 5 files, 6 failures                    | 3849 of 3849 DIFF       |
| `locales`   | `decimal_point[0]` to `decimal_point[1]` in `get_decimal_point`  | yes: 5 files, 6 failures                    | 3849 of 3849 DIFF       |

Reading: the array-index mutant is invisible to all 21 upstream test programs and is caught only by the Differential
harness. With the first corpus (3849 inputs) only 2 inputs caught it (`valid-043`, `valid-051`). The corpus was then
extended with 32 `judge-idx-*` inputs (a 12-element and a 100-element array, and `replace` patches at every digit in
the units position and digits 1-9 in the tens position); the two array-index rows above were re-run on the 3881-input
corpus: 34 inputs DIFF in each feature set. The other rows were measured on the 3849-input corpus and not re-run; their
whole-corpus DIFF counts of 3849 come from the fixed constructors section, which prints in every run. No mutant
survived.

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

## Stage 2 Judge (ticket 05, 2026-10-10)

Everything below was measured on macOS aarch64 (clang from Xcode, rustc 1.94.1 stable) against the accepted Stage 1
crate `pilots/cjson/rust/` (c2rust output, C ABI). `rust/src`, `Cargo.toml` and the RULEBOOK were not changed (the
`rust/src` diff is empty after every run, including the mutant runs).

### Files

| Path                                               | What                                                                                                      |
|----------------------------------------------------|-----------------------------------------------------------------------------------------------------------|
| `rust/tests/adapter/mod.rs`                        | the only file that calls the crate; Stage 1 adapter over the C ABI; interface contract in its header      |
| `rust/examples/dump.rs`                            | line-for-line port of `harness/dump.c` through the adapter; same bytes on every corpus input              |
| `rust/tests/upstream.rs`                           | the 163 upstream tests translated through the adapter, plus 1 extra test (see below)                      |
| `harness/run.py --stage 2 [FEATURES]`              | `cargo test` (translated tests), then original C driver vs `cargo build --example dump`, 2 runs per input |
| `harness/dump.c`                                   | one change: the call of `hooks_pass` is inside `#ifndef DUMP_NO_HOOKS` (Stage 1 output is unchanged)      |
| `experiments/e5_cjson_stage2_judge_sensitivity.py` | mutants in `rust/src`, Stage 2 Judge, restore (see Judge sensitivity below)                               |
| `parts-stage2.tsv`                                 | both Parts, state `todo`, deps as in `parts.tsv`                                                          |

### Adapter interface

`Item` is a `Copy`, comparable handle for a `cJSON *` (NULL allowed); ownership is manual as in C; strings are byte
slices without the NUL; `None` is NULL; parse results carry the error position and parse end as offsets into the input
(`ErrPos::{Unset, At, Outside}`). Two capability constants let a Stage 2 adapter say what it cannot do without touching
the driver or the tests:

- `HOOKS` (RULEBOOK D1): allocator hooks exist. The 13 `*_should_fail_on_allocation_failure` tests of `cjson_add` print
  `SKIPPED:` and return when it is `false`; `dump.rs` skips its allocator section, and `run.py --stage 2` reads the
  constant from the adapter and builds the C driver with `-DDUMP_NO_HOOKS` so that both sides print the same sections.
- `C_LAYOUT` (D7, D11, D14): the `raw_*` functions work (field writes, corrupted lists, pointer identity of buffers).
  Steps that need them print `SKIPPED:` and are left out when it is `false`.

`run.py --stage 2` runs `cargo test -- --nocapture` and prints the distinct `SKIPPED:` lines, so a Stage 2 run shows
what the adapter did not exercise. With the Stage 1 adapter both constants are `true` and nothing is skipped (measured:
0 skipped step kinds).

### Tests that used private functions or struct fields in C

Twelve upstream files call static functions of `cJSON.c` (RULEBOOK section 10). Rule used for all of them:
translate through the public API when the same behaviour is observable there; otherwise `#[ignore]` with the reason, or
(when only a step is not observable) guard that step with a capability constant. No translated test needs the
`test-internals` feature; `cargo test` runs without it.

| Upstream file (tests)                                    | Private item used                                    | Translation                                                                                                                                                                                      |
|----------------------------------------------------------|------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `parse_number` (6)                                       | `parse_number`, `parse_buffer`                       | public `cJSON_Parse` of the number text; same `valueint` and `valuedouble` assertions, node-shape asserts on the root                                                                            |
| `parse_hex4` (2)                                         | `parse_hex4`                                         | `"\uXXXX"` through `cJSON_Parse`; all 65536 values in both cases (surrogates as pairs with U+DC00 / U+D800; U+0000 gives the empty string); the 16 mixed-case spellings of `beef`                |
| `parse_string` (6)                                       | `parse_string`                                       | `cJSON_Parse` of the quoted text; a malformed string must give NULL or a non-String root (the private function rejects any text not starting with a quote)                                       |
| `parse_array` (4), `parse_object` (4)                    | `parse_array`, `parse_object`                        | `cJSON_Parse`; `assert_not_*` accepts NULL or a root of another kind (the private functions reject by kind)                                                                                      |
| `parse_value` (7)                                        | `parse_value`                                        | `cJSON_Parse`                                                                                                                                                                                    |
| `print_string` (3)                                       | `print_string_ptr`, `printbuffer`                    | `cJSON_CreateString` + `cJSON_PrintUnformatted`; the NULL input is `cJSON_CreateStringReference(NULL)`                                                                                           |
| `print_number` (6)                                       | `print_number`                                       | `cJSON_CreateNumber` + `cJSON_PrintUnformatted`; the MSVC three-digit exponent fix-up loop is not ported (macOS prints two digits, D4, D18); the upstream `TEST_IGNORE()` test stays `#[ignore]` |
| `print_array` (3), `print_object` (3), `print_value` (7) | `parse_*`, `print_*`, `printbuffer`                  | `cJSON_Parse`, then `cJSON_PrintUnformatted` == input and `cJSON_Print` == expected (the private call used a fixed 1024-byte `noalloc` buffer; the text is the same)                             |
| `misc_tests`: `ensure_should_fail_on_failed_realloc`     | `ensure`, `printbuffer`, a `realloc` hook            | `#[ignore]`: `cJSON_InitHooks` cannot install `realloc`, nothing public reaches `ensure` with a failing one (D1)                                                                                 |
| `misc_tests`: `skip_utf8_bom_*` (2)                      | `skip_utf8_bom`, `parse_buffer`, `global_hooks`      | public `cJSON_ParseWithOpts`: BOM then `{}` parses with `parse_end` at 5; a BOM after a space gives NULL (the private `offset = 1` setup is not reachable, the observable effect is the same)    |
| `misc_tests`: `..._string_is_aliased`                    | `cJSON_strdup`, `global_hooks`, `item->string` write | `raw_add_item_with_aliased_key`, `C_LAYOUT` only                                                                                                                                                 |

Tests that did not use private functions but wrote or read node fields on stack-built structs:

| Test                                                                                          | Translation                                                                                                                   |
|-----------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------|
| `cjson_array_foreach_should_loop_over_arrays`                                                 | a parsed array of 10; `children()` equals `array_item(i)` for every `i`                                                       |
| `typecheck_functions_should_check_type`                                                       | items get `StringIsConst` through `AddItemToObjectCS`; the `Invalid` node needs `raw_new_with_type` (`C_LAYOUT`)              |
| `cjson_detach_item_via_pointer_should_detach_items`                                           | four heap nulls in a real array; same assertions on `prev`, `next`, `child`                                                   |
| `cjson_detach_item_via_pointer_should_return_null_if_item_prev_is_null`                       | `C_LAYOUT` only (`raw_set_prev`, D14)                                                                                         |
| `cjson_replace_item_via_pointer_should_replace_items`                                         | heap replacements instead of stack structs; same assertions                                                                   |
| `cjson_replace_item_in_object_should_preserve_name`                                           | root is `cJSON_CreateObject()` instead of a zeroed stack node                                                                 |
| `cjson_functions_should_not_crash_with_null_pointers`                                         | all calls kept; the two assertions that depend on the hand-corrupted list and the freed valuestring run only under `C_LAYOUT` |
| `cjson_set_valuestring_should_return_null_if_strings_overlap`                                 | `C_LAYOUT` only (`raw_set_valuestring_from_own`, D7)                                                                          |
| `cjson_should_not_follow_too_deep_circular_references`                                        | `C_LAYOUT` only (cycle, D11); plus the extra test below                                                                       |
| `cjson_compare_should_compare_invalid_as_not_equal`, `..._not_accept_invalid_types`           | `C_LAYOUT` only (`raw_new_with_type`, D14)                                                                                    |
| `cjson_compare_should_compare_raw`                                                            | `cJSON_CreateRaw` instead of parse then `type = cJSON_Raw` (same node)                                                        |
| `cjson_set_number_value_should_set_numbers`                                                   | `cJSON_CreateNumber(0)` instead of a stack node                                                                               |
| `cjson_create_string_reference_...`, `cjson_set_valuestring_to_object_should_not_leak_memory` | pointer-identity assertions guarded by `C_LAYOUT`; value and type assertions always run                                       |

One test is not upstream: `extra_duplicate_should_stop_at_the_circular_limit_without_a_cycle` (RULEBOOK D11
recommendation): an acyclic array nested `CIRCULAR_LIMIT + 1` deep must not be duplicated. It runs in both Stages.
`upstream.rs` therefore has 164 tests: 162 pass, 2 are `#[ignore]` (the upstream `TEST_IGNORE()` and
`ensure_should_fail_on_failed_realloc`).

### Measured

| Command                           | Feature set | Translated tests (passed / failed / ignored)    | Differential harness   | Exit |
|-----------------------------------|-------------|-------------------------------------------------|------------------------|------|
| `run.py --stage 2`                | `""`        | 162 / 0 / 2                                     | 3881/3881 inputs match | 0    |
| `run.py --stage 2 locales`        | `locales`   | 162 / 0 / 2                                     | 3881/3881 inputs match | 0    |
| `run.py` (Stage 1, rerun)         | `""`        | 163 / 0 / 1 (C upstream tests on the staticlib) | 3881/3881 inputs match | 0    |
| `run.py locales` (Stage 1, rerun) | `locales`   | 163 / 0 / 1                                     | 3881/3881 inputs match | 0    |

A Stage 1 debug build needs far more than the 2 MiB of a test thread: the 1000-deep nesting test overflowed the stack.
The tests that recurse (`cjson_should_not_parse_to_deeply_nested_jsons`, the circular tests) run on a 256 MiB thread
(`on_big_stack`), and `dump.rs` runs its body on a 512 MiB thread (the C driver gets the 8 MiB main stack at `-O0`). The
library keeps process-global state (hooks, error position, D1, D9), so every test takes a mutex; `cargo test` stays
correct with the default thread count.

### Judge sensitivity (Stage 2)

`experiments/e5_cjson_stage2_judge_sensitivity.py` plants one one-character mutant in `rust/src` (one integer literal
inside one function), runs `run.py --stage 2` for both feature sets (the files are shared by both), runs the
Differential harness alone (`--skip-tests`), restores the file byte for byte and checks that `git diff` of `rust/src` is
empty. Four mutants, two feature sets: 8 of 8 runs red.

| Mutant (site)                                                                      | Set       | First red piece                                                          | Harness alone     |
|------------------------------------------------------------------------------------|-----------|--------------------------------------------------------------------------|-------------------|
| whitespace `<= 32` to `<= 31`, `cJSON.rs:1214` `buffer_skip_whitespace`            | `""`      | translated tests: 25 failing (compare, get_object_item, ...)             | 43 of 3881 DIFF   |
| whitespace `<= 32` to `<= 31`                                                      | `locales` | translated tests: 25 failing                                             | 43 of 3881 DIFF   |
| hex digit `'9'` to `'8'`, `cJSON.rs:750` `parse_hex4`                              | `""`      | translated tests: 1 failing (`parse_hex4_should_parse_all_combinations`) | 2 of 3881 DIFF    |
| hex digit `'9'` to `'8'`                                                           | `locales` | translated tests: 1 failing (`parse_hex4_should_parse_all_combinations`) | 2 of 3881 DIFF    |
| escape `< 32` to `< 31`, `cJSON.rs:1113` `print_string_ptr`                        | `""`      | translated tests: 1 failing (`print_string_should_print_ascii`)          | 3881 of 3881 DIFF |
| escape `< 32` to `< 31`                                                            | `locales` | translated tests: 1 failing (`print_string_should_print_ascii`)          | 3881 of 3881 DIFF |
| index digit `'9'` to `'8'`, `cJSON_Utils.rs:334` `decode_array_index_from_pointer` | `""`      | differential harness only (translated tests pass)                        | 34 of 3881 DIFF   |
| index digit `'9'` to `'8'`                                                         | `locales` | differential harness only (translated tests pass)                        | 34 of 3881 DIFF   |

Reading: the array-index mutant is invisible to all translated tests, as it was invisible to the C upstream tests in
Stage 1 (E4); only the Differential harness catches it, through the 32 `judge-idx-*` inputs. The other three are caught
by both pieces. The two cJSON.rs byte-class mutants are the same sites as in E4, so the Stage 1 and Stage 2 Judges are
equally sensitive on this sample. An interrupted run of the experiment leaves `rust/src` mutated: check `git diff --
pilots/cjson/rust/src` before anything else.

### Not done or not run

- The Stage 2 adapter (RULEBOOK section 6 is still `TODO`), so the `HOOKS = false` and `C_LAYOUT = false` paths of
  `upstream.rs`, `dump.rs` and `run.py --stage 2` have not been run (only reasoned about). The first Stage 2 run must
  start by checking that the `SKIPPED:` list is the one above.
- `cargo test` was not run under `--release`; the corpus run uses the debug example build.
- Windows, MSVC, Sun: untested (RULEBOOK D18).
- Nothing was committed (instructions of this ticket step).

## Rule proposals

Gaps found while building the Stage 2 Judge. Not applied to the RULEBOOK.

| id | section | proposal                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
|----|---------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| P1 | 6       | State what the Stage 2 API must make observable for the adapter: the `type` word including the `IsReference` and `StringIsConst` bits (D12 says the flag values stay observable, the translated tests assert them), the parse error position and parse end as byte offsets (D9), the status code `0..13` of the patch functions, and a handle that can be compared for identity (tests compare `GetPointer(...) == GetObjectItem(...)`).                                                                         |
| P2 | 10      | Say that the Stage 2 Differential harness compares everything except the allocator sweep: D1 drops the hooks, so the C driver is built with `-DDUMP_NO_HOOKS` and nothing covers allocation failure in Stage 2. Without this line a reader would expect the allocation counters to match.                                                                                                                                                                                                                        |
| P3 | 10      | List the translated tests that D1, D7, D11, D14 exclude, by name. The measured list: D1 the 13 `*_should_fail_on_allocation_failure` of `cjson_add` and `ensure_should_fail_on_failed_realloc`; D7 the overlap test; D11 the circular test (replaced by `extra_duplicate_should_stop_at_the_circular_limit_without_a_cycle`); D14 the null-prev detach, the corrupted part of the null-pointer test, the `Invalid` node of the typecheck test and two compare tests with hand-built nodes; the aliased-key test. |
| P4 | 7 or 8  | Stage 1 debug builds use more than 2 KiB of stack per nesting level in parse (the 1000-deep test overflowed a 2 MiB thread). A Stage 2 recursive parser may need an explicit stack or the Judge needs a bigger thread; decide before Stage 2 (D10).                                                                                                                                                                                                                                                              |
| P5 | Q-table | `cJSON_Parse(NULL)` leaves the global error pointer NULL (`err=-1` in the driver), while a parse of an empty buffer reports offset 0; the Judge relies on both. A Stage 2 error type must reproduce `Unset` for the NULL input.                                                                                                                                                                                                                                                                                  |
