# RUN-NOTES: cJSON

## Stage 1 Judge validation (ticket 02, 2026-10-10)

Everything below was measured by running `pilots/cjson/migration/harness/run.py --scratch [FEATURES]` on macOS
aarch64 (clang from Xcode, rustc stable, c2rust 0.22.1). The Judge was validated on SCRATCH crates produced by
`harness/make_scratch.py` (one c2rust run per feature set, under `harness/work/scratch/`, git-ignored). They are not
the accepted target `pilots/cjson/rust/`, which does not exist yet (ticket 04).

### Files

| Path                                                   | What                                                                                  |
|--------------------------------------------------------|---------------------------------------------------------------------------------------|
| `harness/run.py [--crate DIR \| --scratch] [FEATURES]` | the Judge, exit 0 = pass; `--skip-tests` runs the Differential harness alone          |
| `harness/make_scratch.py`                              | c2rust per feature set and the hand tweaks below                                      |
| `harness/shim/common.h`                                | replaces upstream `tests/common.h` (public headers only, internals as `extern`)       |
| `harness/dump.c`                                       | the canonical dump driver (parse, print, lookups, hooks, Utils, locale, constructors) |
| `harness/make_inputs.py`                               | seeded corpus generator (seed 20261010), writes `harness/inputs/` (git-ignored)       |
| `harness/e4_cjson_judge_sensitivity.py`                | mutation experiment (see "Judge sensitivity"; location note below)                    |

### Upstream tests, per file and feature set

Both columns are `tests / failures / ignored`. The C original is the upstream test built with the real `common.h`
(it includes `cJSON.c`); the Rust column is the same test source with the shim, linked to the staticlib built with
`test-internals`. All 21 test programs linked; no test is C-original-only.

| Test file          | `""` C original | `""` Rust   | `locales` C original | `locales` Rust |
|--------------------|-----------------|-------------|----------------------|----------------|
| `parse_examples`   | 15 / 0 / 0      | 15 / 0 / 0  | 15 / 0 / 0           | 15 / 0 / 0     |
| `parse_number`     | 6 / 0 / 0       | 6 / 0 / 0   | 6 / 0 / 0            | 6 / 0 / 0      |
| `parse_hex4`       | 2 / 0 / 0       | 2 / 0 / 0   | 2 / 0 / 0            | 2 / 0 / 0      |
| `parse_string`     | 6 / 0 / 0       | 6 / 0 / 0   | 6 / 0 / 0            | 6 / 0 / 0      |
| `parse_array`      | 4 / 0 / 0       | 4 / 0 / 0   | 4 / 0 / 0            | 4 / 0 / 0      |
| `parse_object`     | 4 / 0 / 0       | 4 / 0 / 0   | 4 / 0 / 0            | 4 / 0 / 0      |
| `parse_value`      | 7 / 0 / 0       | 7 / 0 / 0   | 7 / 0 / 0            | 7 / 0 / 0      |
| `print_string`     | 3 / 0 / 0       | 3 / 0 / 0   | 3 / 0 / 0            | 3 / 0 / 0      |
| `print_number`     | 6 / 0 / 1       | 6 / 0 / 1   | 6 / 0 / 1            | 6 / 0 / 1      |
| `print_array`      | 3 / 0 / 0       | 3 / 0 / 0   | 3 / 0 / 0            | 3 / 0 / 0      |
| `print_object`     | 3 / 0 / 0       | 3 / 0 / 0   | 3 / 0 / 0            | 3 / 0 / 0      |
| `print_value`      | 7 / 0 / 0       | 7 / 0 / 0   | 7 / 0 / 0            | 7 / 0 / 0      |
| `misc_tests`       | 30 / 0 / 0      | 30 / 0 / 0  | 30 / 0 / 0           | 30 / 0 / 0     |
| `parse_with_opts`  | 6 / 0 / 0       | 6 / 0 / 0   | 6 / 0 / 0            | 6 / 0 / 0      |
| `compare_tests`    | 10 / 0 / 0      | 10 / 0 / 0  | 10 / 0 / 0           | 10 / 0 / 0     |
| `cjson_add`        | 31 / 0 / 0      | 31 / 0 / 0  | 31 / 0 / 0           | 31 / 0 / 0     |
| `readme_examples`  | 3 / 0 / 0       | 3 / 0 / 0   | 3 / 0 / 0            | 3 / 0 / 0      |
| `minify_tests`     | 7 / 0 / 0       | 7 / 0 / 0   | 7 / 0 / 0            | 7 / 0 / 0      |
| `json_patch_tests` | 3 / 0 / 0       | 3 / 0 / 0   | 3 / 0 / 0            | 3 / 0 / 0      |
| `old_utils_tests`  | 6 / 0 / 0       | 6 / 0 / 0   | 6 / 0 / 0            | 6 / 0 / 0      |
| `misc_utils_tests` | 1 / 0 / 0       | 1 / 0 / 0   | 1 / 0 / 0            | 1 / 0 / 0      |
| total              | 163 / 0 / 1     | 163 / 0 / 1 | 163 / 0 / 1          | 163 / 0 / 1    |

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

