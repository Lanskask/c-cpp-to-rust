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
