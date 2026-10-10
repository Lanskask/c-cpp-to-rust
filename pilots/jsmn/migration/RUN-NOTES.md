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
| RP12 (applied §2 C4, §4, commit cf0181b: separate cfg'd arms, B's shape) | §2 C4 (strict/non-strict case labels): say how a `#[cfg]`-dependent case label that shares its body is written in Stage 1: either one arm with the shared body written once (guard or `if` for the strict-only check, strict default folded into the same `_` arm) or separate cfg'd arms with the body duplicated. Same for the primitive loop's `:` label (cfg'd arm vs one or-pattern with a guard). Pick one | B: separate `#[cfg(strict)]` arm for `-`/digits/`t`/`f`/`n` plus `#[cfg(not(strict))] _` arm, the primitive tail (call, `r < 0`, `count`, `size++`) written twice, and a third `#[cfg(strict)] _ => return INVAL` arm; `:` is its own cfg'd arm in the primitive loop. C: one `_` arm holding one copy of the tail, with `matches!` returning `INVAL` for the strict default and the strict key check in a `#[cfg(strict)]` block; `:` in one or-pattern with a `cfg!` guard | idiom C4 allows duplication, prompt rule 8 asks for one copy only for goto labels; both pass the Judge, but the two shapes put the Q5 marker ("non-strict `default` arm") on different sites (B: the non-strict arm only; C: the one arm that also serves strict) |
| RP13 (applied §0, commit cf0181b: alias emitted) | §0 Stage 1 items: state whether `pub type jsmnerr = c_int` (the C `enum jsmnerr` tag) is emitted, or only the three `JSMN_ERROR_*` constants | B: three constants only. C: constants plus `pub type jsmnerr = c_int` | extra public item, harmless to the adapter; two agents differ, nothing observable (low priority, could also be left to cosmetic) |

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

Review (Opus, 2026-10-10): RP12 and RP13 checked against `jsmn.h`, the Faithful port, B and C; both applied
(commit cf0181b). RP12: `jsmn.h` has `#ifdef JSMN_STRICT` around exactly these case labels (lines 145-148,
397-424, 435-439), so prompt HARD RULE 2 decides it: separate `#[cfg]` arms in C order, which is B's and the
Faithful port's shape. Each feature set compiles one copy of the shared tail, as C does; C's merged `_` arm
with `matches!` and a `cfg!` guard makes the label set a runtime test. HARD RULE 8 covers goto labels only.
§4 Q5 now names the two `#[cfg(not(feature = "strict"))]` arms, marker line above the `#[cfg]` line (count
stays 2). RP13: `jsmn.h` declares the named `enum jsmnerr` (not a typedef); the Faithful port and c2rust emit
`pub type jsmnerr = c_int`; nothing in `tests/adapter`, `examples/dump.rs` or the harness uses the name. §0
now lists the alias, so C's choice is the rule and B's omission is a (harmless) miss. Next: translation D
(`stress/d/SESSION.md`, Sonnet, RP1-RP13), diffed against C.

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

## Stress test diff B vs C

C: Sonnet 5.5, prompt rebuilt with RULEBOOK carrying RP1-RP11 (prompt HARD RULE 8 added). B: same model, RP1-RP5
only. C `rust/src/lib.rs`: `cargo check` clean in all four feature sets, 0 TODOs, trailer
`stage=1 confidence=high todos=0 deviations=D3`.

Gate (`python3 -I harness/run.py --crate stress/c/rust <set>`), exact lines:

| set | upstream tests | harness |
|---|---|---|
| `""` | `original tests vs Rust [default]: FAILED: 0` | `harness: 1188/1188 inputs match` |
| `strict` | `original tests vs Rust [strict]: FAILED: 0` | `harness: 1188/1188 inputs match` |
| `parent-links` | `original tests vs Rust [parent-links]: FAILED: 0` | `harness: 1188/1188 inputs match` |
| `strict,parent-links` | `original tests vs Rust [strict,parent-links]: FAILED: 0` | `harness: 1188/1188 inputs match` |

| # | decision | B | C | class |
|---|---|---|---|---|
| 1 | `jsmntype_t`, `jsmntok_t.type_0`, `jsmn_parser` field types, `jsmn_parse`/`jsmn_init` signatures, `len`/`size_t` | `c_uint` + consts, `type_0`, `c_uint`/`c_uint`/`c_int`, C-typed params | identical | agree |
| 2 | arithmetic on signed `int` values (`count`, `size`, `start + 1`) | `wrapping_add` | plain `+= 1`, `start + 1` | covered by RP7 (B predates; RP7 as applied says plain for `int`) |
| 3 | `toknext - 1` in parent-links `}`/`]` arm | plain `- 1` after a `< 1` guard | `wrapping_sub(1)` | covered by RP7 |
| 4 | `QUIRK(port)` / `DEVIATION(port)` markers | none | 4xQ1, 2xQ2, 3xQ3, 3xQ4, 2xQ5, Q6, Q7, 2xQ8, 2xD3 (one line above each site) | covered by RP11 |
| 5 | PORT STATUS `deviations=` | `-` | `D3` | covered by RP11 |
| 6 | `pos++` placement, count-mode arm | one at loop tail, arm falls out | same | agree (RP8; B already matched) |
| 7 | primitive `found` exit | flag, one tail, strict `PART` guarded by `!found` | same | agree (RP9; B already matched) |
| 8 | `}`/`]` scans | two `while i >= 0` sharing `i`, `i == -1` | same | agree (RP10; B already matched) |
| 9 | index variable of the final `PART` scan | new `let mut k` inside the `if` | the function-level `i`, as in C | covered by prompt rule 8 / C5 (same index variable); B predates |
| 10 | strict-vs-non-strict case labels in `jsmn_parse` and `:` in the primitive loop | separate cfg'd arms, primitive tail written twice, third arm for strict default | one `_` arm, one tail, `matches!` + `cfg!` guard | new: RP12 |
| 11 | extra public `jsmnerr` alias | absent | `pub type jsmnerr = c_int` | new: RP13 (minor) |
| 12 | error-path state (Q2, Q3, Q4, Q6, Q8), `unsafe` use (raw deref only) | reproduced | reproduced | agree (both pass 1188/1188 x 4) |
| 13 | token field reset order in `jsmn_alloc_token` | `start` then `end` | `end` then `start` (C's `a = b = -1`) | cosmetic |
| 14 | `offset` vs `add`, `num_tokens_sz` local vs inline cast, `type_0`/`type_1` locals, `///` copies of C comments, `#![allow]` lines, `let c` before vs after the match | various | various | cosmetic |

RP6-RP11 followed by C: RP6 (`pos`, `toknext` are `c_uint`; §0) yes. RP7 (§0: `wrapping_*` on `pos`/`toknext`
expressions, plain on `count`/`size`/`start + 1`/`i`) yes, all sites. RP8 (§2 C4: single tail increment, no `continue`,
no per-arm increment, count-mode arms fall out) yes. RP9 (prompt rule 8 / C1: one flag, one tail) yes. RP10 (rule 8 /
C5: same `i`, two sequential scans, final loop resumes on `i`) yes. RP11 (§4 marker table, §11 trailer) yes: marker
counts match the table (Q1 4, Q2 2, Q3 3, Q4 3, Q5 2, Q6 1, Q7 1, Q8 2, D3 2).

New disagreements C vs B: 2 (RP12, RP13; RP13 is minor). Every other difference is covered by RP6-RP11 or cosmetic.
The differences B had on RP6-RP11 closed in C, so the rules did their job on a same-model run. One gap in the
rulebook: the §4 marker table cannot say where a marker goes when one shared arm serves both strict and non-strict.
Single run per translation; one more same-rulebook run would show whether RP12 is noise.

## Stress test diff C vs D

D: Sonnet 5.5, prompt rebuilt with RULEBOOK carrying RP1-RP13 (`stress/d/prompt.md`). C: same model, RP1-RP11. D
`rust/src/lib.rs`: `cargo check` clean in all four feature sets, 0 warnings, 0 TODOs, trailer
`stage=1 confidence=high todos=0 deviations=D3`. Marker counts: Q1 4, Q2 2, Q3 3, Q4 3, Q5 2, Q6 1, Q7 1, Q8 2, D3 2
(match the §4 table).

Gate (`python3 -I harness/run.py --crate stress/d/rust <set>`), exact lines:

| set | upstream tests | harness |
|---|---|---|
| `""` | `original tests vs Rust [default]: FAILED: 0` | `harness: 1188/1188 inputs match` |
| `strict` | `original tests vs Rust [strict]: FAILED: 0` | `harness: 1188/1188 inputs match` |
| `parent-links` | `original tests vs Rust [parent-links]: FAILED: 0` | `harness: 1188/1188 inputs match` |
| `strict,parent-links` | `original tests vs Rust [strict,parent-links]: FAILED: 0` | `harness: 1188/1188 inputs match` |

| # | decision | C | D | class |
|---|---|---|---|---|
| 1 | `jsmntype_t`, `jsmntok_t.type_0`, `jsmn_parser` field types, `jsmn_parse`/`jsmn_init` signatures, `size_t` alias | `c_uint` + consts, `type_0`, `c_uint`/`c_uint`/`c_int`, C-typed params | identical | agree |
| 2 | `jsmnerr` alias and type of `JSMN_ERROR_*` | alias emitted, constants typed `c_int` | alias emitted, constants typed `jsmnerr` | covered by RP13 (C predates the "typed `jsmnerr`" wording) |
| 3 | strict/non-strict case labels in `jsmn_parse` | one `_` arm, `matches!` for the strict default, one tail | `#[cfg(strict)]` arm for `-`/digits/`t`/`f`/`n`, `#[cfg(not(strict))] _` arm, `#[cfg(strict)] _ => return INVAL`; tail written once per cfg'd arm | covered by RP12 (C predates; D follows) |
| 4 | `:` in the primitive loop | or-pattern with `cfg!(not(strict))` guard | own `#[cfg(not(strict))] b':'` arm first, then the whitespace arm | covered by RP12 (D follows) |
| 5 | Q5 marker sites | above the one shared `_` arm (and the `:` or-pattern) | above the `#[cfg]` line of each non-strict arm, strict arms unmarked | covered by RP12 / §4 Q5 row (D follows) |
| 6 | `pos`/`toknext` arithmetic, `toknext - 1`, signed `int` values | `wrapping_*` on unsigned, plain on `int` | same | agree (RP6, RP7) |
| 7 | `pos++` at loop tail, count-mode arms fall out | yes | yes | agree (RP8) |
| 8 | primitive `found` flag, one tail, strict `PART` guarded by `!found` | yes | yes | agree (RP9) |
| 9 | `}`/`]` scans share `i`, `i == -1`, final loop resumes on `i` | yes | yes | agree (RP10) |
| 10 | error-path state (Q2, Q3, Q4, Q6, Q8), `unsafe` use (raw deref only), no `&mut` from raw pointer | reproduced | reproduced | agree (both pass 1188/1188 x 4) |
| 11 | token field reset order in `jsmn_alloc_token` | `end` then `start` | `end` then `start` (`start = end`) | cosmetic |
| 12 | `let c`/`let h` locals for the D3 byte checks vs repeated `*js.add(..) as u8`, `let mut type_0` vs `let type_0`, `let _ = found;`, `// found:` comment, `#![allow]` lines, doc header text | various | various | cosmetic |

RP12-RP13 followed by D: RP12 (§2 C4 "`#ifdef` around `case` labels": separate cfg'd arms in C order, tail once per
arm, no `matches!`/`cfg!` guard; `:` its own first arm in the primitive loop) yes. RP13 (§0: `pub type jsmnerr =
c_int` emitted, `JSMN_ERROR_*` typed `jsmnerr`) yes. §4 marker table Q5 row (marker line above the `#[cfg]` line of
the two non-strict arms, strict arms unmarked) yes. RP1-RP11 also still followed (rows 1, 6-10).

New disagreements D vs C: 0. Every difference is covered by RP12-RP13 (C predates them) or cosmetic, so no RP14 is
proposed. The lines the rulebook pins down (Stage 1 items, C4 `#ifdef` shape, marker table) produced the same
observable shape in D as in B/the Faithful port. Single run per translation.

## Stress test closed (Opus, 2026-10-10)

Checked the "D vs C: 0 new disagreements" claim by reading
`diff stress/c/rust/src/lib.rs stress/d/rust/src/lib.rs` (16 hunks, 85 `<`/`>` lines; line numbers C → D):

| hunk | change | class |
|---|---|---|
| 1c1 | `//!` header names the commit | cosmetic (comment) |
| 3,8c3,5 | doc text; `#![allow]` set (C: `unused_*`; D: `non_upper_case_globals, non_snake_case`) | cosmetic (lint attributes, no codegen) |
| 22,24c19,21 | `JSMN_ERROR_*: c_int` → `: jsmnerr` | covered by RP13 (`jsmnerr = c_int`, same type) |
| 58c55 | `start = -1` → `start = end` after `end = -1` (C `a = b = -1`) | cosmetic (assignment form, same values) |
| 89,90c86 | `let c` local dropped in the primitive loop | cosmetic (local binding) |
| 92,94c88,89 | `:` or-pattern with `cfg!` guard → own `#[cfg(not(strict))] b':'` arm first | covered by RP12 |
| 97a93,96 | whitespace/`,]}` arm split out | covered by RP12 |
| 101c100,102 | D3 check reads `*js.add(pos)` twice instead of `c` | cosmetic (local binding; no write between reads) |
| 113a115,116 | `// found:` comment, `let _ = found;` | cosmetic (comment, no-op binding) |
| 170c173 | redundant parentheses | cosmetic |
| 183d185 | `let h` local dropped (hex escape) | cosmetic (local binding) |
| 185c187,192 | hex range test on repeated reads instead of `h` | cosmetic (local binding) |
| 224c231 | `let mut type_0` → `let type_0` | cosmetic |
| 356,363c363,368 | merged `_` arm with `matches!` → `#[cfg(strict)]` arm for `-`/digits/`t`/`f`/`n` | covered by RP12 |
| 366,373d370 | strict key check moves into that arm | covered by RP12 |
| 383a381,394 | `#[cfg(not(strict))] _` arm (Q5 marker above `#[cfg]`) + `#[cfg(strict)] _ => INVAL` | covered by RP12 / §4 Q5 row |

Counts: 10 cosmetic, 6 covered by a rule (RP12: 5, RP13: 1), 0 semantic and uncovered. No public item, arithmetic
form, marker count or `unsafe` use differs outside RP12/RP13; marker counts in D match the §4 table. The claim
holds, so no RP14. RULEBOOK status set to `stress-tested (2026-10-10)` (§12 row "E3 stress test close").

Limits of this result:
- n=2 per rule set: each rulebook version (RP1-RP5, RP1-RP11, RP1-RP13) was tested by one pair of runs. "0 new
  disagreements" in D vs C is one sample; a third same-rulebook run could still find a choice the rules leave open.
- One Part: jsmn is a single `judgment` Part (`jsmn.h`), so SKILL.md step 2's "3 hardest Parts" is 1 here. Shapes
  that only show up in other code (unions, `goto` chains, function pointers) were not exercised.
- The Judge is blind to style: all four translations pass 1188/1188 × 4 sets, so every disagreement above was found
  by reading the diffs, not by the Judge. Rules that fix shape (RP8-RP12) are enforced only by review.
- Translations A and B differ in model (Haiku vs Sonnet) as well as rules; C and D are same-model (Sonnet 5.5).

## Stage 2 attempt 1: gate and review (Sonnet governor, 2026-10-10)

Implementer: Sonnet 5.5 subagent, attempt 1. Gate `make pilot-jsmn STAGE=2 MIN_TESTS=16`: PASS, 23 checks (check, test,
harness, clippy, fmt in each of the four feature sets, plus unsafe, unwrap, allow). `src/lib.rs` has 0 `unsafe`
tokens, 0 `TODO(port)`, trailer `stage=2 confidence=high todos=0 deviations=D1,D2,D3`.

Review (Sonnet subagent, `review-prompt.md`): verdict `accept`, no `BLOCK`. Findings below are the reviewer's;
I have not reproduced them by running the inputs, so treat them as unverified until a Judge run shows them.

### Judge gaps (Judge files are protected: proposals only, own commit outside the fan-out)

- **G-S2-1** Q2 (`alloc_token` does not reset `kind`) is invisible: every phase starts from a zero-filled buffer.
  Proposed dump phase on a dirty buffer: parse `["x","y"]`, `reset()`, then parse `{{` in a strict set; C token 1
  keeps `kind=4`. Also hides `Token::default` `start=None` against C `memset` `start=0`.
- **G-S2-2** `\u` hex loop never stops on NUL in the corpus. Inputs: `"\u12\x00"` (bytes `22 5c 75 31 32 00 22`) and
  `["\u1\x00]`; C returns `PART`.
- **G-S2-3** parent-links `}`/`]` arm with a closed root token and `toksuper != -1` (Q8, feature-dependent result).
  Inputs: `{}:}` and `[]:]` in all four sets.
- **G-S2-4** `,` with no open container and a non-container `toksuper`. Inputs: `"a":,` and `1:,` (default keeps
  `toksuper=0`, parent-links gives `-1`).
- **G-S2-5** `parse` with a smaller `num_tokens` than an earlier call on the same parser (see RP14).

### Rule proposals (Opus or user decides; rulebook is deny-protected here)

- **RP14 (needs a decision, may be a new Deviation)** The reviewer says Rust differs from C where C is defined: the
  Rust slice length is `num_tokens`, so `parse(b"[", Some(8))` then `parse(b"[]", Some(0))` without reset gives
  `Err(Inval)` at `]` where C returns 1 (C uses `num_tokens` only as an allocation guard). §8 D2 covers only a token
  array shorter than `toknext`/`toksuper`. Proposed D2 text: add "or `num_tokens` smaller than `toknext`/`toksuper`
  of a reused parser while the real array is larger (defined in C)", Rust: slice length is `num_tokens`, first access
  at an index `>= tokens.len()` returns `Err(Error::Inval)`. If the user does not accept this difference, the API in
  §6 needs a separate `num_tokens` argument instead.
- **RP15** §4 markers: Stage 2 code carries 10 `DEVIATION(port): D2` markers (each `get`/`get_mut(..).ok_or(Inval)`)
  that no table lists; D1 has no marker site (carried by `usize` field types). Add a §4 row for both.
- **RP16** Idiom C5 Stage 2 column: add `slice.iter().rposition(pred)` (and `slice[..i].iter().rposition(pred)` for a
  scan that resumes from `i`).
- **RP17** Idiom I1: new row for unsigned `x - 1` stored into a `-1` sentinel: `checked_sub(1)` giving
  `Option<usize>` (`:` arm at `toknext == 0`).
- **RP18** New idiom row for `arr[i]` with unchecked `i`: plain `arr[i]` with an invariant comment when `i` was just
  validated; `arr.get(i).ok_or(err)?` plus a Deviation when `i` derives from caller state.

NITs (not blocking): `parse_primitive` `self.pos -= 1` comment should name the caller contract (terminators are
handled in earlier arms); add "`toknext >= 1`, checked above" to the parent-links `}` arm; re-wrap the adapter `//!`
line to 120 columns; one-line doc comments on `pos`/`toknext`/`toksuper`/`code`.

### Opus review (2026-10-10)

Reproduction (C `jsmn.h` vs Stage 2 attempt 1, run side by side in all four feature sets, scratch `verify/`):

- G-S2-1..4 (stale `kind` on a reused buffer, `"\u12<NUL>"`, `{}:}`/`[]:]`, `"a":,`/`1:,`): Rust == C in every set.
  The reviewer's divergence claims are refuted; they stay as corpus-coverage gaps only.
- RP14 confirmed. On a reused parser C uses `num_tokens` only as the allocation limit and reads `tokens[toknext - 1]`,
  `tokens[toksuper]` and the backward scans from the caller's whole array. `[` (8) then `[]` (0): C 1, Rust −2.
  `[[` (8) then `[[]]` (1): C 2, Rust −2. `[[` then `[[]` (1): C −3, Rust −2. Control `[` then `[]` (1): match.

Is the C behaviour defined? Yes, whenever the array object really holds the tokens being read: `num_tokens` is a
plain integer, and `tokens[i]` with `i < toknext` is an in-bounds access when the caller's array is at least `toknext`
long (C11 6.5.6p8). It is UB only when the real array is shorter than `toknext`/`toksuper + 1` (that is D2) or when
`num_tokens` exceeds the real array and an allocation goes past it (also D2). A Rust slice carries its real length,
so Rust can tell the defined case from the UB case; mapping `num_tokens` to the slice length erased that difference.

Decision RP14: no new Deviation. The user's goal is C behaviour and C is defined here, so the API changes:
`Parser::parse_with_limit(&mut self, js: &[u8], tokens: &mut [Token], num_tokens: usize) -> Result<usize, Error>`
(the whole caller array plus C's limit); `parse(js, t)` = `parse_with_limit(js, t, t.len())`; `count(js)` unchanged
(C ignores `num_tokens` when `tokens == NULL`). `alloc_token` fails when `toknext >= num_tokens` (Q9, new kept quirk)
or `toknext >= tokens.len()` (D2); every other access is bounded by `tokens.len()` only. The adapter passes
`&mut self.buf` (all 512 slots) and `n`, never `&mut self.buf[..n]`. D2 is reworded: `num_tokens > tokens.len()` is
now representable and gives `NoMem` at the first allocation past the array; the user must confirm that amendment.
RP15-RP18 accepted (Stage 2 marker table: Q9 1, D2 11, D1 has no site; idiom C5 `rposition`, new I10
`checked_sub(1)`, new E6 array index, T5 trap).

RULEBOOK.md and `skill/**` refused the Edit (deny-rules), so the changes are patches in the session scratchpad:
`opus-RULEBOOK.md.patch.md` and `opus-idiom-table-c.md.patch.md` (raw `.diff`, checked with `git apply --check`
against `b9d1dff`; full text in `.new`). Judge additions (corpus +13 inputs, dump phases 5 `dirty` and 6 `shrink`,
adapter `poison_kinds`): `opus-judge-spec.md`. Phases 5 and 6 were run on C for `[]`, `[[]]` and `{"a":1,[`.

Consequence: Stage 2 attempt 1 (`35e87c3`) must be redone for `lib.rs` (new method, `num_tokens` threaded through
`run`/`alloc_token`/`parse_string`/`parse_primitive`, Q9 and D2 markers) and the adapter (`parse_with_limit` on the
whole buffer). Order: apply the patches, build and validate the extended Judge (it must fail on `35e87c3`), then run
Stage 2 attempt 2.

## Judge extension for Stage 2 attempt 2 (Sonnet, 2026-10-10, commit 63a6486)

Spec: Opus `opus-judge-spec.md` (corpus +13, `dump.c`/`dump.rs` phases 5 "dirty" and 6 "shrink",
adapter `poison_kinds`). Corpus 1188 → 1201. Numbers below are from commands run in this session, on scratch
crates built from git history (Faithful port = `35e87c3^` lib.rs and Stage 1 adapter plus `poison_kinds`).

| crate | Stage 1 Judge (C tests + harness) | Stage 2 Judge harness | notes |
|---|---|---|---|
| Faithful port (`35e87c3^`) | 1201/1201 in all 4 sets, `FAILED: 0` | 1201/1201 in all 4 sets | must pass: yes |
| Stage 2 attempt 1 (`35e87c3`) | not applicable (no C ABI) | default 1165/1201, strict 1041/1201, parent-links 1163/1201, strict+parent-links 1039/1201 | must fail: yes. Every failing input differs first in phase `shrink` (RP14); `[]` and `{}` both fail, e.g. `{}`: C `shrink pos=2 toknext=1 toksuper=-1`, Rust `pos=1 toknext=1 toksuper=0` |
| E2 mutant on Faithful port (`32..127` → `32..126`) | not re-run | default 1199/1201, strict 1200/1201, parent-links 1199/1201, strict+parent-links 1200/1201 | must fail: yes |

Deviations from the spec, for Opus:

- Spec §1 says `[[]]` is already in the corpus; it is not (no case has exactly that content). Phase 6 is
  still reached by `[[],[]]` and `[1,[2,[3]]]`; `[[]]` can be added to `EXTRA` (corpus becomes 1202).
- Spec §1 says G-S2-1..4 reproduce attempt 1 == C: confirmed, all four sets differ only in phase `shrink`, so
  the 10 new G-S2-1..4 inputs add coverage without a known failure.
- The Stage 2 adapter in the tree still uses `&mut self.buf[..n]`; only `poison_kinds` was added, as the spec
  says. Making `parse(js, Some(k))` call `parse_with_limit` is the attempt-2 implementer's job.
- Scratch crates for this validation are under the session scratchpad, not in the repo.
- `make pilot-jsmn STAGE=2` now fails on the committed attempt 1 by design (RP14).