| Case | Where                                                                                                           |
|------|-----------------------------------------------------------------------------------------------------------------|
| D3   | locale pass: `LC_NUMERIC` set to the first comma locale the OS has; parse and print of `1.5`, arrays, the input |
| D6   | constructors: `cJSON_CreateNumber(NAN)`, `cJSON_SetNumberHelper(NAN)`, Inf, -Inf                                |
| D15  | every parsed document is printed with `cJSON_PrintBuffered(item, 0, fmt)` for both `fmt` values                 |
| D19  | locale pass: `LC_CTYPE` set to a Latin-1 locale; keys with bytes >= 0x80 looked up, compared and sorted         |
| Q15  | `judge-q15-*` inputs; every input also goes through `cJSON_Minify`                                              |
| Q20  | `judge-q20-*` and `patch-*` inputs with `~0` / `~1` paths                                                       |

- D3: this OS has comma locales (`locale -a`). The driver picked `de_DE.UTF-8` (decimal point `,`). With
  `ENABLE_LOCALES` the parse of `1.5` works and prints `1.5`; without it `1.5` parses as `1` and
  `cJSON_CreateNumber(1.5)` prints `1,5`. Both programs agree in both feature sets (0 DIFF) for the Stage 1
  Faithful port. Stage 2 (`C_LAYOUT=false`) always uses the C locale (approved Deviation), so `run.py --stage 2`
  sets `DUMP_PROBE_IN_C_LOCALE=1` for both drivers: the locale is still chosen and printed, then reset to `C`
  before the probes, so both programs run the same probes in the C locale. The approved behaviour itself is
  asserted in `rust/tests/upstream.rs` (`locale_numeric_comma_locale_still_parses_and_prints_a_dot_d3`).
- D6: on this machine (aarch64, clang -O0) the C original also prints `vi=0` for `(int)NaN`; the dump line is
  `NaN vi=0`. The comparison with C is therefore a real equality here, but C leaves it undefined (RULEBOOK D6), so
  treat a future difference on another platform as informational.
- D19: the driver picked `de_DE.ISO8859-1` (`tolower(0xC9) = 233`). The C original and the Stage 1 crate
  agree. Stage 2 uses the same `DUMP_PROBE_IN_C_LOCALE` comparison as D3;
  `locale_ctype_latin1_keys_are_not_case_equal_d19` asserts that `\xC9` and `\xE9` are not case-equal under it.
- `cJSON_Compare` on objects is exponential in nesting depth (it recurses a to b and b to a at every level): a
  999-deep object did not finish in 100 s with the C original. The driver therefore calls `cJSON_Compare` only when
  the tree is at most 14 deep and prints `compare skipped`. This is upstream behaviour, not a port defect; it is
  not in the RULEBOOK (candidate for a new quirk row, deliberately not edited here).

### Judge sensitivity

`harness/e4_cjson_judge_sensitivity.py` plants one one-character mutant in the scratch crate source, runs the full
Judge, restores the file, and then runs the Differential harness alone (`--skip-tests`). Each mutant changes one
integer literal inside one function. All 8 mutants were detected (exit 1). "Tests" is the first piece to go red.

