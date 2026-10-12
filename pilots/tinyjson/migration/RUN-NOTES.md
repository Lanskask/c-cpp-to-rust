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

## Rule proposals

Ticket 03 stress test (2026-10-12): groups A (`parser`) and B (`operator==`, copy, destructor) translated twice by
Haiku, group C once at first (C2 launch was refused by the auto-mode classifier). Review by Opus:
`pilots/tinyjson/experiments/stress_test_diff_review.md` (differences, evidence, verdict). C2 ran later; its review is
the section "Group C: second run (C2)" of the same file and adds P11. Status: P1-P10 accepted by the user on
2026-10-12 and applied in the RULEBOOK (each one cited by `(Pn)` at the end of the amended cell or sentence, and listed
in RULEBOOK §12); P11 applied, the user may veto it. The user also decided: P6 underflow is accepted as written;
§4 now forbids markers outside its list and requires the exact `DEVIATION(port): D<n>` spelling.

### P1. `operator==` fallthrough shape (B-1) — §2, amends row C4

Status: accepted 2026-10-12, applied in RULEBOOK §2, row C4 (Override cell replaced).

Existing row:

```text
| C4         | `operator==` keeps its switch fallthrough as an explicit chain in case order `array, object, null, string, boolean, number_integer, number_double, default`; never merge or reorder the arms (Q1, D1, D2)                      | the fallthrough decides the result for mismatched types                           |
```

Proposed text (replaces the Override cell):

> `operator==` keeps its switch fallthrough as an explicit chain in case order `array, object, null, string, boolean,
> number_integer, number_double, default`; never merge or reorder the arms (Q1, D1, D2). A Rust `match` arm cannot
> fall through, so write the chain as `if`/`else if` over `(self._type, o._type)` pairs, not as one `match` arm per
> case that returns `false` on a mismatch. The complete result table: same type → compare the payloads; left `array`
> or `object`, right `null` → `true` (Q1); left `number_integer`, right `number_double` → D2 bit-pattern compare;
> every other pair → `false` (right type earlier in case order: `default`; later: D1). The `invalid` arm returns
> `false` like `default:`, it does not panic.

### P2. Code-point strings stay `u32` until `U32ToU8` (A-2) — §2.1, amends row CPP26

Status: accepted 2026-10-12, applied in RULEBOOK §2.1, row CPP26 (Stage 1 cell extended; status column `accepted`).

Existing row:

```text
| CPP26 | `char32_t` input streams: `u32_istream = std::basic_istream<char32_t>`, `u32_sstream = std::basic_stringstream<char32_t>` (`tinyjson.h:14-15`); every parser function takes `u32_istream&` and uses `peek()`/`get()` with EOF `0xFFFFFFFF` (`660-1075`)                                                                                                                                             | `pub struct u32_sstream { buf: Vec<u32>, pos: usize }` with `new(&[u32])`, `peek() -> u32`, `get() -> u32`, `EOF: u32 = u32::MAX`; parser functions take `&mut u32_sstream`                                                                               | `U32Stream`, same methods; stays public (tests call the parser functions directly)                                                                              | the position after an error is observable (Q13); the C++ `int c = peek()` turns EOF into `-1` (D7)                                                                                                                               | proposed |
```

Proposed addition to the Stage 1 cell:

> A local `std::u32string` is a `Vec<u32>` and is turned into bytes only where C++ calls `U32ToU8` (`parse_member`
> h:769, `parse_bool` h:822, `parse_null` h:841, `parse_string` h:881, `parse_number` h:897). Never cast a code point
> to `u8` (`c as u8`): it drops the UTF-8 encoding and the `U32ToU8` error.

### P3. `tolower` on a code point (A-3) — §2, amends row L4, L5, L6

Status: accepted 2026-10-12, applied in RULEBOOK §2, row L4, L5, L6 (Override cell extended).

Existing row:

