# RUN-NOTES: tinyjson

Pilot: `pilots/tinyjson` (C++, header-only `src/tinyjson.h`, GPL-3.0, experiment only). The Source root is
`pilots/tinyjson/src`, so the header is `src/src/tinyjson.h` on disk and `src/tinyjson.h` in the inventory.

## Skill bugs

Found while running ticket 01 (2026-10-11). Nothing was changed under `skill/**`; each line says what was done
instead.

1. `inventory.py --lang cpp` lists symbols of `test/` and `samples/` (1630 symbols, 1564 of them the vendored
   `test/catch.hpp`) although SKILL.md says test and example files are never Parts. The default skip list has
   `fuzzing/ unity/ vendor/ third_party/` but not `test/` or `samples/`. Used the documented `--exclude 'test/*'
   --exclude 'samples/*'`: 42 symbols, one Part (`src/tinyjson`).
2. `inventory.py` regex engine (no `compile_commands.json` for C++) emits non-symbols as `function` rows: `bool`,
   `double`, `long`, `std::string` (from `operator bool()` and conversion operators), and two `json::operator` rows
   with no operator name. Kept as is; the RULEBOOK maps the operators by reading the header.
3. `dead.txt` lists 39 of 42 symbols as dead-code candidates because the test file that calls them is excluded (and
   a header-only library has no in-library caller for its public API). `dead.txt` is not usable for this Pilot.
4. `rulebook-template.md` does not fit C++ (reported by the Opus subagent, not re-checked by the governor):
   - §0 requires a C ABI and `#[repr(C)]` for every public symbol; C++ has no C ABI (decision 5). The RULEBOOK has a
     C++ posture instead.
   - There is no C++ idiom table; the C++ rows live in a new RULEBOOK §2.1 (CPP1-CPP36) with a `status` column.
   - §3 assumes `config-macro` tags; the C++ inventory has none, yet `CHECK_TYPE_MISMATCH` is user-overridable.
   - §7 assumes C error codes and NULL returns; C++ exceptions (class plus `what()`) need their own model.
   - No slot for the reference toolchain: the header does not compile with Apple clang 17 + libc++ per the
     subagent (D16), and judge.md assumes the original builds with the default compiler.
   - §5 has no naming scheme for overloads and operators.
   - §10 "original tests linked to the staticlib" does not apply; the upstream test file itself needs two patches to
     compile with GCC 16.

## Ticket 01 status (2026-10-11)

- RULEBOOK status `drafted`; §8 has D1-D16, all `proposed`. Waiting for the user to approve or veto each row.
- The subagent's measurements (GCC 16: 17 of 22 upstream tests pass, 5 failures from D3; clang does not build the
  header) are **unverified** by the governor: a `-fsyntax-only` clang run of an empty `main` including the header
  gave only a deprecation warning, which does not exercise the templates.

## Decisions by the user (2026-10-11)

- Rule: the Rust result must match the C++ origin as closely as possible. Applied to D3 and D5 (status `approved`);
  D1, D2, D4 and D6-D16 are still `proposed`.
- D3: kept bug. Every `\uXXXX` yields U+0000, as the Reference built at `-O0` does. The value is uninitialised in
  C++ (`-O2` gives U+0001), so the Judge must build the C++ driver at `-O0`; this is my reading of the rule, to be
  confirmed in ticket 02. The 5 upstream tests that assert real hex decoding are expected to fail in Rust as they
  do on the Reference.
- D5: emulate the libstdc++ `wbuffer_convert` chunk rule. The rule is measured with the C++ driver in ticket 02,
  before Stage 1. Fallback if it cannot be reproduced exactly: end the input at the first invalid byte and add a
  new Deviation row.
- D4 interaction: with D3 giving U+0000, `\u` escapes never produce surrogates in the Reference, so D4 is not
  reachable through `\u` input.
- Skill-change requests sent to session `skill-editing-w-C2RustXW-PtrTrans-ORBIT` (7 items).

## Re-run with the AST engine (2026-10-11)