| Feature set | Mutant (C site)                                                       | Upstream tests caught it        | Harness alone caught it |
|-------------|-----------------------------------------------------------------------|---------------------------------|-------------------------|
| `""`        | whitespace `<= 32` to `<= 31` in `buffer_skip_whitespace`             | yes: 8 files, 24 failures       | 43 of 3849 DIFF         |
| `""`        | `print_string_ptr` escape `< 32` to `< 31`                            | yes: `print_string` (1 failure) | 3849 of 3849 DIFF       |
| `""`        | array index digit `'9'` to `'8'` in `decode_array_index_from_pointer` | **no**, 0 failures              | 34 of 3881 DIFF         |
| `""`        | hex digit `'9'` to `'8'` in `parse_hex4`                              | yes: `parse_hex4` (1 failure)   | 2 of 3849 DIFF          |
| `locales`   | whitespace `<= 32` to `<= 31`                                         | yes: 8 files, 24 failures       | 43 of 3849 DIFF         |
| `locales`   | array index digit `'9'` to `'8'`                                      | **no**, 0 failures              | 34 of 3881 DIFF         |
| `locales`   | `46 =>` to `47 =>` ('.' accepted in a number) in `parse_number`       | yes: 5 files, 6 failures        | 3849 of 3849 DIFF       |
| `locales`   | `decimal_point[0]` to `decimal_point[1]` in `get_decimal_point`       | yes: 5 files, 6 failures        | 3849 of 3849 DIFF       |

Reading: the array-index mutant is invisible to all 21 upstream test programs and is caught only by the Differential
harness. With the first corpus (3849 inputs) only 2 inputs caught it (`valid-043`, `valid-051`). The corpus was then
extended with 32 `judge-idx-*` inputs (a 12-element and a 100-element array, and `replace` patches at every digit in
the units position and digits 1-9 in the tens position); the two array-index rows above were re-run on the 3881-input
corpus: 34 inputs DIFF in each feature set. The other rows were measured on the 3849-input corpus and not re-run; their
whole-corpus DIFF counts of 3849 come from the fixed constructors section, which prints in every run. No mutant
survived.

### c2rust workarounds and hand tweaks (scratch crates)

| id | Where                        | What                                                                                                                                                                                                                      |
|----|------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| W1 | compile DB                   | named exactly `compile_commands.json`, one entry per `.c` file (decision 23 workaround 1)                                                                                                                                 |
| W2 | output dir                   | `cjson_c2rust`, never `crate` (decision 23 workaround 2)                                                                                                                                                                  |
| W3 | toolchain                    | not needed: after T1 to T6 the crate builds with stable cargo; no `cargo +<pin>` (decision 25 unused)                                                                                                                     |
| T1 | `lib.rs`, `Cargo.toml`       | rewritten: c2rust's `#![feature(linkage)]`, `#![feature(raw_ref_op)]`, `extern crate f128`, build.rs dropped; features `locales` (no-op) and `test-internals` added; `crate-type = ["staticlib", "rlib"]`                 |
| T2 | `cJSON.rs`                   | `use ::f128;` removed; `::f128::f128` replaced by `f64` (2 sites) and `::f128::f128::new(x)` by `x` (2 sites). macOS `isnan`/`isinf` expand to `__inline_isnanl(long double)` and on aarch64 macOS `long double` is `f64` |
| T3 | `cJSON.rs`                   | `test-internals`: 27 static functions and 2 statics (`global_hooks`, `global_error`) become `pub` with `#[cfg_attr(feature = "test-internals", no_mangle)]`                                                               |
| T5 | `cJSON.rs`, `cJSON_Utils.rs` | one `#[linkage = "external"]` removed per file (nightly-only attribute)                                                                                                                                                   |
| T6 | `cJSON.rs`, `cJSON_Utils.rs` | `#[no_mangle]` removed from the inline libc helper `tolower` in each file: otherwise both modules define the symbol `tolower` (duplicate symbol) and it would shadow libc                                                 |

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
crate `pilots/cjson/rust/` (c2rust output, C ABI). `rust/src` and `Cargo.toml` were not changed (the `rust/src` diff
is empty after every run, including the mutant runs). Updated the same day after the user's decisions on the rule
proposals (D1 and D11 revised, D21 added): see "Change set after the user decisions" at the end of this section; the
text above it is updated where it changed.

