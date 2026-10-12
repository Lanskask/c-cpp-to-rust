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