Skill bugs 1-3 above were fixed by the Skill session (`ast_facts.py` is new; test and sample symbols are roots for
`dead.txt`). Re-ran `inventory.py --lang cpp` and `depmap.py` without `--exclude`: 92 symbols, engine `clang-ast+regex`,
187 edges, 0 cycles, 2 dead-code candidates (`U8ToU32`, `json::has_member`), 3 Parts in `order.txt`. New files:
`facts.tsv` (65 rows: 32 throw, 24 new, 6 delete, 3 conversion), `calls.tsv`, `ast-symbols.tsv`. `parts.tsv` is
unchanged (tier.py not re-run).

**Skill bug 5:** `inventory.py --cxx-flag=-std=c++11` does not work, in either spelling. `inventory.py` line 525 passes
`["--cxx-flag", fl]` to `ast_facts.py`, and argparse refuses a value that starts with `-`; the error is swallowed
and the run falls back to the regex pass (`69 symbols, engine=regex`, no `facts.tsv`). Fix: `f"--cxx-flag={fl}"`.
Workaround used (documented option `--cxx`): a wrapper `clang++ "$@" -std=c++11` in the session scratchpad. The
default of `ast_facts.py` is `-std=c++17` when no `-std=` is given; the wrapper's later flag wins in clang.

### facts.tsv against the RULEBOOK §2.1 rows

The RULEBOOK has **36** CPP rows, not 29. Method: each fact carries the enclosing function's line range; a fact is
"covered" when a line cited by the matching row falls inside that range (script run once in the session, not saved).

| Row   | facts | covered | finding                                                                                          |
|-------|-------|---------|--------------------------------------------------------------------------------------------------|
| CPP8  | 30    | 29      | 1 new: `parser::parse` (626-658) `new std::codecvt_utf8<char32_t>` owned by `wbuffer_convert`    |
| CPP21 | 32    | 22      | 10 throws are `CHECK_TYPE_MISMATCH` expansions (364-425, macro of CPP5); line 1022 is missing    |
| CPP19 | 3     | 0       | the 3 `conversion` facts are `const char*` to `std::string`, not the user conversion operators   |

