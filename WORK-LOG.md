# Work log

**Append only.** One entry per finished unit of work; never edit an old entry, add a correction as a new one.
Every entry: `date`, `who` (model, or script name), `what was done`, `what it tells us`, `next`.
Numbers in an entry come from commands that were run (commit named). Other files: `SPEND.md` is token cost,
`docs/DECISIONS.md` is decisions and their reasons, `pilots/*/migration/RUN-NOTES.md` is per-Pilot detail.
Dates are the commit's local time (UTC+7 shown by git); entries 1-6 were backfilled on 2026-10-10 from git history.

---

## 1. E0: project skeleton
- **Date:** 2026-10-09 22:35
- **Who:** Opus 5.5 (plan review, E0); scripts `spend.py`
- **Done:** uv project, Makefile, README, script stubs, `spend.py` reading token use from Claude Code transcripts
  (`35b725a`, `e25f229`).
- **Tells us:** PLAN.md needed three changes before any code: spend from transcripts, path-scoped rollback,
  ignore upstream clones.
- **Next:** E1, check that c2rust is usable on the first Pilot (jsmn).

## 2. E1: c2rust on jsmn
- **Date:** 2026-10-09 22:36
- **Who:** Opus 5.5; tool `c2rust 0.22.1`; `experiments/e1_c2rust_jsmn.py`
- **Done:** c2rust built and ran on jsmn in all 4 macro configurations (`65d8fb2`).
- **Tells us:** c2rust output compiles and passes 16/16 upstream tests in all 4 configs, so it is a usable
  starting point for the Faithful port (decisions 23, 24).
- **Next:** E2, merge the 4 outputs into one crate and build the Judge.

## 3. E2: merged crate, queue, gate, harness, first Judge
- **Date:** 2026-10-09 22:55 to 23:04
- **Who:** Sonnet 5.5 (executor); scripts `workqueue.py`, `gate.py`, `harness.py`, `inventory.py`, `depmap.py`,
  `tier.py`, `tm.py`, `jsmn/migration/harness/run.py`
- **Done:** one jsmn crate with `strict` and `parent-links` features (`f8e1244`); queue, baseline-aware gate and
  differential harness with tests (`880069e`, `d9dd3de`, `e6ba107`); analysis scripts and decisions 26-31
  (`f44cb8e`).
- **Tells us:** the Judge first passed a mutant (`~` rejected) on 164 hand-picked inputs and all 16 upstream
  tests; only an exhaustive every-byte-in-every-position corpus (932 inputs) caught it
  (`experiments/e2_jsmn_judge_sensitivity.py`, decision 28). A Judge must be shown to fail before it is trusted.
- **Next:** E3, write rulebook, prompts and Skill.

## 4. E3-plan: rulebook, prompts, Skill, script review
- **Date:** 2026-10-09 23:04 to 23:16
- **Who:** Opus 5.5
- **Done:** idiom table (all rows `verified: no`), rulebook template, jsmn `RULEBOOK.md` (Stage 2 API, deviations
  D1-D3, quirks Q1-Q8), `judge.md`, Stage 1/2 and review prompts, `SKILL.md`, deny-rules, script review with gaps
  G1-G19 (no code changed), decisions 32-37.
- **Tells us:** 7 of 19 script gaps block the jsmn Stage 2 run (G1-G4, G7 first); the rest can wait for E4.
- **Next:** user approves deviations D1-D3 (done 2026-10-09 23:56, `5f17954`), then Sonnet fixes the gaps.

## 5. E3-prep: gate fixes, Stage 2 Judge, stress test translation A
- **Date:** 2026-10-10 00:03 to 00:14
- **Who:** Sonnet 5.5 (scripts, Judge, orchestration); Haiku 4.5 subagent (translation A);
  scripts `run.py`, `gate.py`, `experiments/e3_jsmn_stress_prompt.py`
- **Done:** G7, G1, G2, G3, G4 fixed with tests (`3296ba5`..`7381680`); Stage 2 Judge: adapter, Rust dump
  driver, 16 translated tests (`222bd79`); translation A gated.
- **Tells us:** (a) Stage 2 Judge passes the Faithful port: 16/16 tests and 1188/1188 harness inputs in all 4
  feature sets. (b) The 932-input corpus was **blind to the E2 mutant in both `strict` sets**: in strict mode
  the byte is only reached as a primitive continuation (`[1~]`); adding that position (+256 inputs) made all 4
  sets go red on the mutant. (c) Translation A passes the Judge in all 4 sets.
- **Next:** second translation for the diff. Haiku/subagent spawn was refused twice by the auto-mode classifier.

