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