- CPP10 and CPP14 list 12 constructors plus the copy constructor; `ast-symbols.tsv` has 13 `json::json`. Matches.
- No construct in `facts.tsv` is missing from the CPP rows, apart from the three items above.
- Proposed edits (not applied; RULEBOOK is for the user's review): CPP21 add `1022` and "10 more via
  `CHECK_TYPE_MISMATCH` use sites 365-424"; CPP27 note the facet `new` inside `wbuffer_convert`.

## Reference build

Ticket 02, part 1 (C++ side), 2026-10-11. Everything below was run in this session; commands are in
`harness/ref/build_ref.sh`.

### Candidates tried (judge.md section 8)

| Compiler            | Std library      | -std         | Flags                              | Result                                                                                                                           |
|---------------------|------------------|--------------|------------------------------------|----------------------------------------------------------------------------------------------------------------------------------|
| g++-16 (GCC 16.1.0) | libstdc++ 16.1.0 | c++11, c++17 | none                               | fails: `'wstring_convert' is not a member of 'std'` (needs `<locale>`)                                                           |
| g++-16 (GCC 16.1.0) | libstdc++ 16.1.0 | c++11, c++17 | `-include locale`                  | **builds and runs** (only deprecation warnings)                                                                                  |
| g++-11 (GCC 11.5.0) | libstdc++ 11     | c++11, c++17 | with and without `-isysroot`       | fails before the header: `wchar.h: No such file` / `stdio.h: 'FILE' does not name a type` (fixincludes against the macOS 26 SDK) |
| Apple clang 17.0.0  | libc++           | c++11, c++17 | with and without `-include locale` | fails: `implicit instantiation of undefined template 'std::ctype<char32_t>'` (D16)                                               |

### Reference row

| Item                | Value                                                                              |
|---------------------|------------------------------------------------------------------------------------|
| Compiler            | `/opt/homebrew/bin/g++-16`, Homebrew GCC 16.1.0                                    |
| Standard library    | libstdc++ 16.1.0 (Homebrew), macOS arm64                                           |
| -std                | `c++17` (`c++11` gives the same 17 of 22; checked)                                 |
| Extra flags         | `-include locale -w -I <src/src> -I <src/test>`                                    |
| Driver optimisation | `-O0` (D3: the uninitialised value differs with the optimisation level)            |
| Test patch 1        | `tests:5` `#include "..\src\tinyjson.h"` to `#include "tinyjson.h"` (Windows path) |
| Test patch 2        | `tests:70` `9876543210` to `9876543210LL` (ambiguous `long` argument, D7)          |

No expected value is changed (`harness/ref/tinyjsonTests.patch` is the whole diff). Unpatched, GCC 16 stops at
`tinyjsonTests.cpp:5: fatal error: ..\src\tinyjson.h: No such file or directory`; with only patch 1 it stops at
`:70: conversion from 'long int' to 'tinyjson::json' is ambiguous`.

### Upstream tests on the reference build (measured)

Command (any cwd): `bash pilots/tinyjson/migration/harness/ref/build_ref.sh`, then
`pilots/tinyjson/migration/harness/work/ref/tinyjsonTests` (exit code 5, same result 10 runs in a row, and with
`-std=c++11`):

```text
test cases:  22 |  17 passed | 5 failed
assertions: 189 | 184 passed | 5 failed
```

The earlier claim "17 of 22, 5 failures from D3" is **confirmed**. The failing cases are `Tiny Json Hex Parsing`
(line 115), `Tiny Json Escape Parsing` (125), `Tiny Json Hex Char Parsing` (210: `U'$' == h1` printed
`36 == 1`), `Tiny Json Member Parsing` (331) and `SimpleJson Serialization` (382); each asserts the decoding of
a `\uXXXX` escape.

### D3 correction: the value is not U+0000 and not constant (measured)

RULEBOOK D3 says the Reference at `-O0` decodes every `\uXXXX` to U+0000. Measured: `["A"]` parsed by the
`-O0` driver gives **U+0001 in 71 of 100 processes** and U+0007 (5), U+0008 (4), U+0009 (6), U+000A (5), U+000B (7),
U+000C (2) in the others. Inside one process the value is the same for every `\u` (12 in one string, nested keys and
values); between processes it changes; filling the stack with a pattern before the call does not change it. U+0000
was never seen. Consequences: D3 needs a new value (modal U+0001) and a user decision; and the twice-run filter of
judge.md section 3.5 would drop `\u` inputs at random (first corpus run: 24 of 5521 inputs differed between two runs
of the same binary, all with `\u`). `dump.cpp` therefore pins the process: at start it decodes `A` and, when
the value is not U+0001, re-executes itself (counter in `TINYJSON_DUMP_TRY`, at most 60 times). With the pin, 200
runs of one `\u` input gave one md5. This pin is a driver-only measure; the Rust driver needs no port of it.

## Judge, C++ side (ticket 02, part 1)

Files (all under `pilots/tinyjson/migration/harness/` unless stated): `dump.cpp` (driver), `make_inputs.py`
(corpus), `ref/build_ref.sh`, `ref/tinyjsonTests.cpp` (patched copy), `ref/tinyjsonTests.patch`, `.gitignore`
(`work/`, `inputs/`, `inputs.manifest.tsv`; checked with `git check-ignore -v`), and
`pilots/tinyjson/experiments/e1_tinyjson_d5_chunk_rule.py`.

### Driver sections

Output records are `label = value` or `label ! class "what"`. Doubles print as IEEE bit patterns (NaN as `nan`);
text printed by the library (`to_string`) is printed as is. Sections: `parse`, `node` (every node: type, 7 getters,
size, 4 conversions, `to_string`, `has_member`, `operator[]` probes, copy, mutators on the copy, children),
`roundtrip`, `copy`/`assign`, `streams` (`U8ToU32`, `U32ToU8`, every public `parser::` function on a u32 stream
made from the UTF-8 decode and from the bytes as Latin-1, with the stream position after), `build` (values built by
constructors, `add_member`, `add_element`, mutation through `operator[]`), `matrix` (fixed values, all ordered
pairs).

`operator==` is only called on pairs whose C++ result is defined (`eq_safe`: D1 pairs print `skip`; int against
double is kept, D2). The Rust driver must port `eq_safe` line for line.

### Corpus

`make_inputs.py` (seed 20261012, reproducible: two runs, `diff -r` empty) writes 5534 inputs (64 byte-identical
duplicates skipped): valid 45, broken 147, num 129, pos 4078 (16 syntactic positions x 256 bytes), utf8 633 (incl.
offset sweeps of 6 invalid sequences over 0..70 filler bytes), deep 20 (depth up to 200), mut 482.

### Determinism filter and crashes

Command: `python3 -I skill/c-to-rust/scripts/harness.py --orig work/dump --rust work/dump --inputs inputs
--repeat 2 --jobs 8`.

| Run                   | Inputs | Same | Unstable                         | Exit code not 0 | Timeouts |
|-----------------------|--------|------|----------------------------------|-----------------|----------|
| first, without D3 pin | 5521   | 5511 | 14 (+10 differ between two runs) | 0               | 0        |
| with D3 pin (final)   | 5534   | 5534 | 0                                | 0               | 0        |

Dropped inputs after the pin: none. Crashing or hanging inputs: none (every exit code 0, no timeout). The 24
inputs that differed without the pin were all `\u` inputs (`valid_0020-0022`, `valid_0044`, `pos_16xx`,
`mut_xxxx`).

### Timing

One pass of the driver over the corpus: 349 s serial, 37 s with 8 parallel jobs (`xargs -P8`). One harness run
with `--repeat 2` (three driver runs per input), 8 jobs: about 115 s. The depth-200 inputs dominate the per-input
cost (1.3 s each: the dump is quadratic in depth).

### D5 chunk rule (experiment e1)

Exact, measured with the driver at the reference build. The input is delivered in windows of at most 29 bytes
(`_M_get_buf` is 32-3 bytes); a window may end in a valid but incomplete UTF-8 sequence, and the next window then
starts at those bytes. A window that holds an invalid byte (Unicode well-formed table, also for a sequence cut by
the window end) gives EOF at the **start of that window**: its characters are lost and nothing after is read.
ASCII prefix, FF at offset p: the surviving prefix is `29 * floor(p / 29)` bytes (123 offsets, 0 mismatches).
Model against driver on 20160 inputs (12 invalid sequences x 4 character widths x 0..5 chars x 70 offsets): 20160
agree. Not covered: chains of 5 or more windows each ending in a split character.

### Skill notes (C++ side)

1. judge.md section 3.5 (drop inputs whose output differs between runs) assumes nondeterminism is a property of the
   input. Here it is a property of the process (an uninitialised read), so the filter drops inputs at random and
   the corpus is flaky. judge.md needs a "pin or re-exec the original driver" step for C++ originals.
2. `harness.py` treats every regular file in `inputs/` as an input, so a manifest must live outside it.
3. `json(std::string&)` with an lvalue is ambiguous in tinyjson (`json(std::string&)` against
   `json(std::string)`): the lvalue constructor can never be called; not in the RULEBOOK.

## Correction to D3 (2026-10-12)

The "Decisions by the user" entry above says a `\uXXXX` yields U+0000 at `-O0`. That was my reading and it is wrong:
the governor re-measured with a 7-line probe (`["A"]`, GCC 16, `-O0`, 100 processes): U+0001 in 72, U+0007 to
U+000C in 28, U+0000 never. RULEBOOK §8 D3 and §12 now say U+0001 (modal, pinned by the driver's re-exec). The
reference build and the upstream result (22 cases, 17 passed, 5 failed; 189 assertions, 184 passed) were also
re-run by the governor and match the subagent's numbers.

## Judge, Rust side (ticket 02, part 2)

Written 2026-10-12. Every number below was produced by a command run in this session (logs in the session scratchpad,
report `migration/harness/work/diff.tsv`, mutant table `migration/harness/work/mutants/summary.tsv`).

### Files

| File (under `pilots/tinyjson/`)                | Role                                                                                                                                                          |
|------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `experiments/scratch-crate/`                   | STUB crate `tinyjson_scratch`, library name `tinyjson`, no dependencies: Stage 1 names of RULEBOOK §5, `Result<T, Error>` of §7, every body `todo!()`; builds |
| `migration/harness/rust/adapter.rs`            | the only file that calls the crate (`use tinyjson as tj`); plain types `J`, `S`, `AErr`, `R<T>`; Stage 1 version                                              |
| `migration/harness/rust/dump.rs`               | Rust driver, line-for-line port of `dump.cpp` (sections, labels, `eq_safe`, bit-pattern doubles, error class map)                                             |
| `migration/harness/rust/upstream.rs`           | the 22 upstream cases as `#[test]`s (`simple_json_constructor`, `tiny_json_hex_parsing`, ...)                                                                 |
| `migration/harness/run.py`                     | the runner (`--crate DIR`, `--skip-tests`, `--limit N`, `--jobs N`)                                                                                           |
| `experiments/e2_tinyjson_judge_sensitivity.py` | 11 one-character mutants of the C++ header against corpus and upstream tests                                                                                  |

**How the Rust side is shared between the stub and the real crate.** `harness/rust/*.rs` is the single copy. `run.py`
installs it into the crate being judged as `<crate>/tests/adapter/mod.rs` (adapter), `<crate>/examples/dump.rs` (driver,
`#[path = "../tests/adapter/mod.rs"] mod adapter;`) and `<crate>/tests/upstream.rs` (`mod adapter;`), as judge.md §5
lays out. For the scratch crate these three are git-ignored (`scratch-crate/.gitignore`); ticket 04 commits them in
`pilots/tinyjson/rust/` and the Gate protects them. A real crate must have library name `tinyjson` and offer the items
the adapter calls (the stub's `src/lib.rs` is the list); if the faithful port's signatures differ, only `adapter.rs`
changes. Cargo output goes to `migration/harness/work/cargo-<crate>/`.

### Commands

```bash
# the Judge on the stub (default crate); exit 0 = green
python3 -I pilots/tinyjson/migration/harness/run.py
# the Judge on a crate, translated tests skipped, about 600 inputs
python3 -I pilots/tinyjson/migration/harness/run.py --crate pilots/tinyjson/rust --skip-tests --limit 600
# the sensitivity experiment (about 95 s per mutant, one after the other)
python3 -I pilots/tinyjson/experiments/e2_tinyjson_judge_sensitivity.py [m01_hex_class ...]
```

`run.py` prints the Reference compiler first (`g++-16 (Homebrew GCC 16.1.0) 16.1.0`), builds the C++ driver with
`ref/build_ref.sh`, installs and builds the Rust side (`cargo build --example dump`), runs a port check (the string
literals of `dump.rs`, in order, must equal those of `dump.cpp` from the equality guard on: 138 identical), `cargo test
--test upstream`, then `harness.py --orig work/dump --rust <examples/dump> --jobs N`. A build error of the crate or a
driver is exit 2 (the Judge is broken); a panic of the Rust driver is a DIFF (exit code 101).

### Stub run (RED, as required by judge.md §4a item 3)

| Piece            | Result                                                                                                             |
|------------------|--------------------------------------------------------------------------------------------------------------------|
| port check       | 138 of 138 literals identical                                                                                      |
| translated tests | 0 passed, 22 failed of 22 (every `todo!()` panics)                                                                 |
| corpus           | 5534 DIFF of 5534 inputs; Rust driver exit 101 on all 5534, 1 output line (`input.size`) against hundreds to thousands of lines |
| runner           | reports diffs, exit 1, `JUDGE: RED`; no build error                                                                |
| first full run   | 60.3 s wall: C++ build 8.3 s, Rust build 0.1 s (cached), `cargo test` 1.1 s, harness 50.7 s (8 jobs)               |

### Validation beyond the stub (throwaway, not committed)

A stub cannot show that `dump.rs`, the adapter and the translated tests are correct, because after the first `todo!()`
nothing runs. So a rough 420-line Rust port of the header (copy in `migration/harness/work/rough-crate/`, git-ignored,
built from the scratchpad; not a deliverable and not a Faithful port: it has D5 as the e1 window model, D3 as constant
U+0001, a hand-made `strtod`) was run through the same `run.py --crate`. Result: **22 of 22 translated tests pass, 0
DIFF of 5534 inputs, `JUDGE: GREEN`**, 63.7 s. Its first run showed 2259 DIFF, all of them its own simplifications
(invalid UTF-8 without the window rule: 2201; a truncated trailing UTF-8 sequence accepted by `from_bytes`: 58); after
modelling those two, 0. So the driver port, the adapter and the corpus can be green on a plausible port, and the Judge
is red on the stub and on every mutant below. Ticket 04 should not treat the rough crate as the port.

### Mutants (experiment e2)

| Id  | One-character change in `tinyjson.h` (copy under `work/mutants/`) | Upstream tests (C++)                                      | Corpus (DIFF of 5534)         | Caught by   |
|-----|-------------------------------------------------------------------|-----------------------------------------------------------|-------------------------------|-------------|
| m01 | `isxdigit(c)` to `isdigit(c)` (hex digit class)                   | assertion count only (184 to 174)                         | 53                            | both        |
| m02 | `parse_hex` loop `i < 4` to `i < 3`                               | new failing case: SimpleJson String Parsing Failure       | 49                            | both        |
| m03 | `trim` default whitespace: TAB to VT                              | new failing case: Tiny Json Number Parsing                | 42                            | both        |
| m04 | `parse_number` ends at `[` instead of `]`                         | new failing case: Tiny Json Array Parsing                 | 3880                          | both        |
| m05 | `parse_value` starts a number at `,` instead of `.`               | survived                                                  | 25                            | corpus only |
| m06 | `escape_char` `case U'f'` to `U'F'`                               | new failing case: Tiny Json String Parsing                | 19                            | both        |
| m07 | `object_to_string` separator `" : "` to `" :"`                    | survived                                                  | 5534                          | corpus only |
| m08 | `to_string` boolean false prints `fals`                           | assertion count only (184 to 174)                         | 5534                          | both        |
| m09 | `array_to_string` comma `!=` to `==` (after last element)         | assertion count only (184 to 174)                         | 5534                          | both        |
| m10 | `operator[](int)` `>=` to `>` (index == size accepted)            | survived                                                  | 5534 (driver aborts, exit -6) | corpus only |
| m11 | `to_integer` tail check `!=` to `==`                              | new failing cases: at least 4 (Json Parsing 1, Array, Number, ...) | 1160                          | both        |

**11 of 11 caught by the corpus, 0 survived.** The C++ upstream tests alone catch 8 of 11 and miss m05, m07 and m10; for
m01, m08 and m09 they catch only through the assertion count, because the failing set is unchanged (the five D3 cases
already fail, and the mutant just makes SimpleJson Serialization stop earlier). The translated Rust tests assert the D3
value instead of failing, so for them these three mutants flip pass to fail. m02 was first planted as `4` to `5`; that
also made the driver's own D3 pin probe (`\u0041`) throw, all 5534 inputs differed trivially, and it was replaced by `4`
to `3`. Mutants were run on the C++ driver as the "Rust side" (same experiment, no translation noise); no Rust mutant
was run because the stub cannot be green to begin with.

### D3 in the translated tests

The five cases that fail on the reference (Hex Parsing, Escape Parsing, Hex Char Parsing, Member Parsing, SimpleJson
Serialization) keep the upstream expectation in a comment and assert U+0001 per `\u` escape, marked `DEVIATION(port):
D3`. No other expected value changed (the other 17 are copied as they are; SimpleJson Json Class Test uses `assign` for
`a2 = json(...)`). **Finding for RULEBOOK D3:** the value is not one constant inside the reference's own Catch binary. I
rebuilt `tinyjsonTests.cpp` with every D3 expectation set to U+0001 (scratch copy) and ran it 12 times: 19 of 22 pass,
identical each time; Hex Parsing gets U+0001 for the first three escapes and U+0000 for the 4th and 5th, Escape Parsing
U+0000 for both, and the SimpleJson Serialization key does not match (not found). The value is stack residue of the
tests that ran before. The pinned C++ *driver* gives U+0001 (396 corpus inputs with `\u`: only U+0001 in the decoded
strings). So U+0001 is the right value for the Differential harness, and the upstream-test D3 values are an
approximation that no Rust port can reproduce exactly.

### §5a classes

| Class                              | Status                   | Where / why                                                                                                                                                                                                                                                                                                                                                                                                                                     |
|------------------------------------|--------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Allocation failure (D14)           | not testable             | `new` throws `bad_alloc`; a Rust allocation failure aborts the process, which cannot be compared with an exception. The driver would need a replaced `operator new`; recorded as Deviation D14, the driver's `attempt` has a `bad_alloc` arm that never runs                                                                                                                                                                                    |
| Absent members of built values     | covered                  | `dump.cpp` section `build` and `matrix` and the 22 translated tests build values through every constructor, `add_member`, `add_element` and `operator[]`; absent keys and out-of-range indexes are probed on every node (`has_member(absent)`, `op[absent]`, `op[-1]`, `op[size]`). C++ has no NULL string or key inside a `json`; the NULL-pointer entries (`json((char*)nullptr)`, `parse(nullptr)`, `j[nullptr]`) are UB (D12): not testable |
| Input without a trailing NUL (Q10) | covered                  | the adapter passes the slice as it is, no NUL added or removed (`adapter::parse`); the C++ driver passes `c_str()`, so an embedded NUL ends the document on both sides; corpus classes `broken_` and `pos_` hold NUL bytes. The `json(char[])` case is cut at its NUL by the driver (`build.char_array`)                                                                                                                                        |
| Exceptions (class and `what()`)    | covered                  | every call goes through `attempt`; the classes `out_of_range`, `invalid_argument`, `runtime_error` are printed with the exact text. `range_error`, `bad_alloc`, `exception`, `unknown` never occur (the header rethrows everything as `runtime_error`)                                                                                                                                                                                          |
| Copy semantics                     | partly                   | copy constructor, `operator=`, chain assignment, assignment of null, `get_object`/`get_array` copies (Q19) and mutation of a copy leaving the original unchanged: covered (`copy`, `assign`, `node`, `build` sections). Self-assignment `a = a` is not exercised (the adapter's `assign(&mut J, &J)` cannot express it in Rust)                                                                                                                 |
| UB pairs of `operator==` (D1)      | not testable on C++ side | `eq_safe` skips them; the Rust port returns false there (D1). RULEBOOK §10 lists extra Rust-only tests (D1 pairs false, Q1, D2 values, Q4, Q5, Q9): not written in this ticket; Q1, D2, Q4, Q5 are covered by the differential matrix and round trip, **D1 has no test**                                                                                                                                                                        |
| Locale (D8, D9)                    | not testable             | both sides run in the C locale (the driver never calls `setlocale`); a Rust crate has no locale                                                                                                                                                                                                                                                                                                                                                 |
| Deep nesting (D11)                 | partly                   | depth up to 200 in the corpus (`deep_`); depth 100000 crashes the C++ side and is deliberately absent                                                                                                                                                                                                                                                                                                                                           |

### Notes for the Skill session

1. judge.md §4 and §5 do not say how one Rust driver/adapter/test set serves both a scratch crate and the real crate.
   Used: single copy in `harness/rust/`, installed into the crate by `run.py`, `[lib] name = "tinyjson"` in the scratch
   crate so the adapter's `use tinyjson as tj` is the same. A rule for this would help every C++ Pilot.

2. A stub proves nothing about the driver port (execution stops at the first `todo!()`). A cheap port check (ordered
   string literals of the Rust driver equal the C++ driver's) caught nothing here but would catch drift; a throwaway
   rough port did the real validation. Suggest §4a mention both for the C++ route.

3. judge.md §6 "one run at a time" held; the e2 experiment is 11 mutants x about 95 s = 17 minutes, most of it the
   corpus on mutants the upstream tests already catch. judge.md says to skip the corpus then, but this ticket asked
   which piece catches what, so all were run on both.

4. `harness.py` detail strings are the whole differing line (up to several kB for `streams.*.src` records); a
   `--detail-width` option would keep logs readable. `run.py` truncates to 160 characters itself.