## 6. E3 stress test: translation B and diff A vs B
- **Date:** 2026-10-10 00:38 to 00:50
- **Who:** Opus session applied RP1-RP5 to the RULEBOOK (`b90c698`); Sonnet 5.5 subagent translated B, gated it
  and wrote the diff (`8cc8ef7`); Sonnet 5.5 (this session) re-ran the 4 gates and fixed the heading (`b42451d`).
- **Done:** B gated in all 4 sets (`FAILED: 0`, 1188/1188 each); 6 new disagreements RP6-RP11 recorded in
  `pilots/jsmn/migration/RUN-NOTES.md`.
- **Tells us:** RP6 (`c_int` vs `c_uint` fields), RP7 (`wrapping_*` or not), RP8 (`pos++` shape), RP11 (markers)
  are real rulebook gaps; RP9, RP10 are prompt-emphasis problems. The Judge sees none of them, so they are style
  and fidelity issues, not bugs. A (Haiku, no RP1-5) vs B (Sonnet, with RP1-5) changes model and rules together,
  so this diff cannot yet support "stress-tested".
- **Next:** Opus applies RP6-RP8, RP11 to the rulebook and RP9/RP10 to `stage1-prompt.md`; then translation C
  (Sonnet, same rules) is diffed against B. Agreement there earns the `stress-tested` status line.

## 7. E3 stress test: RP6-RP11 applied to the rulebook and prompt
- **Date:** 2026-10-10 (after entry 6)
- **Who:** Opus 5.5 subagent (`e210aad`, `67735be`)
- **Done:** RP6, RP7, RP8, RP11 applied to jsmn `RULEBOOK.md` §0/§2/§4/§11 with amendment rows; RP9, RP10 added
  as HARD RULE 8 in `stage1-prompt.md` and in idiom rows C1, C5. RP7 changed on the way: `wrapping_*` on
  unsigned `pos`/`toknext` arithmetic only, plain operators on signed `count`/`size`/`i`. Prompt for
  translation C generated. `pytest`: 36 passed, 1 skipped (re-run by Sonnet).
- **Tells us:** the Judge reaches `toknext - 1` with `toknext == 0`, so unsigned wrap is observable; signed
  overflow is only D1 (unreachable). The Faithful port `pilots/jsmn/rust/src/lib.rs` has no PORT STATUS trailer
  or markers, so it does not satisfy the new §4/§11.
- **Next:** translation C, same model and same rules as B, diffed against B.

## 8. E3 stress test: translation C and diff C vs B
- **Date:** 2026-10-10 (after entry 7)
- **Who:** Sonnet 5.5 subagent (translation, gate, diff; `6dd5f7b`); Sonnet 5.5 (this session) re-ran the 4 gates
- **Done:** C gated in all 4 feature sets: `FAILED: 0` and 1188/1188 harness inputs each (re-run, same numbers);
  0 TODO(port); trailer `stage=1 confidence=high todos=0 deviations=D3`. Diff C vs B: 2 new disagreements.
- **Tells us:** with the same model and the new rules, B and C differ in only two places. RP12: how a
  `#[cfg]`-dependent case label that shares its body is written (B: separate arms, tail written twice; C: one
  arm, one tail), which also leaves open which site carries the Q5 marker. RP13 (minor): whether
  `pub type jsmnerr = c_int` is emitted. Everything else followed RP6-RP11 or was cosmetic. n = 2 translations
  per rule set, so RP12 may be a real gap or noise.
- **Next:** Opus decides RP12 (§2 C4 or §4) and RP13 (§0) in the rulebook; one more Sonnet translation (D) with
  that rulebook; if D agrees with C, Opus may set the status line to `stress-tested (2026-10-10)`.

## 9. E3 stress test: RP12 and RP13 decided
- **Date:** 2026-10-10 (after entry 8)
- **Who:** Opus 5.5 subagent (`cf0181b`, `3493eda`); Sonnet 5.5 (this session) checked commits, `pytest` (36 passed,
  1 skipped) and that the status line was untouched
- **Done:** RP12: each group of C case labels inside an `#ifdef` becomes its own `#[cfg]` match arm, shared tail
  written once per arm, Q5 marker above the `#[cfg(not(feature = "strict"))]` line of two arms (count stays 2).
  RP13: `pub type jsmnerr = c_int` is emitted. Rationale per Opus: `jsmn.h` has `#ifdef JSMN_STRICT` around exactly
  those labels, and the Faithful port and c2rust emit the alias.
- **Tells us:** both were free choices the Judge cannot see (behaviour is identical); the rule only removes
  variation between runs.
- **Next:** translation D, same model and rules as C, diffed against C.

