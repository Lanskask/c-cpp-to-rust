# PLAN: build the `c-to-rust` Skill

Self-contained brief for the executor (Opus). Written 2026-10-10 after a four-round design interview with the user.
**Do not re-ask decisions already settled.** Read first, in this order:
1. `CONTEXT.md`: vocabulary (Pilot, Part, Faithful port, Idiomatic pass, Same behaviour, Judge, Differential harness, Skill). Use these words.
2. `docs/DECISIONS.md`: 15 settled decisions with reasons.
3. `RESEARCH-FINDINGS.md`: literature and prior-art notes, with what was and wasn't verified.

Source of the user's standing rules: `~/.claude/CLAUDE.md` (see §9).

---

## 1. Goal

A reusable Claude Code **Skill** that translates a C/C++ project into Rust safely, and can **resume** after a session ends or crashes.
It is proven on small **Pilots** first. Order: jsmn → cJSON → json-parser → miniz (C), then tinyxml2 → pugixml → jsoncpp (C++).
The long-term goal (a maintained Rust client for s25client) is **out of scope** for this plan; do not touch `s25client/` or widelands.

Success for a Pilot: its Faithful port passes the Judge, then its Idiomatic pass passes the acceptance rules in §6.
Success for this plan: the Skill takes jsmn end to end, and tinyxml2 through at least Stage 1, within the budget in §8.

## 2. Settled design (summary; details in DECISIONS.md)

- Two stages: **Faithful port** (Stage 1) then **Idiomatic pass** (Stage 2). Final target is idiomatic Rust with **Same behaviour**, not the same API.
- **Part** = a class, or the file holding one (for C: a file or a cohesive group of functions; never a single function).
- One separate Rust crate per Pilot. **No mixed C/C++ + Rust build.** Consequence: original C++ tests cannot be linked to Rust.
- **Judge** per Part: original tests (C only, Stage 1 only, through an exported C ABI) + **Differential harness** (same inputs to original and Rust, diff outputs) + tests translated into Rust (always for C++, and for all Pilots from Stage 2).
- State lives in `migration/` inside each Pilot's workspace: `RULEBOOK.md`, `parts.tsv`, `RUN-NOTES.md`. One git commit per accepted Part. "Done" = gate passed and commit exists. A fresh session restarts from `parts.tsv` + `git log` only.
- Stage 1 engine: **LLM by default** (the only path that can ever handle C++). `c2rust` 0.22.1 is installed at `~/.cargo/bin/c2rust` as an **optional fast path for C only**. It has never been run on a real file yet (experiment E1).
- Models: small model for Part bodies and fixes; strongest model for rulebook, stress test, reviews; scripts for everything else.
- The Skill ships (a) an **idiom table** in `references/` that the rulebook template loads, and (b) a **translation-memory cache** (SQLite FTS5 default, DuckDB optional), rebuildable from files, never the source of truth.

## 3. Repository layout to create

```
settlers_to_rust/                 (already a git repo; branch master)
  Makefile                        setup / test / lint / typecheck / pilot-<name>
  README.md                       how to use the Makefile and the Skill
  CONTEXT.md  PLAN.md  SPEND.md   (SPEND.md: token log, see §8)
  docs/DECISIONS.md               decision log (append new decisions here)
  articles/                       downloaded papers (exists)
  experiments/                    one script per ad-hoc question, with docstring: question + answer
  skill/c-to-rust/
    SKILL.md                      entry: when to use, the decision tree (see §5)
    references/
      rulebook-template.md        seeded from Anthropic kit templates/RULEBOOK.md
      idiom-table-c.md            libc -> std, types, error sentinels, allocation
      idiom-table-cpp.md          std:: -> Rust, RAII, virtual, templates, exceptions
      traps.md                    UB, overflow, std::map order, iterator invalidation, ...
      judge.md                    how to build the Differential harness
      stage1-prompt.md  stage2-prompt.md  review-prompt.md
    scripts/                      Python 3, see §4
    assets/                       settings.json deny-rules template, parts.tsv header
  pilots/<name>/                  one dir per Pilot (downloaded code is untrusted)
    UPSTREAM                      upstream URL + pinned commit (decision 18)
    src/                          git clone of upstream (read-only, git-ignored)
    rust/                         the Rust crate
    migration/                    RULEBOOK.md, parts.tsv, RUN-NOTES.md, harness/
  tests/                          pytest for scripts, with fixtures
```

Rules for downloaded code: each clone lives in its own new directory; never run an interpreter or build tool with a pilot directory as the working directory; pass paths as arguments; run Python that reads them with `-I`.

## 4. Scripts (build these first; decision 13)

All Python 3 with strict type hints, managed by `uv`, checked with `uvx ty`, tested with pytest. Each script: `--help`, exit code 0/1, machine-readable output (TSV or JSON) plus a one-line human summary. Stdlib only unless noted.