### Files

| Path                                               | What                                                                                                                                     |
|----------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------|
| `rust/tests/adapter/mod.rs`                        | the only file that calls the crate; Stage 1 adapter over the C ABI; interface contract in its header                                     |
| `rust/examples/dump.rs`                            | line-for-line port of `harness/dump.c` through the adapter; same bytes on every corpus input                                             |
| `rust/tests/upstream.rs`                           | the 163 upstream tests translated through the adapter, plus 3 extra tests (see below)                                                    |
| `harness/run.py --stage 2 [FEATURES]`              | `cargo test` (translated tests), the release stack check (D10), then original C driver vs `cargo build --example dump`, 2 runs per input |
| `harness/dump.c`                                   | M7 gained the D21 view and Q3 field-write lines; the failure sweeps print `live=`; no `DUMP_NO_HOOKS` any more                           |
| `experiments/e5_cjson_stage2_judge_sensitivity.py` | mutants in `rust/src`, Stage 2 Judge, restore (see Judge sensitivity below)                                                              |
| `parts-stage2.tsv`                                 | both Parts, state `todo`, deps as in `parts.tsv`                                                                                         |

### Adapter interface

The contract is the header of `rust/tests/adapter/mod.rs` and RULEBOOK §6.0 (eight points: identity, fields of every
node, links with the `prev` invariant, parse end and error position with Q26, status codes, strings and buffer
addresses, writable views (D21), allocation hooks (D1)). `Item` is a `Copy + Eq + Hash` handle for a `cJSON *` (NULL
allowed); ownership is manual as in C; strings are byte slices without the NUL; `None` is NULL; parse results carry
the error position and parse end as offsets into the input (`ErrPos::{Unset, At, Outside}`).