## 10. E3 stress test: translation D and diff D vs C
- **Date:** 2026-10-10 (after entry 9)
- **Who:** Sonnet 5.5 subagent (translation, gate, diff; `5b27d60`); Sonnet 5.5 (this session) re-ran the 4 gates
- **Done:** D gated in all 4 feature sets: `FAILED: 0` and 1188/1188 harness inputs each (re-run, same numbers);
  0 TODO(port); trailer `stage=1 confidence=high todos=0 deviations=D3`. D vs C: 0 new disagreements reported.
- **Tells us:** the rulebook (RP1-RP13) produced the same shape in two same-model runs with no new rule gap.
  Limits: n = 2 per rule set; C and D are still not byte-identical (85 differing lines by `diff`, which the
  subagent classified as cosmetic: field-reset order, local `let`s, comments; I did not re-read those lines);
  the Judge cannot see any of these differences as behaviour; only jsmn (one Part, 471 lines) was tested.
- **Next:** Opus reads the RUN-NOTES D-vs-C section and, if it agrees, sets the rulebook status line to
  `stress-tested (2026-10-10)`; then the Stage 2 queue (`parts-stage2.tsv`) and the first Stage 2 batch.

## 11. E3 Stage 1 stress test closed; Stage 2 queue created
- **Date:** 2026-10-10 (after entry 10)
- **Who:** Opus 5.5 subagent (`65e9705`); Sonnet 5.5 (this session) checked commit, status line, `pytest`, and
  created `migration/parts-stage2.tsv` with `workqueue.py status`
- **Done:** Opus read the C-vs-D diff itself: 16 hunks, 85 lines; 10 cosmetic, 6 covered by RP12/RP13, 0 semantic.
  RULEBOOK status line now `stress-tested (2026-10-10)`; RUN-NOTES has "Stress test closed". Stage 2 queue: one
  Part `jsmn`, `todo`, attempts 0.
- **Tells us:** the Stage 1 rules are stable for jsmn. Limits stay: n = 2 per rule set, one Part, the Judge is
  blind to style (RP8-RP12 are enforced only by review), and A vs B also differed in model.
- **Open:** the working tree has uncommitted formatting edits to PLAN.md, README.md, SPEND.md and jsmn
  RULEBOOK.md that no agent of this session made (Opus noted a split list item under RULEBOOK §0).
  Stage 2 fan-out wants the deny-rules loaded by `claude --settings skill/c-to-rust/assets/settings.json`.
- **Next:** user decides on the two open items; then Stage 2 on jsmn (Sonnet, `stage2-prompt.md`, gate
  `make pilot-jsmn STAGE=2 MIN_TESTS=16`).

## 12. E3 Stage 2 on jsmn: attempt 1 blocked before the implementer started
- **Date:** 2026-10-10 (after entry 11)
- **Who:** Sonnet 5.5 governor session (started with `--settings skill/c-to-rust/assets/settings.json`)
- **Done:** deny-rule check passed (an `Edit` of `RULEBOOK.md` was refused). `make spend-now` timestamp
  `2026-10-10T04:27:15+00:00`. `verify` ok for `parts.tsv` (1 done) and `parts-stage2.tsv` (1 todo); SKILL.md branch 4
  holds. Marked `jsmn` `in_progress` (attempts 1) and ran `make pilot-jsmn-snapshot STAGE=2`: baseline HEAD `effbebb`,
  6 pre-existing failures (`fmt` in all 4 sets, `unsafe[]`, `allow[]`), which are what Stage 2 must fix. Filled
  `stage2-prompt.md` (66 KB, in the session scratchpad). Spawning the Sonnet implementer subagent was **refused by
  the auto-mode classifier** ("Blocked by classifier"). No workaround was tried; no code was changed.
- **Tells us:** the same classifier refusal as in entry 5 (Haiku spawn) now also hits a Sonnet implementer, so the
  subagent step of this Skill cannot run unattended in this environment without a permission change. No tokens were
  spent on an implementer, so no `SPEND.md` row.
- **Open:** `parts-stage2.tsv` is `in_progress` with attempts 1 although no attempt ran (uncommitted).
- **Next:** user decides how to unblock the subagent spawn (allow the Agent tool for this session, or run the
  implementer step differently); then re-send `stage2-prompt.md` and run the gate.

## 13. E3 Stage 2 on jsmn: attempt 1 accepted
- **Date:** 2026-10-10 (after entry 12)
- **Who:** Sonnet 5.5 subagent (implementer); Sonnet 5.5 subagent (reviewer); Sonnet 5.5 governor session
- **Done:** the subagent spawn that entry 12 reported as blocked worked after decision 38 (`autoMode.allow`). Attempt 1
  rewrote `src/lib.rs` and the adapter; gate `make pilot-jsmn STAGE=2 MIN_TESTS=16`: PASS, 23 checks (all four feature
  sets: check, test, harness, clippy, fmt; plus unsafe, unwrap, allow). 0 `unsafe`, 0 `TODO(port)`. Review verdict
  `accept`, no BLOCK; committed as `35e87c3`, Part marked `done`, attempts 1. The attempts counter in entry 12's
  "Open" line was right (1) and is now true.