```text
| L4, L5, L6 | `std::isxdigit`, `std::isspace`, `std::tolower` are called on `char32_t` code points: ASCII C-locale rules, and every value above 0x7F is "no" / unchanged (D10). Whitespace is `\t \n \v \f \r` and space (L5: `\v` included) | `tinyjson.h:818`, `837`, `917`, `1044`, `1059`                                    |
```

Proposed addition to the Override cell:

> The code form is fixed: `fn tolower32(c: u32) -> u32 { if c <= 0x7F { (c as u8).to_ascii_lowercase() as u32 }
> else { c } }`, `fn isspace32(c: u32) -> bool { matches!(c, 9..=13 | 32) }`, `fn isxdigit32(c: u32) -> bool { c <=
> 0x7F && (c as u8).is_ascii_hexdigit() }`. Never `(c as u8).to_ascii_lowercase()` without the `<= 0x7F` test, and
> never Rust `is_ascii_whitespace` (it lacks `\v`).

### P4. `to_bool` lowercases again (A-4) — §2.1, amends row CPP30

Status: accepted 2026-10-12, applied in RULEBOOK §2.1, row CPP30 (trap cell extended; status column `accepted`).

Existing row:

```text
| CPP30 | lambda in `std::transform` (`tinyjson.h:913-918`)                                                                                                                                                                                                                                                                                                                                                   | `for b in &mut s { *b = b.to_ascii_lowercase(); }`                                                                                                                                                                                                        | same                                                                                                                                                            | —                                                                                                                                                                                                                                | proposed |
```

Proposed addition to the trap cell:

> `to_bool` is public and the Judge calls it on raw input: keep the lowercasing even though `parse_bool` already
> lowercased its token. A statement that looks redundant in its only internal caller is still ported.

### P5. `stoll` scanner (A-5) — §2, amends row I9

Status: accepted 2026-10-12, applied in RULEBOOK §2, row I9 (Override cell replaced).

Existing row:

```text
| I9         | `std::stoll` → hand-written `strtoll` scanner (base 10) + `i64` with `out_of_range` on overflow (D8)                                                                                                                           | `tinyjson.h:929-941`                                                              |
```

Proposed text (replaces the Override cell):

> `std::stoll(str, &pos)` → `fn stoll(s: &[u8]) -> Result<(i64, usize), Error>` on bytes (no UTF-8 check, no
> `trim`): skip leading C `isspace` bytes (`\t \n \v \f \r` and space), one optional `+` or `-`, then decimal digits.
> No digit → `Err(InvalidArgument, "stoll")`. Accumulate in `i128` (or as a negative number) so that
> `-9223372036854775808` parses; a value outside `i64` after all digits are read → `Err(OutOfRange, "stoll")`.
> `pos` = index after the last digit. `to_integer` then returns `Err(RuntimeError, "Unexpected number(integer)
> format.")` when `pos != s.len()` (h:935-938). Measured (probe): `" 5 "` → RuntimeError, `""` and `"-"` →
> InvalidArgument, `99999999999999999999` → OutOfRange, `"\v5"` → 5, i64 min → OK.

### P6. `stod` scanner (A-6) — §2, amends row I8

Status: accepted 2026-10-12, applied in RULEBOOK §2, row I8 (Override cell replaced; the underflow rule accepted as
written).

Existing row:

```text
| I8         | `std::stod` → hand-written `strtod` prefix scanner + `f64::from_str`, with the `pos`, `invalid_argument` and `out_of_range` rules of D8                                                                                        | `tinyjson.h:943-954`                                                              |
```

Proposed text (replaces the Override cell):

