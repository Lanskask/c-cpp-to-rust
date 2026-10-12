# tinyjson report (Stage 1 and Stage 2), 2026-10-12

Pinned upstream: tinyjson bcca77e (GPL-3.0), one header of 1,079 lines of C++17, one Part (`src/tinyjson` to
`rust/src/lib.rs`). C++ route: there is no C ABI in either Stage. Numbers below come from runs I executed or from the
ledgers named in each row.

## Result

| Item                        | Stage 1 (Faithful port)                  | Stage 2 (Idiomatic pass)                       |
|-----------------------------|------------------------------------------|------------------------------------------------|
| Parts done                  | 1 of 1 (`parts.tsv`, 5 attempts)         | 1 of 1 (`parts-stage2.tsv`, 1 attempt)         |
| Commit                      | 558bb65 (Judge re-gated at 4392c49)      | aa15689                                        |
| Gate                        | PASS, 3 of 3 checks                      | PASS, 8 of 8 checks                            |
| Translated upstream tests   | 22 of 22                                 | 22 of 22                                       |
| Differential harness        | 0 DIFF of 5755 inputs (g++-16, libstdc++)| 0 DIFF of 5755 inputs                          |
| `unsafe` in `rust/src`      | allowed (void* payload)                  | 0                                              |
| `TODO(port)` open           | 0                                        | 0                                              |
| Judge sensitivity (mutants) | 1 of 1 (D5 window 29 to 30: tests green, | same mutant, Stage 2 Judge via the section 6   |
|                             | corpus 13 DIFF)                          | shim: red                                      |

One feature set (none). Stage 2 crate: 1,396 lines, no dependencies.

## Rules

- Deviations: 16 rows (D1-D16), all approved (D3 and D5 by the user's "match the origin" rule).
- Kept quirks: 20 rows (Q1-Q20). Stress-test rules: P1-P11. Amendments in RULEBOOK section 12: 20 rows.
- Tier rule (section 0): the `stoll`/`stod` scanners go to a Sonnet task. It matched glibc on every corpus input first
  time. The rest of the Part was written by Haiku 5.5 and needed repair in both Stages.
- Stage 2 API (section 6) was filled by the governor on the user's instruction and checked against `tinyjson.h:90-150`
  and `626-1077`.

## Review

| Review                    | Verdict | Findings                                            | Outcome                                   |
|---------------------------|---------|-----------------------------------------------------|-------------------------------------------|
| Sonnet, Stage 1 (7115bf6) | fix     | 3 behaviour BLOCK, 4 marker BLOCK, 4 GAP, 1 RULE    | all fixed; Judge strengthened (5561 to 5755 inputs) |
| Sonnet, Stage 2 (attempt 1)| fix    | 2 marker BLOCK, 2 GAP, 1 RULE, 10 NIT               | markers, `peek` signature, `skip_char` D4 fixed |

The three Stage 1 behaviour BLOCKs sat where the corpus could not reach: a code point above U+00FF cast to `u8` at the
value start, a leak in `assign`, and a NUL key in `index_key`; a fourth defect (`U32ToU8` applied per code point
instead of once) came as a GAP. The 5561-input corpus and the translated tests had passed all four. They became corpus inputs (`broken_` P2, `utf8_` window edges, `%f` ties,
`stream_probes` on the wide stream).

## Tokens (output tokens, `SPEND-MODELS.tsv`, 2026-10-11 23:58 and later)

| Model                         | Output tokens | Note                                                           |
|-------------------------------|---------------|----------------------------------------------------------------|
| claude-sonnet-5-5             | 413,347       | governor, Judge builders, scanner and review subagents         |
| claude-haiku-5-5              | 86,410        | stress-test rounds and part of the implementer runs            |
| claude-haiku-4-5-20251001     | 3,936         | mislabelled: subagent tokens often show under this id          |
| claude-opus-5-5               | 3,152         | rulebook and stress-test review                                |

Total about 506,800 output tokens over 33 phase rows (T1-mapping ... E5-tinyjson-s2-review). Cache reads are about
174M tokens; there is no dollar meter, so cost is not stated. Attribution by model is approximate: the rows cannot
tell Haiku subagents from Sonnet ones reliably.

## Time

Calendar time: 2026-10-11 20:31 (pin) to 2026-10-12 about 23:30, with a rate-limit stop and one session compaction. Corpus step per Judge run: about 50-60 s.

## Interventions by the user

| Intervention                                                    | On the allowed list (decision 39)? | Logged as a Skill bug          |
|-----------------------------------------------------------------|------------------------------------|--------------------------------|
| Start the deny-ruled session                                    | yes (a)                            | -                              |
| Approve Deviations D1-D16, D3 and D5 rule, section 12 rows      | yes (b)                            | -                              |
| Edit `.claude/settings.local.json` (autoMode allow for Agent)   | no, permission classifier          | WORK-LOG 58                    |
| "Run the work here if a subagent is impossible"                 | no, classifier denied Agent launch | WORK-LOG 58, 59, 63            |
| Choose the implementer model (Haiku 5.5 to Sonnet 5.5)          | no, a choice                       | WORK-LOG 63                    |
| Approve the section 4 marker rule and the Judge gap changes     | yes (b)                            | -                              |
| Delegate the section 6 review to the governor                   | yes (b)                            | -                              |
| Ask for the guard to stop locking the Judge during fan-out      | no, a Skill design change          | sent to the Skill session      |
| Budget check                                                    | yes (c)                            | -                              |

Skill bugs found in the run and filed or fixed: the Stage 1 gate PASS is vacuous when the baseline already fails the
Judge checks (also at Stage 2 for `fmt`, `unwrap`, `allow`); a part id with a slash needs `migration/gate/src/`;
rollback crashes on an untracked `--also` path; the guard refuses read-only Bash that names a protected path with a
`>`; `fmt` checks the installed Judge files; a Haiku `STATUS: done` was false twice; `judge.md` section 5 has no
shim recipe for a section 6 adapter.

## Recommended next step

Apply the three Skill changes already sent to the Skill-editing session: make the gate report `N of M` and refuse
`mark done` unless the Judge is green and nothing is excused; replace the fan-out lock with a snapshot hash check at
the gate; make Sonnet 5.5 the implementer default and treat `STATUS: done` as a claim, not a result. Then run a second
C++ Pilot (tinyxml2) as the check that a fresh Translation session needs no intervention outside the allowed list.
