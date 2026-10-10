# RUN-NOTES: jsmn

## Stage 2 Judge validation (E3-prep, 2026-10-10)

Judge pieces (judge.md §5): `rust/tests/adapter/mod.rs` (Stage 1 adapter, `unsafe` calls into the C ABI),
`rust/examples/dump.rs` (line-for-line port of `dump.c`), `rust/tests/upstream.rs` (all 16 upstream tests).
Run: `python3 -I pilots/jsmn/migration/harness/run.py --stage 2 [FEATURES]`.

On the Faithful port, all numbers from commands run in this session:

| feature set | translated tests | harness (C dump.c vs Rust dump) | E2 mutant `32..127`→`32..126` |
|---|---|---|---|
| default | 16/16 | 1188/1188 | DETECTED (1186/1188) |
| strict | 16/16 | 1188/1188 | DETECTED (1187/1188) |
| parent-links | 16/16 | 1188/1188 | DETECTED (1186/1188) |
| strict,parent-links | 16/16 | 1188/1188 | DETECTED (1187/1188) |

Stage 1 Judge also still passes on all four sets with the enlarged corpus (`make pilot-jsmn`; upstream C tests
`FAILED: 0`, harness 1188/1188).

**Finding: the Stage 1 corpus was blind to the E2 mutant in both `strict` sets.** With the 932-input corpus the
mutant was DETECTED only in `default` and `parent-links` (strict sets: 932/932 match). Reason: in strict mode a
primitive must start with one of `-0-9tfn`, so a byte placed at the *start* of a primitive (`[<c>a]`, the only
primitive position the corpus covered) is rejected before the per-byte range check; `~` is only reached as a
*continuation* (`[1~]`). Fix: `make_inputs.py` adds `[1<c>]` for every byte value (+256 inputs, 1188 total).
Lesson for judge.md §3 item 3: enumerate each byte class in every position **per feature set**, not once.
Not yet applied to RULEBOOK §10 (still says 932 inputs); Opus to update.

## Stress test (SKILL.md step 2): INCOMPLETE, one of two translations

Prompt builder: `experiments/e3_jsmn_stress_prompt.py` (37 KB prompt: rulebook §0,2,3,4,5,7,8,9,11, 75 idiom rows
tagged `*`/`goto`/`config-macro:*`, c2rust `none`). Scratch crates under the session scratchpad, not in the repo.

- Translation A (Haiku subagent): 542 lines, `cargo check` clean in all four sets, gated with
  `run.py --crate <A>` in all four sets: upstream tests `FAILED: 0`, harness 1188/1188 each.
- Translation B: **not produced.** Two attempts to start the second Haiku subagent were refused by the Claude
  Code auto-mode classifier ("Auto-Mode Bypass", then "Create Unsafe Agents"). No workaround was attempted.
  Without B there is no two-agent diff, so the stress test is not done and the rulebook status line must not
  say `stress-tested`.

## Rule proposals

Provisional. They come from reading A against the c2rust-merged Faithful port and the rulebook, **not** from a
two-agent diff, so each is a candidate only; they still need B (or a second A-run) to show agents disagree.

| id | proposal | evidence (A's choice) | why it matters |
|---|---|---|---|
| RP1 | §2 override for T8, Stage 1: keep `type jsmntype_t = c_uint` plus `pub const JSMN_*` (as idiom T8 Stage 1 says). Say explicitly that §2 "TokenKind keeps C discriminants" is Stage 2 only | A made `#[repr(i32)] pub enum jsmntype_t` and used it as a `#[repr(C)]` field | A Rust enum field holding a value C wrote outside 0,1,2,4,8 is UB; the original tests link to the Rust crate and pass in-range values only, so the Judge cannot see it |
| RP2 | §0/§5: Stage 1 `jsmn_parse` signature is exactly `(parser: *mut jsmn_parser, js: *const c_char, len: size_t, tokens: *mut jsmntok_t, num_tokens: c_uint) -> c_int` | A used `js: *const u8` and `num_tokens: usize`; C declares `unsigned int num_tokens` | `usize` for a 32-bit C parameter reads undefined upper register bits on x86-64/arm64; passes here by luck of the calling convention |
| RP3 | §4: Stage 1 does not turn raw pointers into `&mut` references that live across calls (use raw-pointer field access) | A binds `let parser_mut = &mut *parser;` in every function (4 sites) | creates aliasing `&mut` while helper functions also receive `*mut` to the same struct |
| RP4 | §11 / prompt rule 7: copy no licence header into `lib.rs`; the crate header is the `//!` doc comment naming the source commit | A copied the 21-line MIT block from `jsmn.h` | cosmetic, but two agents will differ; the crate already carries licence info in `UPSTREAM` |
| RP5 | §2: field `type` is named `type_0` (c2rust spelling, used by the adapter and rulebook examples) | A named it `type_` | the adapter (`tests/adapter/mod.rs`) reads `t.type_0`; a different name breaks the Judge build, not behaviour |

## Judge changes need their own commit

`tests/adapter`, `examples`, `tests/upstream.rs` and `make_inputs.py` changed in commit `222bd79`. From now on the
deny-rules in `assets/settings.json` protect `tests/upstream.rs` and `examples/`.