> `std::stod(str, &pos)` → `fn stod(s: &[u8]) -> Result<(f64, usize), Error>` on bytes, never `trim` +
> `f64::from_str` of the whole token: skip leading C `isspace`, one optional sign, then the longest prefix of one of:
> `inf`/`infinity`/`nan`/`nan(` [alnum `_`]* `)` (ASCII case-insensitive); `0x`/`0X` hex float (hex digits, optional
> `.`, optional `p`/`P` exponent that is consumed only if digits follow); decimal digits with optional `.` and
> optional `e`/`E` exponent consumed only if digits follow. No digit consumed → `Err(InvalidArgument, "stod")`. The
> value is `f64::from_str` of the decimal prefix (hex: exact computation, correctly rounded). Finite digits that give
> ±inf, or non-zero digits that give 0 or a subnormal → `Err(OutOfRange, "stod")` (D8 measured `1e400`, `1e-310`,
> `1e-400`). `to_double` then returns `Err(RuntimeError, "Unexpected number(double) format.")` when `pos !=
> s.len()`. Probe: `1.5x` → RuntimeError, `abc` → InvalidArgument, `1e400` → OutOfRange.

### P7. Stub signatures are binding (A-7) — §5, amends the second bullet

Status: accepted 2026-10-12, applied in RULEBOOK §5, second bullet (sentence inserted before the table lead-in).

Existing bullet:

```text
- Stage 1 keeps the C++ names (`json`, `json_t`, `parser`, `u32_sstream`, `get_string`, `U32ToU8`, …) with
  `#![allow(non_camel_case_types, non_snake_case)]`. Overloads and operators get these names (proposed):
```

Proposed sentence appended to the bullet:

> The signatures in the stub crate (`experiments/scratch-crate/src/lib.rs`) are binding in Stage 1: never change a
> parameter type, return type, name or visibility, because `harness/rust/adapter.rs` compiles against them (for
> example `get_next_non_space -> u32`, `peek_next_non_space -> i32`); compare with `-1i32` or `u32::MAX` at the call
> site instead.

### P8. D5 carries the measured window rule (A-1) — §8, amends row D5

Status: accepted 2026-10-12, applied in RULEBOOK §8, row D5 (Rust-behaviour cell replaced).

Existing row:

```text
| D5  | Input decoding through `wbuffer_convert` (`tinyjson.h:630-633`): invalid UTF-8 (a stray byte, an overlong form, an encoded surrogate, a code point above U+10FFFF) is not an error but an early EOF, at the start of the libstdc++ conversion chunk that holds it. Measured: `["` + 26 `a` + `\xff"]` (32 bytes) fails `invalid json format` (the first `peek` is already EOF); with 27 `a` it fails `expected char '"' not found`; a valid document followed by garbage about 40 bytes later parses OK. A UTF-8 BOM is not skipped (U+FEFF is not whitespace) in both                                                                                                  | emulate the libstdc++ `wbuffer_convert` chunk rule so the early EOF lands at the same byte as in the Reference. The exact rule is measured with the C++ driver in ticket 02 before Stage 1. If it cannot be reproduced exactly, fall back to ending the input at the first invalid byte and record the remaining mismatch as a new Deviation row; `DEVIATION(port): D5`                                                                                                                              | yes: corpus byte classes 0x80-0xFF, compared byte-exactly (error text and parse result)                                                                                                                                                                                                                                                                                                                                                                                                                   | approved (user rule: match C++ origin as closely as possible) |
```

Proposed text (replaces the Rust-behaviour cell):

> Emulate the libstdc++ `wbuffer_convert` window rule (measured exactly, experiment e1, 20160 inputs, 0 mismatches):
> `parse` does not call `U8ToU32`. It decodes the bytes before the first NUL in windows of at most 29 bytes; window 0
> starts at byte 0. A window is decoded as a whole with the Unicode well-formed table (no overlongs, no surrogates,
> nothing above U+10FFFF). If its last bytes are a valid but incomplete sequence of `u` bytes, the next window starts
> at those bytes and holds them plus `29 - u` new bytes. If a window holds an ill-formed byte (also a sequence cut by
> the window end that is ill-formed so far), the stream ends at the **start of that window**: every character of that
> window is dropped, nothing after it is read, no error is raised. An incomplete sequence at the very end of the input
> is dropped. `DEVIATION(port): D5`.

### P9. `U8ToU32` / `U32ToU8` error texts (A-8, S-2) — §2.1, amends row CPP28

Status: accepted 2026-10-12, applied in RULEBOOK §2.1, row CPP28 (Stage 1 cell replaced; status column `accepted`).

Existing row:

```text
| CPP28 | `std::wstring_convert<std::codecvt_utf8<char32_t>, char32_t>` `to_bytes`/`from_bytes` (`U32ToU8`, `U8ToU32`, `tinyjson.h:42-68`)                                                                                                                                                                                                                                                                    | hand-written UTF-8 encoder/decoder with the C++ error message (`"invalid utf32 string: "` + library text, D4)                                                                                                                                             | same                                                                                                                                                            | a surrogate code point (`0xD800-0xDFFF`) or a value above `0x10FFFF` fails (D4); `U8ToU32` has no caller in the header but is public: port it                                                                                    | proposed |
```

Proposed text (replaces the Stage 1 cell):

> Hand-written UTF-8 encoder/decoder. `U32ToU8` fails on a surrogate or a value above `0x10FFFF` with
> `Err(RuntimeError, "invalid utf32 string: wstring_convert::to_bytes")`. `U8ToU32` decodes with the Unicode
> well-formed table and fails on any ill-formed sequence with `Err(RuntimeError, "invalid utf32 string:
> wstring_convert::from_bytes")`; a valid but incomplete sequence at the very end is dropped without error (probe:
> `a\xFF` fails, `a\xC3` → `[U+0061]`).