| Script | Contract |
|---|---|
| `inventory.py` | In: source root, language (`c`/`cpp`), compile commands (optional). Out: `migration/inventory.tsv` (symbol, kind, file, lines, risky-construct tags: `virtual`, `template`, `goto`, `union`, `setjmp`, `varargs`, `macro-fn`, `static-global`, `errno`, `malloc`). Uses `clang -Xclang -ast-dump=json` when a compile DB or flags are available; falls back to a regex pass and says so. |
| `depmap.py` | Include graph, call graph, class hierarchy, SCCs, leaf-first order, dead-code candidates (unreachable from tests and public API). Out: `edges.tsv`, `cycles.txt`, `order.txt`. Study `anthropics/code-migration-kit-with-claude-code` `scripts/depmap_c.py` and its fixtures first; reuse its fixtures as tests. |
| `tier.py` | Assigns each Part a tier from inventory tags: `mechanical` (POD, enums, constants, pure helpers), `judgment` (ownership, `virtual`, globals, unions), `unknown` (flag for the user or stub). Out: `parts.tsv` skeleton. |
| `queue.py` | `status`, `next --batch N`, `mark <part> <state> [--commit H] [--error ...]`, `verify` (a "done" Part must have its output file non-empty and its commit present). Derives truth from files and git, never from memory. Model on the kit's `queue_runner.mjs`. |
| `gate.py` | For one Part: record HEAD (path-scoped rollback, decision 17), `cargo check`, then `cargo test`, then Differential harness, then `unsafe` count, `clippy`, `rustfmt --check` (Stage 2 only). **Baseline-aware**: record which checks failed before the change; reject only regressions. On failure: restore the snapshot and report the first error grouped by `rustc` error code. |
| `harness.py` | Runs the original-side driver and the Rust-side driver on every file in `migration/harness/inputs/`, diffs stdout and exit code, writes `harness-report.tsv`. Drivers themselves are written per Pilot (see `references/judge.md`). |
| `spend.py` | Sums `usage` from Claude Code session transcripts (`~/.claude/projects/<project>/*.jsonl`, subagents included) between two timestamps and appends `date, phase, part, input, output, cache_create, cache_read, note` to `SPEND.md` (decision 16). |
| `tm.py` | `build` (index accepted Part pairs and error → fix from `migration/` and git into `migration/tm.sqlite`, FTS5), `search "<error or snippet>"`. Cache is disposable. |
| `snapshot.py` (optional) | Thin wrapper over git for per-Part checkpoint/rollback if `gate.py` needs it. |

`parts.tsv` columns (tab-separated, header line required):
`part_id  kind(class|file)  source_paths  target_path  tier  deps  state(todo|in_progress|done|failed|stubbed)  attempts  model  last_error  commit  updated`

Retry policy: at most 5 attempts per Part (ENCRUST). Then `stubbed`: real signatures with `todo!()` so the crate still compiles (PtrTrans), and flag in `RUN-NOTES.md`.

## 5. SKILL.md decision tree (what a fresh session does)

1. `migration/` absent → run Stage 0: `inventory.py`, `depmap.py`, `tier.py`; draft `RULEBOOK.md` from the template + idiom tables; **stop and let the user review Deviations** (decision 11).
2. Rulebook exists, no stress test recorded → translate the 3 hardest `judgment` Parts twice, diff, amend rules only (Anthropic step 2).
3. `parts.tsv` has `todo` Parts → `queue.py next`, fan out with the small model, run `gate.py`, commit per accepted Part, `spend.py` after each batch.
4. All Parts `done`/`stubbed` for Stage 1 → run Stage 2 per Part with the same gate plus the acceptance rules (§6).
5. Report: Parts done/stubbed/failed, deviations list, tokens spent, one recommended next step.

Stage 1 prompt contents (per Part): the Part's source, the callees' **signatures only**, already-translated struct definitions, the rulebook sections that apply, and the idiom-table rows matched by tags. Nothing else. Use `tm.py search` for similar past fixes before retrying.

## 6. Idiomatic pass acceptance (decision 10)

Zero `unsafe`; `cargo clippy` no warnings; `rustfmt` clean; no `unwrap()`/`expect()` on input-driven paths (elsewhere only with an invariant comment); all Judge checks pass. UB/platform behaviour follows the rulebook's **Deviations** list (decision 11).

## 7. Execution order (stop after each experiment and report tokens)