- **Tells us:** Stage 2 passes the Judge first try, but the reviewer found 5 Judge gaps (Q2 stale `kind`, NUL in `\u`,
  two parent-links/`,` paths, `num_tokens` smaller than `toknext`) and 5 rule gaps (RUN-NOTES, G-S2-1..5, RP14-RP18),
  not reproduced by me. Most important: RP14, a possible undocumented behaviour difference from C when `num_tokens`
  is smaller than `toknext` on a reused parser; the rulebook D2 text does not cover it.
- **Next:** user or Opus decides RP14 (new Deviation vs API change) and applies RP15-RP18; Judge gaps G-S2-1..5 go in
  as their own commit and Stage 2 is re-gated against the stronger Judge.

## 14. E3 Stage 2 on jsmn: reviewer findings reproduced, RP14 decided (Opus), patches pending
- **Date:** 2026-10-10 (after entry 13)
- **Who:** Sonnet 5.5 subagent (reproduction); Opus 5.5 subagent (decision, patches, Judge spec); Sonnet 5.5 governor
- **Done:** C and Rust run side by side in all 4 feature sets (scratch only). G1-G4: Rust == C (coverage gaps, not
  divergences). RP14 confirmed: `[` (8) then `[]` (0): C 1, Rust -2; `[[` (8) then `[[]]` (1): C 2, Rust -2.
  Opus: C is defined here (`num_tokens` only limits allocation), so no new Deviation; the §6 API gains
  `parse_with_limit(js, tokens, num_tokens)` and a quirk Q9; D2 reworded (needs user confirmation). RP15-RP18 written
  as patches because the deny-rules refused the edits; Judge spec (13 inputs, phases 5-6) written. RUN-NOTES has the
  Opus entry.
- **Tells us:** commit `35e87c3` does not match C in one defined case and the Judge could not see it. Stage 2 must be
  redone after the rulebook and Judge change.
- **Open:** nothing outside RUN-NOTES is applied. The rulebook, idiom table and Judge are deny-protected in this
  session and Judge changes need their own commit outside the fan-out (SKILL.md rules).
- **Next:** user confirms the D2 amendment; patches applied and Judge extended in a session without the deny-rules;
  then attempt 2.

## 15. E3 Stage 2 on jsmn: Opus patches applied, Judge extended for attempt 2
- **Date:** 2026-10-10 (after entry 14)
- **Who:** Sonnet 5.5, governor session without deny-rules (`e2aa765`, `63a6486`, `858de4c`); the patches and the
  Judge spec were written by Opus in the deny-ruled Skill session
- **Done:** `git apply` of the Opus RULEBOOK and idiom-table diffs, committed. Judge extension per
  `opus-judge-spec.md`: +13 corpus inputs (1188 → 1201), `dump.c` / `dump.rs` phases 5 "dirty" and 6 "shrink",
  adapter `poison_kinds`. Validated on scratch crates built from git history.
- **Tells us:** the Faithful port passes 1201/1201 in all 4 sets under both the Stage 1 and the Stage 2 Judge.
  `35e87c3` (attempt 1) fails 36 / 160 / 38 / 162 inputs (default / strict / parent-links / strict+parent-links),
  all first differing in phase `shrink`: RP14 is now caught mechanically, where attempt 1 had passed the old
  Judge 23/23. The E2 mutant still fails (2 / 1 / 2 / 1 inputs). Spec detail wrong: `[[]]` is not yet a corpus case.
- **Next:** Stage 2 attempt 2 in the deny-ruled session: implement `parse_with_limit` per RULEBOOK §6, switch the
  adapter's `Some(k)` path to it, gate with `make pilot-jsmn STAGE=2 MIN_TESTS=16` (must now pass 1201/1201).

## 16. E3 Stage 2 on jsmn: `[[]]` added, Judge revalidated at 1202 inputs
- **Date:** 2026-10-10 (after entry 15)
- **Who:** Sonnet 5.5, governor session (`a60f56a`, `15d1760`)
- **Done:** `[[]]` added to the corpus (spec said it was there; it was not). RULEBOOK §10 count and a §12 row
  updated 1188 → 1202. All 16 validation runs repeated.
- **Tells us:** Faithful port 1202/1202 in all 4 sets under both Judges; `35e87c3` fails 37 / 161 / 39 / 163
  inputs (all in phase `shrink`); the E2 mutant fails 2 / 1 / 2 / 1. Opus spec errors found so far: one
  (`[[]]`), so spec claims about corpus content should be checked by a script, not by reading.