One capability constant is left, `C_LAYOUT` (D7, D14): the `raw_*` functions work (hand-corrupted lists, a write into
the node's own key, a node with two kind bits). Steps that need them print `SKIPPED:` and are left out when it is
`false`. The `HOOKS` constant and the `-DDUMP_NO_HOOKS` build were removed with the D1 revision: every adapter has the
hooks, so the allocator section is always compared and no adapter can switch it off.

`run.py --stage 2` runs `cargo test -- --nocapture` and prints the distinct `SKIPPED:` lines, so a Stage 2 run shows
what the adapter did not exercise. With the Stage 1 adapter `C_LAYOUT` is `true` and nothing is skipped (measured: 0
skipped step kinds). With `C_LAYOUT = false` set by hand on the Stage 1 adapter (one `cargo test`, then restored),
163 passed, 0 failed, 2 ignored, and 5 skipped step kinds: the two D7 tests and the three D14 steps of RULEBOOK §10.

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
| `misc_tests`: `ensure_should_fail_on_failed_realloc`     | `ensure`, `printbuffer`, a `realloc` hook            | public variant (D1 revised): `PrintBuffered` with a 10-byte prebuffer, growth allocation 2 fails; NULL, 2 allocations, 1 free, 10 bytes (custom hooks have no `realloc`, `cJSON.c:233-237`)      |
| `misc_tests`: `skip_utf8_bom_*` (2)                      | `skip_utf8_bom`, `parse_buffer`, `global_hooks`      | public `cJSON_ParseWithOpts`: BOM then `{}` parses with `parse_end` at 5; a BOM after a space gives NULL (the private `offset = 1` setup is not reachable, the observable effect is the same)    |
| `misc_tests`: `..._string_is_aliased`                    | `cJSON_strdup`, `global_hooks`, `item->string` write | `raw_add_item_with_aliased_key`, `C_LAYOUT` only (D7)                                                                                                                                            |

Tests that did not use private functions but wrote or read node fields on stack-built structs:

| Test                                                                                          | Translation                                                                                                                                                              |
|-----------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `cjson_array_foreach_should_loop_over_arrays`                                                 | a parsed array of 10; `children()` equals `array_item(i)` for every `i`                                                                                                  |
| `typecheck_functions_should_check_type`                                                       | items get `StringIsConst` through `AddItemToObjectCS`; the `Invalid` node is a root after a `remove` patch at `""`, then added with `AddItemToObjectCS` (type 512, as C) |
| `cjson_detach_item_via_pointer_should_detach_items`                                           | four heap nulls in a real array; same assertions on `prev`, `next`, `child`                                                                                              |
| `cjson_detach_item_via_pointer_should_return_null_if_item_prev_is_null`                       | public variant always (a node in no list, guard `cJSON.c:2260`); the hand-linked list `C_LAYOUT` only (`raw_set_prev`, D14)                                              |
| `cjson_replace_item_via_pointer_should_replace_items`                                         | heap replacements instead of stack structs; same assertions                                                                                                              |
| `cjson_replace_item_in_object_should_preserve_name`                                           | root is `cJSON_CreateObject()` instead of a zeroed stack node                                                                                                            |
| `cjson_functions_should_not_crash_with_null_pointers`                                         | all calls kept; the two assertions that depend on the hand-corrupted list and the freed valuestring run only under `C_LAYOUT`                                            |
| `cjson_set_valuestring_should_return_null_if_strings_overlap`                                 | `C_LAYOUT` only (`raw_set_valuestring_from_own`, D7)                                                                                                                     |
| `cjson_should_not_follow_too_deep_circular_references`                                        | as written (D11 revised: handles hold the cycle); plus the extra test below                                                                                              |
| `cjson_compare_should_compare_invalid_as_not_equal`                                           | the `Invalid` node is a root after a `remove` patch at `""` (Q23)                                                                                                        |
| `cjson_compare_should_not_accept_invalid_types`                                               | `C_LAYOUT` only (`raw_new_with_type(Number \| String)`, D14)                                                                                                             |
| `cjson_compare_should_compare_raw`                                                            | `cJSON_CreateRaw` instead of parse then `type = cJSON_Raw` (same node)                                                                                                   |
| `cjson_set_number_value_should_set_numbers`                                                   | `cJSON_CreateNumber(0)` instead of a stack node                                                                                                                          |
| `cjson_create_string_reference_...`, `cjson_set_valuestring_to_object_should_not_leak_memory` | all assertions always run; buffer addresses through `value_string_addr` (no longer `C_LAYOUT`)                                                                           |

Three tests are not upstream, all in both Stages: `extra_duplicate_should_stop_at_the_circular_limit_without_a_cycle`
(an acyclic array nested `CIRCULAR_LIMIT + 1` deep must not be duplicated: the boundary, D11),
`extra_release_build_reaches_the_limits_on_a_2_mib_stack` (D10, ignored in debug builds, run by `run.py --stage 2`
with `--release`) and `extra_add_to_a_reference_of_a_node_in_no_list_succeeds_without_appending` (D14, see the change
set). `upstream.rs` has 166 tests: in a debug `cargo test` 164 pass and 2 are ignored (the upstream `TEST_IGNORE()`
and the release-only test).

### Measured (ticket 05, before the change set)

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

### Judge sensitivity (Stage 2, ticket 05)

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

### Change set after the user decisions (2026-10-10)

The user decided D1 revised (an `AllocHooks` seam in the Stage 2 API, so allocation failure is judged in both Stages),
D11 revised (the node store is `Copy` handles, so a cycle can be built and the circular test runs as written) and D21
new (writable byte-slice views of string values and buffers). What changed in each Judge file:

| File                                               | Change                                                                                                                                                                                             |
|----------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `rust/tests/adapter/mod.rs`                        | `HOOKS` removed; contract header; `value_string_addr`; `set_value_int` / `set_value_double`; `with_string_value_mut`; `Printed::bytes_mut`                                                         |
| `rust/tests/upstream.rs`                           | `failing_hooks` unconditional; `make_invalid` through a root `remove` patch; circular test ungated; public null-`prev` and `ensure` realloc variants; address asserts ungated; two new extra tests |
| `harness/dump.c`, `rust/examples/dump.rs`          | no `DUMP_NO_HOOKS`; D21 view and Q3 field-write lines in the constructors; `live=` counts on the fail-print, fail-buffered and fail-duplicate sweeps                                               |
| `harness/run.py` (`--stage 2`)                     | no `HOOKS` parsing; the release stack check (D10) as its own step                                                                                                                                  |
| `experiments/e5_cjson_stage2_judge_sensitivity.py` | `--set`, `--quick`, text mutants, six new mutants                                                                                                                                                  |
| `experiments/e6_cjson_reference_add_probe.c`       | new: probe of the `add_item_to_array` branch with `prev == NULL`                                                                                                                                   |

Disagreement with the review (`experiments/stage2_proposals_review.md`): the review said the branch of
`add_item_to_array` that handles `child->prev == NULL` (`cJSON.c:2050-2055`) may be removed from the Stage 2 API
because it is private state. E6 ran on the original C and printed `add=1 insert=1 size=1 print=[null] y.prev_null=1
z.prev_null=1`: the branch is publicly reachable through `cJSON_CreateArrayReference` of a node that is in no list.
Stage 2 keeps it, and the extra test `extra_add_to_a_reference_of_a_node_in_no_list_succeeds_without_appending` pins
the observable behaviour (the add succeeds and nothing is appended).

D21 note: no upstream test writes through a `char *` that the library returned (grep of `src/tests`: only identity
compares at `misc_tests.c:555/581/711/718`; the in-place writes go to caller buffers, `minify_tests` with `strcpy` and
`cJSON_Minify`, and the fix-up loop of `print_number.c`). D21 is therefore judged by the new dump lines, not by an
upstream test.

`ensure_should_fail_on_failed_realloc` called the private `ensure`. The translation is a public variant: `PrintBuffered`
with a 10-byte prebuffer and a hook set whose second allocation fails. The expected values (NULL, 2 allocations, 1
free, 10 bytes) are the C results, because custom hooks have no `realloc` and `ensure` falls back to allocate, copy
and free (`cJSON.c:233-237`).

#### Measured (change set)

Wall-clock numbers marked "loaded" were taken while the machine was heavily loaded by other processes (load average
about 50, swap almost full); they are not comparable with the earlier rows. The harness runs each input with a 20 s
timeout, so a loaded run can report a spurious DIFF or UNSTABLE.

| Command                           | Feature set | Translated tests (passed / failed / ignored) | Differential harness   | Wall                |
|-----------------------------------|-------------|----------------------------------------------|------------------------|---------------------|
| `run.py` (Stage 1, final)         | `""`        | 163 / 0 / 1                                  | 3881/3881 inputs match | 1:40.40             |
| `run.py locales` (Stage 1, final) | `locales`   | 163 / 0 / 1                                  | 3881/3881 inputs match | 1:33.85             |
| `run.py --stage 2` (first timed)  | `""`        | 163 / 0 / 2, release check 1 passed          | 3881/3881 inputs match | 1:12.21             |
| `run.py --stage 2` (first timed)  | `locales`   | 163 / 0 / 2, release check 1 passed          | 3881/3881 inputs match | 1:34.89             |
| `run.py --stage 2` (final)        | `""`        | 164 / 0 / 2, release check 1 passed          | 3881/3881 inputs match | 58:19.92 (loaded)   |
| `run.py --stage 2` (final)        | `locales`   | 164 / 0 / 2, release check 1 passed          | 3881/3881 inputs match | 1:09.30             |

The first timed Stage 2 rows are from before the last `live=` edit of `dump.c` and before the third extra test (163
passed); the debug test binaries had been built by a `cargo test --no-run` just before, and the release build was
cold. After the `live=` edit the same two runs gave 1:13.75 and 1:14.67, both green with the same counts. The Stage 1
rows are after all `dump.c` edits; the first Stage 1 runs, before the `live=` edit, took 1:51.92 and 1:50.93 and were
green.

The final `""` run took 58:19.92 wall but only 36.98 s user + 65.89 s system CPU: it ran while two other Judge runs
shared the work directory and the machine load average was 50 to 150 (the stray runs and an IDE index), so its wall
number is not a cost figure. The `locales` run, alone on the machine, took 1:09.30. Under that load an earlier run
hit the harness's 20 s per-input timeout on `valid-058`, `valid-059` and `byte-num-37`; those are load artefacts, and
the loaded-run results were not used. Rule: one Judge run at a time (judge.md §6).

`C_LAYOUT = false` set by hand on the Stage 1 adapter (one `cargo test`, then restored): 163 passed, 0 failed, 2
ignored, 5 `SKIPPED` step kinds (D7 overlap, D7 aliased key, D14 two-kind-bits compare, D14 hand-built null-`prev`
list, D14 corrupted array item and string). That is the list of RULEBOOK §10.

#### Judge sensitivity (change set)

Six new mutants, each run with `e5 --only NAME --set default --quick` (the translated tests and the Differential
harness; no harness-alone rerun). All six are detected.

| Mutant                                                  | Site                 | First red piece                                                       |
|---------------------------------------------------------|----------------------|-----------------------------------------------------------------------|
| `strdup-size` (allocation size 1 to 2)                  | `cJSON.rs:263`       | harness: 1729 of 3881 DIFF (allocator section)                        |
| `duplicate-fail-leak` (`cJSON_Delete(newitem)` to drop) | `cJSON.rs:3064`      | harness: 1869 of 3881 DIFF, caught by the new `live=` counts          |
| `patch-status-13`                                       | `cJSON_Utils.rs:883` | harness: 279 of 3881 DIFF                                             |
| `circular-boundary` (`>=` to `>`)                       | `cJSON.rs:3025`      | translated tests: 1 failing (`extra_duplicate_...`)                   |
| `invalid-type`                                          | `cJSON.rs:3203`      | translated tests: 1 failing (`typecheck_functions_should_check_type`) |
| `print-valueint`                                        | `cJSON.rs:689`       | harness: 3881 of 3881 DIFF                                            |

All-sets confirmation, `e5 --quick` (10 mutants, the four of ticket 05 and the six above, times 2 feature sets): 20 of
20 DETECTED, identical verdicts per set (`ws-boundary` 25 failing tests, `hex-digit` 1, `string-escape-boundary` 1,
`array-index-digit` harness 34 of 3881, plus the six above); `git diff` of `rust/src` empty afterwards; wall 12:12.84.
The harness-alone column for the four old mutants is the one of ticket 05 (table above).

#### Q26 measurement

`dump_orig` run over all 3881 inputs of `harness/inputs/` one after the other, counting the lines
`Parse(NULL) root=NULL err=...`: `err=-1` 2946 times and `err=outside` 1842 times (4788 lines: one per input, and a
second one on some inputs, e.g. `byte-keyci-15.json`, where the parse section is repeated). These are the values of
the review (2946 and 1842), so the figure in the RULEBOOK Q26 row, which came from a predecessor, is now confirmed by
a run and both values are listed there.

### Not done or not run

- The Stage 2 adapter does not exist yet (ticket 06). The `C_LAYOUT = false` path of `upstream.rs` was run once by
  hand on the Stage 1 crate (change set above); the first Stage 2 run must check that the `SKIPPED:` list is the
  five steps of RULEBOOK §10.
- Only the D10 release check runs under `--release`; the translated tests and the corpus run use debug builds.
- Windows, MSVC, Sun: untested (RULEBOOK D18).
- Nothing was committed (instructions of this ticket step).

## Stage 2 review GAPs: Judge additions (ticket 06, 2026-10-11)

Added after the Opus review (`experiments/stage2_port_review.md`, commit d14ed4a and its fix-up):

- Corpus 3881 -> 3886: `judge-q18-sort-add2`, `judge-q18-sort-add3` (sort leaves the first `prev` stale, Q18),
  `judge-q19-wrap` (array index wrap-around), `judge-q27-detach` (case-sensitive detach is case-insensitive, Q27),
  `judge-utils-oom` (fixed allocator probes); two more `GetPointer` paths in `fixed[]`.
- New dump section `== utils-hooks` for the 117 `patch-*` and `judge-*` inputs: counting trace (`a<size>` per
  allocation, `f` per free) and fail-the-Nth sweep (cap 48) for ApplyPatches (both cases), MergePatch,
  GenerateMergePatch; trace only for GeneratePatches and FindPointerFromObjectTo; `dump_tree` after each in-place sort.
  `judge-utils-oom` also runs two probes: `FindPointerFromObjectTo` with allocation 1 failing (BLOCK 1) and
  `GeneratePatches([], [1])` with allocation 2 failing (BLOCK 2).
- Translated tests: 6 more in `upstream.rs` (BLOCK 1, 2, 4 and the two D22 neighbours), adapter helpers
  `parse_unterminated`, `parse_with_opts_unterminated`, `trace_start`, `trace_stop`.
- Allocator classes (judge.md 5a): allocation failure covered by the section above and the 13 translated
  `*_should_fail_on_allocation_failure` tests; absent members by the D22 tests (NULL-path `test`, NULL-from `copy`, both
  layouts); input without a trailing NUL by the two `*_unterminated` tests.
- Red state on the attempt-1 crate (34f7efe), `run.py --stage 2 --skip-tests ""`: 3885/3886 inputs match, the one DIFF
  is `judge-utils-oom` (Rust aborts, exit -6 = BLOCK 1/2). Of the new translated tests, BLOCK 1 and 2 abort the test
  binary (SIGABRT), BLOCK 4 (two tests) and `apply_patches_copy_with_null_from_is_status_5` (D22) fail. The corpus step
  of this run took 32 s wall with the harness `--jobs` default; the corpus step alone is about that.

## Rule proposals

Gaps found while building the Stage 2 Judge. Reviewed in `experiments/stage2_proposals_review.md`; resolved on
2026-10-10 with the user's decisions and applied to the RULEBOOK (§12 Amendments).

Rows 6-11 of `experiments/stage2_port_review.md` applied to the RULEBOOK 2026-10-11 by user approval (Q27, Q18, D22,
D20, D2, section 5).

| id | section | proposal                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | resolution                                                                                                                                                                                  |
|----|---------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| P1 | 6       | State what the Stage 2 API must make observable for the adapter: the `type` word including the `IsReference` and `StringIsConst` bits (D12 says the flag values stay observable, the translated tests assert them), the parse error position and parse end as byte offsets (D9), the status code `0..13` of the patch functions, and a handle that can be compared for identity (tests compare `GetPointer(...) == GetObjectItem(...)`).                                                                         | modified: RULEBOOK §6.0 with the review's full list plus hooks, views (D21) and field writes; adapter header states the same contract                                                       |
| P2 | 10      | Say that the Stage 2 Differential harness compares everything except the allocator sweep: D1 drops the hooks, so the C driver is built with `-DDUMP_NO_HOOKS` and nothing covers allocation failure in Stage 2. Without this line a reader would expect the allocation counters to match.                                                                                                                                                                                                                        | moot: D1 revised; the C driver is never built with `-DDUMP_NO_HOOKS`, the allocator sweep is compared in Stage 2                                                                            |
| P3 | 10      | List the translated tests that D1, D7, D11, D14 exclude, by name. The measured list: D1 the 13 `*_should_fail_on_allocation_failure` of `cjson_add` and `ensure_should_fail_on_failed_realloc`; D7 the overlap test; D11 the circular test (replaced by `extra_duplicate_should_stop_at_the_circular_limit_without_a_cycle`); D14 the null-prev detach, the corrupted part of the null-pointer test, the `Invalid` node of the typecheck test and two compare tests with hand-built nodes; the aliased-key test. | modified: RULEBOOK §10 lists the corrected exclusions (D7 two tests, D14 three steps); `Invalid` nodes, the null-`prev` guard, buffer addresses, the hooks and the cycle run in both Stages |
| P4 | 7 or 8  | Stage 1 debug builds use more than 2 KiB of stack per nesting level in parse (the 1000-deep test overflowed a 2 MiB thread). A Stage 2 recursive parser may need an explicit stack or the Judge needs a bigger thread; decide before Stage 2 (D10).                                                                                                                                                                                                                                                              | modified: D10 duties (non-recursive delete; release check on a 2 MiB thread, now a step of `run.py --stage 2`)                                                                              |
| P5 | Q-table | `cJSON_Parse(NULL)` leaves the global error pointer NULL (`err=-1` in the driver), while a parse of an empty buffer reports offset 0; the Judge relies on both. A Stage 2 error type must reproduce `Unset` for the NULL input.                                                                                                                                                                                                                                                                                  | corrected (the fact was wrong): `Parse(NULL)` keeps the previous error (Q26)                                                                                                                |
