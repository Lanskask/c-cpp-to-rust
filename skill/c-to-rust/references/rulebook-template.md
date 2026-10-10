# Rulebook template

Copy to `pilots/<name>/migration/RULEBOOK.md` in Stage 0 and fill every `[...]`. Structure adapted from
`anthropics/code-migration-kit-with-claude-code` `templates/RULEBOOK.md` (Apache-2.0), changed for a
two-stage C/C++ → Rust port with a per-Part gate. Filled example: `pilots/jsmn/migration/RULEBOOK.md`.

**Meta-rule: if two agents could answer a question differently, the answer goes in the rulebook.**
Every implementer, reviewer and fixer reads the sections their prompt names before touching code.
Inside a batch the rulebook is **read-only**: proposals go to `migration/RUN-NOTES.md` under `## Rule proposals` (the
deny-rules in `assets/settings.json` block edits to `RULEBOOK.md`) and are applied to §12 by Opus (or the user) between
batches.

---

```markdown
# RULEBOOK: <pilot> (<upstream url> @ <short commit>)

Status: draft | deviations-approved (<date>) | stress-tested (<date>)
Idiom table: skill/c-to-rust/references/idiom-table-c.md (rows apply unless §2 overrides them)

## 0. Scope and posture

- Stage 1 (Faithful port): same structure, same names, C ABI (`#[no_mangle] extern "C"`, `#[repr(C)]`)
  for every public symbol, `unsafe` allowed. You are translating, not improving.
- Stage 2 (Idiomatic pass): the API in §6, zero `unsafe`, acceptance rules in §10. Same behaviour, not
  same API.
- Engine for Stage 1: [c2rust fast path | LLM]. [If c2rust: one run per configuration in §3, merged by hand.]
- Run `cargo check` in the inner loop; `gate.py` is the only referee for acceptance.

## 1. Dependencies and ecosystem

| Area          | Decision                                                      | Why                          |
|---------------|---------------------------------------------------------------|------------------------------|
| Crates        | [default: none; `libc` allowed in Stage 1 only]               |                              |
| Bytes vs text | [e.g. `&[u8]` everywhere; `&str` only where C requires UTF-8] |                              |
| Toolchain     | [stable                                                       | pinned nightly, decision 25] | |

## 2. Overrides of the idiom table

One canonical mapping per construct. Rows not listed here apply as written in the idiom table.
| Idiom row | Override for this Pilot | Reason |
|---|---|---|
| [T6] | [...] | [...] |

## 3. Configuration macros → cargo features (decision 24)

Every `config-macro:NAME` tag in `inventory.tsv` gets one row.

- A macro that changes behaviour or data layout → a cargo feature, guarded with
  `#[cfg(feature = "...")]`; the Judge runs every combination the upstream tests use.
- A macro that only changes linkage or packaging (static/inline/header-only/export) → no feature;
  expressed with `pub` / private items (idiom M4).
- A platform macro → `#[cfg(target_os = ...)]`; untested platforms go to §8.
  | Macro | Kind | Cargo feature | Judge runs it | Notes |
  |---|---|---|---|---|
  | [NAME] | behaviour / layout / linkage / platform | [feature or —] | [yes/no] | |

## 4. Escape hatch and markers

- Stage 1: `unsafe` wherever the C code dereferences pointers; each `unsafe` block or fn that is not a
  direct pointer translation carries `// SAFETY: <why>`.
- Stage 2: none. If a construct seems to need `unsafe`, stop and propose a rule (`RUN-NOTES.md`, `## Rule proposals`).
- Greppable markers, exact spelling (they are the queue for later work):
  `TODO(port): <open question>`, `PERF(port): <faster version>`, `BUG(port): <repro>` (defective
  original behaviour reproduced on purpose), `QUIRK(port): <rule id>` (defined but surprising behaviour
  kept, see §9), `DEVIATION(port): <D-id>` (see §8).
- **UNKNOWN rule:** when neither this rulebook nor the idiom table decides a case, take the most
  conservative mapping, mark `TODO(port):`, and keep going.
- **BUG rule:** defective original behaviour is reproduced, not fixed. Fixes are separate,
  flagged changes after the Pilot passes.

## 5. Naming and paths

- Part → target file: [`rust/src/<stem>.rs`; a one-Part crate uses `rust/src/lib.rs`].
- Names: Stage 1 keeps C names (`#![allow(non_camel_case_types)]`); Stage 2 uses Rust naming.
- Translated tests: `rust/tests/<upstream test file stem>.rs`; Stage 2 driver: `rust/examples/dump.rs`;
  both reach the crate only through the Judge adapter (see `judge.md`).
- Shared types live in [module]; nothing is defined twice.

## 6. Stage 2 public API

Decided here once, before Stage 2 starts. Signatures, types, error enum, how each C mode/sentinel maps.
[...]

## 7. Error model

- C error codes → `enum Error` with `fn code(self) -> i32` returning the exact C value (idiom E1).
- Each guard is classified: **allocation-guard** (malloc failed: unreachable in Rust, §8) or **precondition-guard** (bad
  input or too-small buffer: kept, same error, same state left behind).
- Partial state after an error (positions, tokens written) stays readable (idiom E4).

## 8. Deviations (decision 11)

UB and platform-specific behaviour where Rust will not match the original. The user approves or vetoes
each row before Stage 1 fan-out (SKILL.md step 1). Code sites carry `DEVIATION(port): D<n>`.
| id | Original behaviour | Rust behaviour | Reachable by Judge? | Status |
|---|---|---|---|---|
| D1 | [...] | [...] | [yes/no] | proposed / approved / vetoed |

## 9. Kept quirks

Defined behaviour that looks wrong but is Same behaviour, so it is reproduced (`QUIRK(port): Q<n>`).
| id | Behaviour | Where |
|---|---|---|

## 10. Judge and acceptance

- Feature matrix: [...]. Gate: `make pilot-<name> STAGE=<n>`.
- Stage 1: cargo check/test, original tests linked to the staticlib (C only), Differential harness.
- Stage 2: translated tests, Differential harness with the Rust driver, clippy `-D warnings`,
  rustfmt, zero `unsafe`, no `unwrap()`/`expect()` on input-driven paths (others need an invariant
  comment) (decision 10).

## 11. Per-file obligations

Every translated file ends with:
`// PORT STATUS: stage=<1|2> confidence=<high|medium|low> todos=<N> deviations=<D-ids or ->`

## 12. Amendments

Accepted rule changes, applied between batches from `RUN-NOTES.md` `## Rule proposals`.
| date | proposer | section | proposal | decision |
|---|---|---|---|---|
```
