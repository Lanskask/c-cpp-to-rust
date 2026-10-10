# settlers_to_rust

A Claude Code **Skill** (`skill/c-to-rust/`) that translates C/C++ projects into Rust in two
stages (Faithful port, then Idiomatic pass), resumable after a crash. It is proven on small
Pilots (`pilots/`) first. Vocabulary: `CONTEXT.md`. Plan: `PLAN.md`. Decisions: `docs/DECISIONS.md`.

## Commands

| Command                                            | What it does                                                                                                                                          |
|----------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------|
| `make setup`                                       | create the `uv` environment (pytest)                                                                                                                  |
| `make test`                                        | run pytest for the Skill scripts                                                                                                                      |
| `make lint`                                        | `ruff` lint and format check, then `ty` type check                                                                                                    |
| `make fmt`                                         | auto-format and auto-fix Python                                                                                                                       |
| `make pilot-jsmn`                                  | gate the jsmn Part over all 4 feature sets: cargo check/test, original C tests via staticlib, differential harness (`STAGE=2` adds clippy/fmt/unsafe) |
| `make pilot-jsmn-snapshot`                         | record HEAD and baseline before editing the Part                                                                                                      |
| `make spend-now`                                   | print a UTC timestamp; note it at the start of a phase                                                                                                |
| `make spend PHASE=E1 SINCE=<timestamp> NOTE="..."` | sum Claude Code token usage since then into `SPEND.md`                                                                                                |

Requires: `uv`, Python ≥ 3.12, Rust (`cargo`), `clang`. Optional: `c2rust` (C-only fast path).

## Running the Skill

The Skill is not installed as a global Claude Code skill. Start a session from the repo root and point it at
`skill/c-to-rust/SKILL.md`:

```bash
claude --settings skill/c-to-rust/assets/settings.json --model sonnet
```

Before the first run, check these (each one has bitten a session in E3):

1. **Deny-rules are active.** `--settings` loads `skill/c-to-rust/assets/settings.json`, which makes the rulebook, the
   Judge files, `parts*.tsv` and the upstream source read-only. The first thing the session must do is try an `Edit` of
   `pilots/<name>/migration/RULEBOOK.md` and see it refused (SKILL.md step 3.1).
2. **Subagents are allowed in auto mode.** With `"defaultMode": "auto"` in `~/.claude/settings.json`, the classifier may
   refuse an `Agent` spawn ("Auto-Mode Bypass", "Create Unsafe Agents"). Add an `autoMode.allow` entry for the Skill's
   subagents (decision 38 has the wording). The deny-rules file cannot fix this. If a spawn is refused anyway, the
   session does the step inline and reports it.
3. **Log every subagent.** After each one: `make spend PHASE=<phase> SINCE=<timestamp> NOTE="..."` and an entry in
   `WORK-LOG.md` (append only).
4. **Markdown style.** Files written by the session follow `~/.claude/md-style.md` (120 columns, aligned tables).

## Layout

- `skill/c-to-rust/` — the Skill: `SKILL.md`, `references/`, `scripts/` (stdlib Python), `assets/`.
- `pilots/<name>/` — one Pilot each: `UPSTREAM` (pinned source), `src/` (git-ignored clone),
  `rust/` (the crate), `migration/` (rulebook, `parts.tsv`, run notes, harness).
- `experiments/` — one script per question, docstring states the question and the answer.
- `tests/` — pytest for the scripts.