| # | Experiment | Done when |
|---|---|---|
| E0 | `git init`; Makefile, README, `uv` project, empty script stubs with tests | `make test` and `make lint` pass; first commit |
| E1 | Run c2rust on jsmn (header-only: it needs a `.c` file that includes it and a compile DB). Record in `experiments/` | c2rust output compiles, or the failure is documented and Stage 1 falls back to the LLM path |
| E2 | Build scripts §4 against jsmn fixtures; Differential harness for jsmn (token stream dump) | `gate.py` passes on the c2rust output as a no-op Part |
| E3 | Stage 1 + Stage 2 on jsmn through the full Skill | acceptance §6 met; `SPEND.md` has real numbers; **decision point: extrapolate cost for the rest** |
| E4 | cJSON (adds ownership, recursion, serialization; Unity tests) | as E3 |
| E5 | json-parser, then miniz (C binary data; tests have C++ driver files) | as E3 |
| E6 | tinyxml2 Stage 1 (C++; separate crate; harness + translated tests) | Judge passes; C++ construct table in `references/idiom-table-cpp.md` filled from real constructs |
| E7 | tinyxml2 Stage 2; then pugixml; then jsoncpp (check its licence first) | as E3, budget permitting |

Do E0–E3 before spending on anything else. If E3 shows the budget is not enough for E4+, stop and report.

## 8. Budget and measurement

The user has a Claude Code subscription. "$20" is a rough time/effort budget that **includes planning and skill writing**. No dollar meter exists, so log tokens from session transcripts via `spend.py` (decision 16; the in-session "tokens left" counter is the context window, not the quota). After E3, write a one-paragraph extrapolation for the remaining Pilots and ask the user before continuing.
Cost levers, in order: scripts instead of LLM; reachable code only; small prompts (signatures only); small model for bodies; group errors by root cause; cap retries at 5; `cargo check` in inner loops.

## 9. User's standing rules (from `~/.claude/CLAUDE.md`; follow them)

- Every project has a `Makefile` (setup, run, test, lint, deploy) and a short README.
- Python: `uv`, strict type hints, `uvx ty`, pytest.
- Commit at every working checkpoint with a descriptive message. Commit trailer: `Co-Authored-By: Claude <model name> <noreply@anthropic.com>` as given by the session's attribution reminder.
- Record non-obvious decisions in `docs/DECISIONS.md` with the reason (requirement, standard practice, or measured result).
- Save ad-hoc analysis scripts in `experiments/` with a docstring stating the question and the answer; commit them.
- Report numbers only from runs actually executed; label anything else unverified. Include negative results.
- Long runs go in the background; say what runs, where output goes, how long.
- macOS BSD userland: `sed -i ''`, no GNU-only flags.
- A message phrased as a question wants an answer, not edits.
- End substantial work with one recommended next step and why.
- Reply in the language of the user's message (English or Russian). Explain physics/ML terms for a student; the user is a backend developer with Go/Scala/AWS background.
- Suggest `/message-for-compact` when the session gets long.

## 10. Known risks and open items

- **c2rust on header-only C and on LLVM 20** is untested (E1). Its README requires LLVM 15+; build succeeded.
- **Idiom tables are written from general knowledge**, not verified against a source. Mark each row `verified: no` until a Pilot test exercises it.
- **C++ construct table** is empty until E6 shows tinyxml2's real constructs.
- **Differential harness for C++** needs a per-Pilot driver on both sides; keep drivers tiny (read input, print canonical dump).
- **Licences:** jsmn, cJSON, miniz, tinyxml2 (Zlib), pugixml: MIT/BSD/Zlib family. jsoncpp: GitHub reports `NOASSERTION`; read its LICENSE before E7. Do not copy text from `arustydev/ai` (AGPL-3.0); `rewrite-rs/skills` is BSD-3-Clause (attribute if copied).
- **Prior art to read before writing scripts** (not read in full): `anthropics/code-migration-kit-with-claude-code` (`prompts/`, `scripts/`, `skill/SKILL.md`, `CLAUDE.md`, `RUN-NOTES.md`); `rewrite-rs/skills` `port-to-rust` (PARITY-CONTRACT.md, PHASES.md, ANTIPATTERNS.md) and `port-from-cpp`; `dollspace-gay/vibe-fork` (hooks that enforce read → write → verify → commit).
- **Settings deny-rules:** install `assets/settings.json` into the Pilot workspace before any fan-out and verify it exists; the Anthropic kit saw this silently skipped once.
- **Goal project** (s25client/widelands): mixed build was rejected for now; revisit before touching it.

## 11. First actions for the executor

1. Read the three documents listed at the top. Challenge anything in this plan you think is wrong **before** building; if you change a decision, add an entry to `docs/DECISIONS.md` with the reason.
2. `git init`, commit the existing files (`CONTEXT.md`, `PLAN.md`, `RESEARCH-FINDINGS.md`, `docs/`, `articles/`; leave out `s25client/`, which is a separate checkout, via `.gitignore`).
3. E0, then E1. Report results and token spend before E2.