- **Next:** Stage 2 attempt 2 in a fresh deny-ruled session (command in the chat; fresh, not `--resume`).

## 17. E3 Stage 2 on jsmn: attempt 2 passes the gate, held before review (Judge file touched)
- **Date:** 2026-10-10 (after entry 16)
- **Who:** Sonnet 5.5 governor (deny-ruled session); Sonnet 5.5 subagent (implementer)
- **Done:** deny-rule check passed (Edit of `RULEBOOK.md` refused). `make spend-now` 05:23:37. Marked `jsmn`
  in_progress (attempts 2), snapshot STAGE=2. Implementer added `Parser::parse_with_limit`, threaded `num_tokens`,
  adapter `Some(k)` calls `parse_with_limit(js, &mut self.buf, k)`. Gate `make pilot-jsmn STAGE=2 MIN_TESTS=16`: PASS
  23/23, harness 1202/1202 in all 4 sets, 0 `unsafe`, 0 `unwrap`/`expect`, 0 `TODO(port)`.
- **Tells us:** (1) the gate baselines failures: on `35e87c3` it printed PASS (15 checks) while the harness failed
  1165/1202 (default), because the snapshot recorded those failures as pre-existing. A gate PASS proves nothing
  about the harness unless the harness rows are `pass`; here they are. Proposed fix: never baseline `harness` in
  `gate.py` (protected, not edited). (2) The implementer's `cargo fmt` reformatted two `writeln!` in
  `examples/dump.rs` (Judge file; committed version is not rustfmt-clean, which is why `fmt` failed in attempt 1).
  The implementer's revert was denied. The `fmt` PASS above is therefore only true with that unauthorised edit.
- **Open:** `dump.rs` is modified in the working tree and must not go into the Part commit. No review, no commit, Part
  still `in_progress`.
- **Next:** user decides: revert `dump.rs` (then `fmt` is red again) or commit the formatting as its own Judge
  commit; then review (`review-prompt.md`) and commit lib.rs + adapter.

## 17. E3 Stage 2 attempt 2: gate could pass a failing Judge (fixed)
- **Date:** 2026-10-10 (after entry 16)
- **Who:** Sonnet 5.5 governor session; the finding came from the deny-ruled Stage 2 session
- **Done:** `gate.py`: at Stage 2 the `test` and `harness` checks are never excused by the baseline
  (`Result.regresses`); Stage 1 keeps the baseline because a half-translated crate may not compile. New test
  `test_stage2_judge_checks_are_never_baselined` (37 passed, 1 skipped). `examples/dump.rs` rustfmt'd (Faithful
  port still 1202/1202 through it).
- **Tells us:** a baseline taken on a tree where the Judge already fails turns the gate into a rubber stamp.
  Review gap G-new: `make pilot-jsmn-snapshot` should be taken before the Judge extension or ignore Judge checks;
  now enforced by the script instead of by procedure. Attempt 2's gate PASS must show 1202/1202 in the harness.
- **Next:** the Stage 2 session re-gates attempt 2 with the fixed gate (script is read at run time).

## 18. E3 Stage 2 on jsmn: attempt 2 reviewed, committed, done
- **Date:** 2026-10-10 (after entry 17)
- **Who:** Sonnet 5.5 subagent (reviewer); Sonnet 5.5 governor
- **Done:** `gate.py` fix and `dump.rs` rustfmt were committed by the user session (`5b07c46`); gate re-run: PASS 23/23,
  harness 1202/1202 in all 4 sets. Review: accept, 0 BLOCK, 0 RULE, 1 NIT, 2 GAP. Committed `3dcdbda` (lib.rs +
  adapter); `jsmn` marked `done` (attempts 2); `verify` ok. 0 `unsafe`, 0 `unwrap`/`expect`, 0 `TODO(port)`.
- **Tells us:** RP14 is fixed and the Judge now sees it. Reviewer GAPs (unverified, by code reading only): phase 6 tests
  one `k` (`toknext/2`); `num_tokens > tokens.len()` (the D2 `NoMem` amendment) is unreachable by the Judge because the
  adapter asserts `n <= CAPACITY`.
- **Open:** user confirmation of the D2 amendment (RULEBOOK §8 still says "awaits user confirmation"); optional Judge
  additions (phase 6 for k in {0, toknext-1, toknext}).
- **Next:** user confirms D2; then E4 (not started).

## 19. E3 gap closure before E4: G5, G8, G9, G11, G12, G14, G15, G16
- **Date:** 2026-10-10 (after entry 18)
- **Who:** Sonnet 5.5 governor session did G5, G8, G9, G11 inline (two subagent spawns were refused by the
  auto-mode classifier, decision 38 fallback); Sonnet 5.5 subagents did G12 (`754b1c0`) and G14/G15/G16 (`f6e52e0`).
