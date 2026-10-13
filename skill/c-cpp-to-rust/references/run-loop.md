# Running a stage unattended: `scripts/run.py`

`run.py` drives one stage of one Pilot: for each Part it takes a snapshot, runs the implementer, runs the gate, runs
the reviewer, commits and marks the Part `done`. The mechanical steps are the Skill's scripts; the two model steps are
headless Claude Code runs (`claude -p`). Decision 66. **Status: built and tested against a fake `claude`; not yet run
against the real one**, so the first real run is the test of the `claude -p` flags and the prompt slot filling.

## Command

```bash
python3 -I skill/c-cpp-to-rust/scripts/run.py pilots/<name> --stage 1 [--max-attempts 5] [--budget-output-tokens N]
```

Without `--go` nothing is spent: the driver checks the preconditions, prints the `claude -p` commands, the snapshot and
gate commands and the budget, then exits 0. Add `--go` to start. `--deviations ask|policy` decides who approves §8 rows: `ask` (default) stops on every row that is not `approved`
by you, `policy` also accepts `approved (policy)`. Other options: `--model sonnet`,
`--reviewer-model sonnet`, `--max-turns 60`, `--settings <file>`, `--gate-cmd` / `--snapshot-cmd` (templates with
`{root} {ws} {name} {stage} {part}`; default `make -C {root} pilot-{name} STAGE={stage} PART={part}` and
`pilot-{name}-snapshot`). Stage 2: `--stage 2` (reads `parts-stage2.tsv`, ticket 06).

## What it needs before it starts

| Need                                                            | Checked as                                           |
|-----------------------------------------------------------------|------------------------------------------------------|
| Tickets before the fan-out ticket (04, Stage 2: 06) are `done`  | `migration/tickets/NN-*.md` `**Status:**` lines      |
| Rulebook approved                                               | `Status: deviations-approved` or `stress-tested`     |
| No §8 row `proposed`; `approved (policy)` only with `--deviations policy` | the status cell of each `D<n>` row |
| Mapping, Judge, crate skeleton with every Part's module         | by the tickets (01-03); not checked separately       |
| `make pilot-<name>` and `make pilot-<name>-snapshot` targets    | or pass `--gate-cmd` / `--snapshot-cmd`              |
| `--settings skill/c-cpp-to-rust/assets/settings.json` allow rules   | the implementer runs with `--permission-mode acceptEdits` |

## What it does per Part

1. `workqueue.py mark <part> in_progress` (the attempt counter), then the snapshot command (records HEAD, the baseline
   and the Judge hashes, decisions 58 and 63).
2. `claude -p --model sonnet` with the filled Stage prompt (rulebook sections per stage, the idiom tables, source with
   line numbers, types and signatures of the Parts already `done`, the previous failure, translation-memory hits).
3. The gate command. Its exit code decides; the implementer's `STATUS: done` is a claim only.
4. `claude -p` reviewer with `review-prompt.md`. A `BLOCK` line or a verdict other than `accept` restores the target
   and counts as a failed attempt. `GAP` and `RULE` lines go to `RUN-NOTES.md` and stop the driver after this Part.
5. `git commit` of the target only, then `workqueue.py mark <part> done --commit <sha>`.

## When it stops (exit 3) and how to resume

| Stop                                              | You do                                                              |
|---------------------------------------------------|---------------------------------------------------------------------|
| Precondition missing                              | fix it (the message names it), rerun                                |
| Part `stubbed` after the attempt limit            | write the stub (`todo!()` bodies), RUN-NOTES section, rerun         |
| Output-token budget past 70% of the cap           | raise the cap or finish by hand, rerun                              |
| Review `GAP` / `RULE`                             | apply it to the corpus or the rulebook (own commit), rerun          |
| `mark done` refused (excused checks, attempts 0)  | make the Judge green, or `--accept-excused` yourself, rerun         |

State is `parts.tsv`, the tickets and git: **rerun the same command** to resume. A `failed` Part is retried; an
`in_progress` Part is resumed first.

## Where it logs and how to read the summary

- `WORK-LOG.md`: one entry per run (`run.py <name> stage <n>`): Parts accepted, review findings kept, the stop reason.
- `migration/attempts.tsv`: one row per implementer run (written by the snapshot command).
- `migration/RUN-NOTES.md`: `## Review findings <part> stage <n>` for GAP and RULE lines.
- Tokens: `spend.py` (the driver reads `spend.py sum --since <start>` for the budget; log the phase with
  `spend.py log` as in SKILL.md).

Final summary: `Parts accepted this run` lists the Parts committed in this run, `review findings kept` counts GAP,
RULE and NIT lines, and `STOPPED:` names the reason above. No `STOPPED:` line and exit 0 mean every Part of the stage
is `done`; run `workqueue.py verify` next.

## Known limits

- The prompt's idiom rows are the whole table, not the tag-matched rows; `{c2rust}` is `none`.
- A `claude -p` implementer carries no `agent_id`, so `guard.py` does not see it; the gate's hash check does.
- It never writes the Rulebook, the Judge, §6 or a stub: those stop for the model or the user.

## Workspace layout (decision 72)

`run.py <ws>` works on any workspace that has `<ws>/migration/`. With a `migration/layout.json` (every workspace made
by `tickets.py init` since decision 72) the root of git operations and of `claude -p` is `<ws>` itself, the gate and
snapshot commands default to `make -C <ws>/migration gate|snapshot STAGE=.. PART=..` (the Judge's `gate.mk` supplies
`GATE_ARGS`), and the work log is `<ws>/migration/WORK-LOG.md`. Without `layout.json` the legacy conventions of the
rest of this file apply. `make -C <ws>/migration run` and `run-go RUN_ARGS="..."` call the driver.

## Whole run: `run.py <ws> --all` (decision 74)

`scripts/run_all.py` loops: first ticket that is not `done` with every blocker `done` -> one step -> re-read the ticket
files. Steps: no `migration/` -> `tickets.py init --lang` (`--lang`, else detected from the file suffixes); ticket 01
with no `parts.tsv` -> `make parts`; 01, 02, 05 -> governor run on `--planner-model` (default `opus`); 03, 07 ->
governor on `--model`; 04, 06 -> this file's Part loop for stage 1 or 2, then a governor run on `--model` that
verifies, ticks and marks the ticket. A governor run is `claude -p --model M --permission-mode acceptEdits
--allowed-tools "Read,Edit,Write,Grep,Glob,Bash(python3 -I:*),Bash(make:*),Bash(cargo:*),Bash(git:*),Bash(ls:*)"
--max-turns --governor-turns`, with the ticket text and the Skill path in the prompt. It ticks tasks and marks its
ticket itself; the driver checks the file afterwards, never the reply. After each step `migration/` is committed.

| stop (exit 3)                                          | what the user does                                |
|--------------------------------------------------------|---------------------------------------------------|
| `migration/QUESTIONS.md` is not empty                  | answer in RULEBOOK or the ticket, delete the file |
| a deviation row is `proposed`                          | approve, or rerun with `--deviations policy`      |
| a ticket is open after `--max-attempts` governor runs  | read the last lines of its transcript, fix, rerun |
| a Part is `stubbed`                                    | write the stub (section 6), rerun                 |
| output tokens passed 70% of `--budget-output-tokens`   | raise the budget, rerun                           |

The budget counts the whole run (governor and Part steps). Not covered: the answer to a question is not fed back
automatically; the governor runs have no `agent_id`, so only `gate.py`'s hash check guards the Judge (decision 63).

