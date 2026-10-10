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
Applied to RULEBOOK §10 (2026-10-10, §12 amendments).

## Stress test (SKILL.md step 2): two translations done (A Haiku no RP1-5, B Sonnet with RP1-5); status not yet stress-tested

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
| RP6 (applied §0, commit e210aad) | §0: add the exact `jsmn_parser` fields to the Stage 1 items: `pos: c_uint, toknext: c_uint, toksuper: c_int` (as in C), like the `jsmntok_t` list | A: `pos: c_int, toknext: c_int`; B: `c_uint` (from `jsmn.h`). Layout is identical, signedness and every comparison/cast differ | the adapter and C tests read these fields; `c_int` makes `pos as usize` sign-extend and turns `+= 1` into signed overflow (see RP7) |
| RP7 (applied with change §0, commit e210aad) | §0 or prompt rule 4: in Stage 1 every `+`/`-` on `pos`, `toknext`, `count`, `size`, `start + 1`, `toknext - 1`, `pos + 1` uses `wrapping_*`; a plain operator is not allowed | A: plain `+= 1`, `-= 1`, `toknext - 1`, `pos + 1`, `start + 1` everywhere. B: `wrapping_*` at every site (including signed `count`, `size`) | the Judge runs debug builds, where plain overflow is a panic; rule 4 says "unsigned wraps", "signed follows §8", and §8 D1 is a Stage 2 statement, so signed sites are left to the agent |
| RP8 (applied §2 C4, commit e210aad) | §2 C4: resolve the conflict with "keep the C structure". State: the `for` increment `pos++` is written once at the loop tail, `{`/`[`/`}`/`]` arms with `tokens == NULL` fall out of the `match` (no `continue`, no per-arm `pos += 1`) | A: `pos += 1` copied into 6 arms plus `continue` (what C4 says literally). B: one increment at the tail, no `continue` | both behave the same here, but A's shape drops the C control flow and gets an arm wrong easily; the rulebook text currently asks for A's shape and the prompt asks for B's |
| RP9 (applied as prompt rule 8 + idiom C1, commit e210aad) | §2 C1: say "one `found` flag, one shared tail after the loop; the strict-mode `PART` return is guarded by `!found`". C1 already says Stage 1 keeps the flag, so this is a restatement | A: no flag, the `found` tail (count-mode, alloc, fill, parent, `pos--`) is written three times in `match` arms and at loop end | three copies can drift; B has one copy. The rule exists, A did not follow it |
| RP10 (applied as prompt rule 8 + idiom C5, commit e210aad) | idiom C5 Stage 1 (already "same with `c_int`"): `jsmn_parse` keeps `i: c_int` and two sequential `while i >= 0` scans in the `}`/`]` arm; no `for .. in (..).rev()`, no extra `found` bool or nested scan | A: `for i in (0..toknext).rev()` with a nested `for j in (0..i).rev()` and a `found` flag; B: C's two scans sharing `i`, `if i == -1` | the C code's `i == -1` test and "second scan starts at the same `i`" are the structure to keep; A's rewrite is equivalent here but is the Stage 2 form |
| RP11 (applied §4, §11, commit e210aad) | §4 and §11: Stage 1 annotations are fixed. Place `QUIRK(port): Q<n>` only at the sites in §9 "Where"; `// PORT STATUS: stage=1 confidence=<high\|medium\|low> todos=<N> deviations=<ids that apply to this file>` | A: Q-markers at three sites, one wrong (Q1 on the `c < 32` check), `confidence=draft`, `deviations=D1,D2,D3`. B: no markers at all, `confidence=high`, `deviations=-` | markers and the trailer are free text today; neither file is right by the rulebook (B omitted markers the prompt asked for; D3 applies to B's `u8` compares) |

Review (Opus, 2026-10-10): RP6-RP11 checked against `jsmn.h`, the Faithful port, A and B. None rejected.
RP7 changed before applying: the proposal asked for `wrapping_*` at every site, including the `int` values
`count`, `size`, `start + 1`. In C those overflow only past 2^31 (signed overflow is UB, D1), so RULEBOOK §0 now
says plain operators there (as the Faithful port and c2rust write them) and `wrapping_*` only on the
`unsigned int` `pos`/`toknext` expressions; `toknext - 1` at `toknext == 0` is reached by the Judge (`:` or a
closing bracket first) and must wrap. RP8: the rulebook now asks for the C shape (one loop-tail increment, no
`continue`), not the Faithful port's `pos += 1; continue`. RP9/RP10 are emphasis only: prompt HARD RULE 8 and
the Stage 1 column of idiom rows C1/C5. RP11: §4 lists every Stage 1 marker site and §11 fixes the trailer
values (Stage 1 `deviations=D3`). The Faithful port `rust/src/lib.rs` has no PORT STATUS trailer and no
markers; it was not edited (rust/ is out of scope). Next: translation C (`stress/c/SESSION.md`, Sonnet, same
rules as B plus RP6-RP11) to see whether a same-model run with the full rulebook still disagrees with B.

## Judge changes need their own commit

`tests/adapter`, `examples`, `tests/upstream.rs` and `make_inputs.py` changed in commit `222bd79`. From now on the
deny-rules in `assets/settings.json` protect `tests/upstream.rs` and `examples/`.

Review (Opus, 2026-10-10): RP1-RP5 checked against `rust/src/lib.rs` and `tests/adapter/mod.rs`; all accepted and
applied to RULEBOOK §0, §2 T8, §4, §11 (amendments in §12). Status stays short of `stress-tested`: translation B
and the A/B diff are still missing (SKILL.md step 2 "done when ... RUN-NOTES records the diffs").

## Stress test diff A vs B

B: produced by Sonnet 5.5 from the same prompt builder output but with RULEBOOK already carrying RP1-RP5 (the prompt
shows them in §0/§2/§4/§11). A: Haiku, produced without RP1-RP5. So the two differ in model and in rules; the table
separates what RP1-RP5 already fix from what is still open. B `rust/src/lib.rs`: 4 `cargo check` sets clean, 0 TODOs.

Gate (`run.py --crate stress/b/rust <set>`), all four sets, same lines shape:

| set | upstream tests | harness |
|---|---|---|
| `""` | FAILED: 0 | 1188/1188 inputs match |
| `strict` | FAILED: 0 | 1188/1188 inputs match |
| `parent-links` | FAILED: 0 | 1188/1188 inputs match |
| `strict,parent-links` | FAILED: 0 | 1188/1188 inputs match |

Both translations pass the Judge in all four sets. The Judge does not see any of the disagreements below as
behaviour; they are differences in type choices, shape and annotation.

| # | decision | A | B | class |
|---|---|---|---|---|
| 1 | `jsmntype_t` | `#[repr(i32)] enum` used as a `#[repr(C)]` field | `type jsmntype_t = c_uint` + `JSMN_*` consts | covered by RP1 |
| 2 | token field name | `type_` | `type_0` | covered by RP5 |
| 3 | `jsmn_parse` `js` param | `*const c_int` (cast to `*const u8` inside) | `*const c_char` | covered by RP2 |
| 4 | `jsmn_parse` `num_tokens` | `c_int` (cast to `usize`) | `c_uint` | covered by RP2 |
| 5 | `&mut *parser`, `&mut *tokens.offset(..)` references | in every function, live across calls to helpers | raw-pointer field access only | covered by RP3 |
| 6 | licence header | 21-line MIT block | `//!` doc comment | covered by RP4 |
| 7 | `jsmn_parser.pos` / `toknext` type | `c_int` | `c_uint` | new: RP6 |
| 8 | overflow policy | plain `+=`, `-=`, `+ 1`, `- 1` | `wrapping_*` at every site | new: RP7 |
| 9 | loop-increment placement, count-mode arm | `pos += 1` in each arm, `continue` | one `pos++` at loop tail, arm falls out | new: RP8 |
| 10 | primitive `found` exit | no flag, tail written 3 times | `found` flag, one tail | new: RP9 (existing C1 not followed by A) |
| 11 | `}`/`]` scan | `for .. .rev()` + nested scan + `found` bool | two `while i >= 0` scans sharing `i`, `i == -1` test | new: RP10 (existing C5 Stage 1 not followed by A) |
| 12 | markers / PORT STATUS | 3 QUIRK markers (one misplaced), `draft`, `D1,D2,D3` | no markers, `high`, `-` | new: RP11 |
| 13 | public signatures otherwise | `jsmn_init`, `jsmn_parse` names, `#[no_mangle] extern "C"`, `len: usize` | same | agree |
| 14 | error-path state (Q2, Q3, Q4, Q6, Q8) | reproduced | reproduced | agree (both pass 1188/1188 x 4) |
| 15 | `unsafe` use | everywhere raw deref, plus `&mut` (row 5) | raw deref only | covered by RP3 |
| 16 | helper `js` type, `size_t` alias, `std::os::raw` vs `core::ffi`, `offset` vs `add`, `#![allow]` lines, `derive(Debug)`, `b'0'` vs `48` | various | various | cosmetic |

New disagreements: 6 (RP6-RP11). Two of them (RP9, RP10) restate an existing rulebook row that A did not follow,
so they are prompt-emphasis issues rather than missing rules. RP6-RP8 and RP11 are gaps in the rulebook.
Not shown by this test: whether two agents with the same model and the same rulebook agree (one A-run with
RP1-RP5 would show it).