- **Done:** gate pass record `migration/gate-passed.tsv` checked by `workqueue.py verify`; harness raw-bytes compare,
  timeout = DIFF, `--repeat` / `UNSTABLE`; `verify` checks stubbed `todo!()` + RUN-NOTES and that the commit
  touched the target; one-Part crate -> `lib.rs`; ten new inventory tags; `# cmd:` header. Real Stage 2 gate on
  jsmn re-run: all 4 feature sets, clippy, fmt, unsafe, unwrap, allow pass and are recorded (`ce9c06e`).
  Full default suite: 74 passed, 1 skipped.
- **Tells us:** the review's "blocks E4" gaps are closed; G6, G10, G13, G17, G18, G19 remain (G18, per-model input
  tokens, matters for the extrapolation). One unrequested fix by a subagent: clang ran without the compile DB's
  `directory`, so relative `file` entries silently dropped every symbol (G14 test found it).
- **Next:** G18 in `spend.py`, then the user's go/no-go for E4 (open questions Q1-Q7 in chat).

## 20. Idiom table verified by tests (70 of 72 rows)
- **Date:** 2026-10-10 (after entry 19)
- **Who:** three Sonnet 5.5 subagents wrote `tests/idiom/` (two were cut off by a rate limit and re-run); Sonnet 5.5
  governor ran the suite and wrote the `verified` column with `experiments/e3_idiom_verified_column.py` (`74f1d4e`)
- **Done:** 80 tests compile a C snippet and the Rust mapping of a row and compare stdout; where a row has a trap the
  naive mapping is shown to differ. `make idiom` (about 4 minutes, excluded from default pytest). A6 and M4 are design
  rules with no observable behaviour. New inventory tags now select T11, T12, I7, I8, L4-L6, L10 instead of `*`.
- **Tells us:** 0 rows disproved. Caveat: the same agents wrote tests and judged rows against a table they could read,
  so this checks that the rows are internally consistent with real C and rustc output, not that the table is
  complete; a missing row is invisible to it. Rows with `*` tags are still sent to every prompt (the jsmn stress
  prompt was 37 KB).
- **Next:** first real use is E4 (cJSON); expect `fnptr`, `float-format`, `strtod` rows to be exercised there.

## 21. E3 close-out: user answers applied (decisions 39-43)
- **Date:** 2026-10-10 (after entry 20)
- **Who:** Sonnet 5.5 governor session; scripts `spend.py`, `experiments/e3_md_reflow.py`,
  `experiments/e3_idiom_verified_column.py`
- **Done:** G18 (`SPEND-MODELS.tsv`, per-model input/output/cache); glossary: Mapping (Stage 0), Stub, Governor;
  "referee" and "orchestrator" removed from the Skill docs; the Skill installed by name
  (`.claude/skills/c-to-rust`); Markdown reflowed to 120 columns, word-for-word checked; PLAN order amended
  (cJSON → tinyxml2); decisions 39-43 record the E4 go, its three allowed interventions, the 700k-token / $15 stop and
  the pilot order.
- **Tells us:** two of my own tools bit me, both caught before commit or repaired after: the reflow script turned a
  lone `-` table cell into a separator row, and treated rows without a closing pipe as shorter (fixed, rerun
  from the pre-reflow file); and a `make spend` with a guessed `SINCE` overlapped the previous row (removed from
  `SPEND.md` and `SPEND-MODELS.tsv`). Use the last row's time, never a guess. `SPEND.md` also has two identical
  `E3-stage2 | 05:30` rows written by the Stage 2 worker session: a double count of 25,297 output tokens, left as is.
- **Next:** a fresh deny-ruled session starts E4 on cJSON from `/c-to-rust` (Stage 0 Mapping first, then the
  Deviations table for the user).

## 22. E4 Pilot prepared: cJSON cloned and pinned
- **Date:** 2026-10-10 (after entry 21)
- **Who:** Sonnet 5.5 governor session (`git clone`, not the Skill: Pilot creation is outside the Skill)
- **Done:** `pilots/cjson/src` cloned (git-ignored, untrusted), `pilots/cjson/UPSTREAM` pins commit
  `6d9f2443ab071f86e5d9b43025a40929ec41c46c` (MIT). Sizes: `cJSON.c` 3206 lines, `cJSON_Utils.c` 1485,
  `cJSON.h` 306: about 10x jsmn.
- **Tells us:** E4 cost will not scale from jsmn's one 471-line Part linearly; the two `.c` files give the first
  real multi-Part queue and the first dependency order.
- **Next:** a fresh deny-ruled session runs `/c-to-rust` on `pilots/cjson` (Stage 0 Mapping), stops at the
  Deviations table.

