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

## Layout

- `skill/c-to-rust/` — the Skill: `SKILL.md`, `references/`, `scripts/` (stdlib Python), `assets/`.
- `pilots/<name>/` — one Pilot each: `UPSTREAM` (pinned source), `src/` (git-ignored clone),
  `rust/` (the crate), `migration/` (rulebook, `parts.tsv`, run notes, harness).
- `experiments/` — one script per question, docstring states the question and the answer.
- `tests/` — pytest for the scripts.