### P10. Q5 is code, not a comment (C-1) — §9, amends row Q5

Status: accepted 2026-10-12, applied in RULEBOOK §9, row Q5 (Behaviour cell extended).

Existing row:

```text
| Q5  | Inside `object_to_string`/`array_to_string` keys and nested results go through `.c_str()`, so they stop at the first NUL: `["a\u0000b"]`-like content prints as `["a]` (measured with a `std::string` holding a NUL); a top-level string keeps the NUL (BUG)                                                                                                                                                                                                                                                               | `tinyjson.h:582`, `586`, `606` vs `555`                  |
```

Proposed addition to the Behaviour cell:

> Stage 1 code: a helper `fn c_str(s: &[u8]) -> &[u8]` (the bytes before the first NUL) applied to the key at h:582
> and to the nested `to_string()` result at h:586 and h:606, not to the top-level string at h:555. A `BUG(port)` or
> `QUIRK(port)` marker never replaces reproducing the behaviour.

### P11. Stacked markers (C-4, C2-7) — §4, amends the markers paragraph

Status: applied 2026-10-12 in RULEBOOK §4 (second paragraph after the marker list); cosmetic; the user may veto it.
Full text in `pilots/tinyjson/experiments/stress_test_diff_review.md`, section "Group C: second run (C2)".

## Stress test round 2 plan

State on 2026-10-12: RULEBOOK amended (P1-P11 and the §4 marker rule, §12); stub fixed (`json` derives `Debug` only,
`impl Clone for json` is a hand-written `todo!()` per CPP14; `run.py --skip-tests --limit 50` builds and is RED by
diff, 50 of 50); corpus extended for P3 and P4 (27 new `broken_` inputs `broken_0149`-`broken_0175`, 5561 inputs, the
twice-run filter `harness.py --repeat 2` on the C++ driver: 5561 same, 0 unstable, 0 dropped, every exit code 0).

1. Group prompts give the **signatures only** of the stub functions outside the group; their bodies stay `todo!()`
   and the agent must not fill them (answer to open question 1 of the review).
2. Round 2 reruns A, B and C once each: three Haiku runs (A3, B3, C3) against the amended RULEBOOK, same group
   slices as round 1, one run at a time.
3. Review each round 2 run against its round 1 pair (A1 or A2, B1 or B2, C1 or C2) with the same table as
   `stress_test_diff_review.md`.
4. Success: no pair differs on any BEHAVIOUR item, or every remaining difference is classified (BEHAVIOUR with a
   rule proposal, DEVIATION-RELATED, or COSMETIC). Success closes the ticket 03 acceptance item "a same-model rerun
   agrees", and the RULEBOOK status line can then become `stress-tested`.

### Round 2 result (2026-10-12)

A3 (Haiku), B3 and C3 were reviewed against their round 1 pairs; every BEHAVIOUR item they wrote agrees with the
faithful round 1 run and the C++ probe, the rest is COSMETIC or DEVIATION-RELATED, and no P12 was needed. A3 left
`stoll`/`stod` as `todo!()`, so group A was rerun with `stage1-prompt.md` HARD RULE 9: the Haiku run A3b
(`work/stress/R2b-A3`) wrote a `stod` that is wrong on 3 inputs (`1e-310` returned `Ok`, `0x1p3` gave
`invalid_argument`, `-inf` lost its sign) although I8 states all three. A Sonnet 5.5 run (`work/stress/R2c-A3`, only
`stoll`/`stod` rewritten) matched GCC 16.1 libstdc++ on 5257 inputs with 0 mismatches. The remaining differences are
therefore classified as agent-tier, not a rule gap: the user decided (2026-10-12) to send the `strtod`/`strtoll`
scanners to Sonnet (RULEBOOK §0 tier bullet, §12), and the subnormal and `nan(` facts measured in R2c-A3 were written
into I8 and D8. This closes the plan's success criterion (item 4, second form); the RULEBOOK status is now
`stress-tested (2026-10-12)`. Evidence: `pilots/tinyjson/experiments/stress_test_diff_review.md`, section "Round 2".

## Review of 7115bf6 (ticket 04, Sonnet reviewer a6f12a13c115b598e) - triage, 2026-10-12
Fixed in attempt 5: `parse_value` cast the code point to `u8` (P2); `assign` leaked the old payload (D13);
`index_key` looked up the full slice instead of `std::string(key)` (Q5); the five `U32ToU8` sites encoded per code
point instead of once at the end (D4); markers spelled `MARKER: Id` on their own line above the site, trailer
`deviations=` now equals the DEVIATION ids in the file (D1,D2,D3,D4,D5,D8,D9,D13).

### Corpus gaps (Judge change, own commit, not done: ticket 04 stays `in_progress` for the Part)
- `[` + `C4 A2` + `]` and `EF BD 9B` (U+FF5B), `C5 94`, `C5 AE`: a code point whose low byte is a value start char.
- `utf8_edge_*`: `["` + 26x`a` + `C3 A9` + `"]`; `["` + 26x`a` + `FF 22 5D`; `["` + 27x`a` + `FF 22 5D`;
  `["` + 25x`a` + `F0 9F 98 80` + `"]`; `["` + 25x`a` + `F0 9F 28 22 5D`.
- `%f` ties: `[0.0078125]` (`0.007812`), `[0.0000005]`, `[-0.0000001]`, `[1e22]`, `[4.35]`.
- driver: `index_key` with an embedded NUL key; `parse_string` on the hand-built stream `[0x22, 0xD800, 0x22]`
  (C++ `invalid utf32 string: wstring_convert::to_bytes`).

## Rule proposals
- RULEBOOK §4: "Until the stress test fixes the exact sites, a marker goes on every source site cited in §8 and §9"
  is not decidable by `gate.py`. Proposed: "A marker goes on every statement where the Rust code reproduces a QUIRK or
  implements a DEVIATION. The PORT STATUS `deviations=` field is the set of D ids with at least one
  `DEVIATION(port): D<n>` line in the file."
- CPP26: the buffer field is `buf` and the position `_pos` (the rulebook says `pos`); amend the row to the field names
  the Judge adapter and the skeleton use.
- Q2, Q6-Q10, Q12-Q20 and D6, D7, D10-D16 have no marker in the file: they describe behaviour with no single statement.

## Stage 2 Judge validation (ticket 05, 2026-10-12)
- Adapter `harness/rust/adapter_stage2.rs` calls the RULEBOOK §6 API; `harness/rust/stage2_shim.rs` implements §6 over
  the Faithful port for validation only (both `mod stage2_shim; use stage2_shim as tj;` lines become
  `use tinyjson as tj;` when the Stage 2 crate lands; the shim is then deleted). `run.py --stage 2`.
- Faithful port, STAGE=2: 22/22 translated tests, corpus 0 DIFF of 5755, JUDGE GREEN (about 65 s). STAGE=1 still GREEN.
- Mutant: `WINDOW: usize = 29` -> `30` in `wbuffer_convert_decode` (D5): translated tests 22/22 (they do not see it),
  corpus 13 DIFF of 5755 (`utf8_0207`, `utf8_0278`, ...), JUDGE RED; restored, `git diff` clean.
- Shim cost: a deep input (nesting about 400, `deep_0019`) timed out with a fully converting shim, so the shim converts
  one level for scalar getters; the Stage 2 crate has no such cost.

### §5a: what the corpus never feeds
| Class                         | Status        | Where / why                                                                        |
|-------------------------------|---------------|------------------------------------------------------------------------------------|
| Allocation failure            | not testable  | D14: Rust aborts on OOM, there is no recoverable path to compare                   |
| Absent members of built values| not applicable| no NULL string or key exists in the §6 API (`&[u8]`, `Vec<u8>`); D12 covers C++ UB  |
| Input without a trailing NUL  | covered       | `Parser::parse(&[u8])` takes the slice as is; the NUL is part of the input (Q10), `broken_` NUL inputs |
| Surrogate / > U+10FFFF stream | covered       | `streams.wide` probes (Judge commit b9d53af)                                        |
| Code point above U+00FF at a dispatch | covered | `broken_` P2/P3 inputs (Judge commit b9d53af)                                     |
| Deep nesting                  | partly        | `deep_` up to 200; D11 (100000 deliberately absent)                                |

## Stage 2 review of attempt 1 (ticket 06, Sonnet reviewer a7dc33c2a1f542bca), 2026-10-12
Fixed by the governor before the commit: D8 markers on `stoll`/`stod` (the trailer listed D8 without a marker), Q5 pair on
`get_key`, Q1/Q4 markers, D3 marker on its own line, stale doc comments, two `PERF(port)` lines that recorded no
problem, `U32Stream::peek` back to `&mut self` (§6), and `skip_char` now builds its message as `u32` and calls
`u32_to_u8` (D4: a surrogate or a value above U+10FFFF gives `invalid utf32 string: wstring_convert::to_bytes`, as
C++; this was inherited from Stage 1 and is not reachable through `parse`).
Not done, left open:
- GAP: the adapter calls only `get_key_mut` and `get_index_mut`; `get_key` and `get_index` (§6, read-only forms) are
  never run. By reading they match. Proposed test: `{"a":1}`, `get_key(b"a\0zz")` is `Ok(1)`, `get_key(b"b\0c")` is
  `key b not found.`; `[1]`, `get_index(1)` and `get_index(-1)` are `index 1 out of range.` / `index -1 out of range.`.
- GAP: `skip_char` on a surrogate is not in the corpus (public API only): input `skip_char(U32Stream::new(&[0x61]),
  0xD800)`; the `streams.wide` probes cover surrogates in the stream, not as `expected`.
- NIT: `object_to_string`/`array_to_string` take `&Json` and re-match the tag (C `void*` leftover); `scan_*` has a
  `let _ = idx` artefact (scanner code is verified separately, §0).
Gate: 8 of 8 (check, test, harness, clippy, fmt, unsafe, unwrap, allow); 22/22 tests; corpus 0 DIFF of 5755.