## 23. Ticket sets: this chat's polish tickets; the Skill writes the run's tickets itself
- **Date:** 2026-10-10 (after entry 22)
- **Who:** Sonnet 5.5 governor session; script `skill/c-to-rust/scripts/tickets.py`
- **Done:** seven polish tickets for the Skill-building chat in `.scratch/skill-polish/issues/` (01 ticket generator
  done; 02 script gaps G6/G13/G17/G19; 03 spend overlap guard; 04 resume dry run on jsmn; 05 WORK-LOG numbering;
  06 fold cJSON findings; 07 E6 preparation). `tickets.py init|status|mark` writes and tracks the seven-ticket chain of
  a run in `<ws>/migration/tickets/`; SKILL.md step 0 makes every session read it. 4 new tests; suite 80 passed, 1
  skipped.
- **Tells us:** the run's phases are now machine-readable state, so a fresh session resumes from `tickets.py status`
  plus `parts.tsv` instead of from prose. Not yet exercised on a real Pilot; ticket 04 does that on jsmn.
- **Next:** start the cJSON run from the Skill; its first action creates the cJSON tickets.

## 24. E4 cJSON run started: tickets, inventory, depmap, tiers; blocked by deny-rules at Judge/rulebook writing
- **Date:** 2026-10-10 (after entry 23)
- **Who:** Sonnet 5.5 governor session, `/c-to-rust pilots/cjson`
- **Done:** deny-rule proof: `Edit` of `pilots/cjson/migration/RULEBOOK.md` refused ("File is in a directory that
  is denied by your permission settings"). `tickets.py init` wrote 7 tickets; 01 `in_progress`. `inventory.py`
  (regex engine, 2100 symbols), `depmap.py` (852 edges, 2 cycles), `tier.py` (5 Parts, all `judgment`) ran.
  Spend row `E4-mapping`: 41,812 output tokens (budget gate 490k not reached).
- **Tells us:** (1) `tier.py` lists `fuzzing/afl`, `cjson_read_fuzzer`, `fuzz_main` as Parts: they are fuzz drivers,
  not library code, so `parts.tsv` needs them dropped or marked. (2) `depmap` reports a `cJSON`/`cJSON_Utils` cycle
  and `parts.tsv` makes each a dep of the other; `cJSON.c` does not include `cJSON_Utils.h`, so this looks like a
  name-based false edge (unverified). (3) `cycles.txt` also lists vendored `tests/unity` files: `inventory.py` walks
  `tests/`.
- **Skill bug:** the Skill's Stage 0 steps 5-6 write `migration/harness/**` and `RULEBOOK.md`, but the deny-ruled
  session the user asked for refuses `Edit`/`Write` to exactly those paths, so Stage 0 cannot finish inside it.
- **Next:** user decides how to write the Judge and rulebook (see the stop message of this session).

## 25. Skill bugs from the first cJSON run fixed (tickets 08, 09, 10)
- **Date:** 2026-10-10 (after entry 24)
- **Who:** Sonnet 5.5, Skill-editing session; scripts `guard.py`, `inventory.py`, `depmap.py`, `tier.py`
- **Done:** (08) `guard.py` PreToolUse hook: Rulebook, `parts*.tsv`, Judge files, `Cargo.toml` are read-only only while
  the Pilot's ticket 04 or 06 is `in_progress`; `assets/settings.json` keeps the always-protected paths; SKILL.md step
  3.1, README and decision 44 updated; 6 tests. (09) `inventory.py`/`depmap.py` skip vendored directories and take
  `--exclude GLOB`; fuzz dirs are not Parts. (10) The cJSON ↔ cJSON_Utils cycle was a real script bug, not a real
  edge: `static enum E f(...)` was classified as the definition of `E`, which desynchronised brace tracking in
  `cJSON_Utils.c`, produced symbols named `if` and `return`, and every function "called" `if`. Fixed and tested.
  `pilots/cjson/migration/` regenerated with the fixed scripts: 2 Parts (`cJSON`, `cJSON_Utils`), 0 cycles.
- **Tells us:** a name-based call graph is only as good as the declaration scanner under it; the cJSON run found in
  one afternoon what jsmn's 471 lines could not. The guard is unit-tested but its hook has not yet fired in a real
  session (ticket 08 stays open on that point until the Translation session proves it). `spend.py` sums every
  session of the project directory, so a Skill-editing session and a Translation session that run at the same time
  overlap in the same window: rows are attributable to a phase only when one session runs at a time.
- **Next:** restart the Translation session with the new proof step; it resumes at ticket 01 from the regenerated
  Mapping files.

## 26. E4 cJSON: RULEBOOK drafted (ticket 01), Deviations table awaiting the user
- **Date:** 2026-10-10 (after entry 25)
- **Who:** Sonnet 5.5 governor; Opus subagent drafted `pilots/cjson/migration/RULEBOOK.md`
- **Done:** RULEBOOK status `draft`: §3 has 18 rows, checked by script against the 18 distinct `config-macro` tags
  of `inventory.tsv` (none missing); §8 has D1-D20, all `proposed`; §9 has Q1-Q24. No prose line over 120 columns.
  Spend row `E4-rulebook`: 10,017 output tokens in the window (E4 total 51,829; budget gate 490k not reached).
- **Open for Opus (marked TODO in the file):** Stage 1 engine (c2rust or LLM); 12 of 21 upstream tests use static
  internals through `#include "../cJSON.c"`, so Stage 1 must export them under a cfg or those tests run only
  against the C original; Stage 2 API (cycles, references, hooks). Q15 and Q20 are traced by reading, not run.
- **Skill bug:** the spend window counts 499 Opus output tokens while the subagent reported 192,943 tokens used in
  total (mostly input/cache); the subagent's sidechain output may not be in `spend.py`'s per-session sum
  (unverified). Check before the E4 extrapolation.
- **Next:** user approves or vetoes D1-D20; then ticket 01 closes and the stress test (ticket 03) follows.

## 27. E4 cJSON: Q15/Q20 proved by running, engine decision, Judge cases for D3/D6/D15/D19
- **Date:** 2026-10-10 (after entry 26)
- **Who:** Sonnet 5.5 governor, user answers in chat
- **Done:** (1) Q15 and Q20 compiled and run against the pinned `cJSON.c`/`cJSON_Utils.c` (`cc`, driver in the session
  scratchpad). Q15: `["\\", "x y"]` minifies to `["\\", "xy"]`, expected `["\\","x y"]`. Q20, calling the static
  `decode_pointer_inplace` directly: `a~1` -> `a~`, `a~1b` -> `a~/`, `a~0b` -> `a~0`, `a~0` -> `a~` (right);
  `replace` patches with `/a~1b`, `/a~0b`, `/x~1` return status 13, `/a` and `/x~0` return 0;
  `cJSONUtils_GetPointer` finds all four keys (it does not use the decoder). The traced-by-reading claim "wrong unless
  the escape is last" was itself wrong: `~1` is wrong even when last. RULEBOOK Q15 and Q20 rewritten from the run.
  (2) User decisions: Stage 1 engine is c2rust with per-feature runs and LLM polish (RULEBOOK §0); internals for
  the 12 static-using tests exported as `pub` under the cargo feature `test-internals` (§3, §10); D3, D6, D15, D19
  get Judge cases (§8 "Reachable" column). All 20 Deviation rows are still `proposed`.
  Spend row `E4-rulebook-fixes`: 36,988 output tokens; E4 total 88,817 (budget gate 490k not reached). The window
  overlaps the skill-editing chat, so it is an upper bound for this session.
- **Skill bugs:** (a) `assets/settings.json` hook used a relative path, so one `cd` into a Pilot directory broke
  every Bash/Write/Edit call (fixed by the user: `$CLAUDE_PROJECT_DIR`); the governor's own `cd` into
  `pilots/cjson/src` caused it. (b) `Edit(experiments/**)` is denied in `assets/settings.json:23`, so the proof
  script could not be saved to `experiments/` as the user's rules ask; it stays in the scratchpad until the line is
  dropped or another directory is named (user decision pending). (c) c2rust has not been run on cJSON yet.
- **Next:** user approves or vetoes D1-D20; then ticket 01 closes, `tickets.py mark 04 in_progress` and the guard proof.

## 28. E4 cJSON: Deviations D1-D20 approved, ticket 01 done
- **Date:** 2026-10-10 (after entry 27)
- **Who:** Sonnet 5.5 governor; the user approved all 20 rows in chat
- **Done:** RULEBOOK §8 rows D1-D20 `approved`, status line `deviations-approved (2026-10-10)`;
  `tickets.py mark 01 done` (1/7); proof scripts saved to `pilots/cjson/experiments/` (`cjson_q15_q20.py`,
  `q20_direct.c`) on the user's instruction to keep experiments in the Pilot workspace, not the Skill's folder.
  No subagent ran, so no spend row.
- **Tells us:** ticket 02 (Stage 1 Judge) must link the upstream tests "to the Rust crate", but the crate only exists
  after ticket 04, which 02 blocks. With the c2rust engine the resolution is a c2rust run per feature set first
  (a raw transpile, not an accepted Part); the Skill does not say so. Skill gap for the skill-editing chat.
- **Next:** ticket 02: one c2rust run per feature set into a scratch crate, then the Judge (driver, corpus,
  planted mutant), then 03 (stress test), then `mark 04 in_progress` and the guard proof.
