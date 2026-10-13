---
name: c-cpp-to-rust
description: Port a C or C++ project to Rust in two gated stages (Faithful port, then Idiomatic pass), resumable from files and git. Use when translating, porting or migrating C/C++ code to Rust, or when resuming such a port from a `migration/` directory.
---

# c-cpp-to-rust

Translate a Source project into Rust **Part by Part**, each Part accepted only when the **Judge**
passes. Vocabulary (Part, Faithful port, Idiomatic pass, Same behaviour, Judge, Differential harness):
the project's `CONTEXT.md`. State lives on disk; this session's memory is never the source of truth.

**Where things are (decision 72).** `<source>` is the original's source directory and `<crate>` the Rust crate
(`<ws>` and `<ws>/migration/rust` in a new workspace; `layout.py` prints both). `<ws>` is the folder the Skill works in: the directory the session was opened in
(the project itself, e.g. `~/Programming/CppToRust_Translations/tinyxml2`), or the path the user names. `S` is the
Skill's `scripts/` directory (installed: `~/.claude/skills/c-cpp-to-rust/scripts`). **Everything the Skill creates is
under `<ws>/migration/`** (tickets, Makefile, `layout.json`, inventory, parts, RULEBOOK, harness, the Rust crate in
`migration/rust/`); nothing is created elsewhere, and no Python file is copied there: scripts run from `S`, either
directly (`python3 -I $S/<script>.py`, every script has `--help`) or through `make -C <ws>/migration <target>` (the
Makefile holds `WORK_DIR` and the absolute path of `S`; `make help` lists the targets). The original source is read in
place and never edited; `layout.py <ws>` prints where the source and crate are (a workspace made before decision 72,
`pilots/<name>` with `src/` and `rust/`, has no `layout.json` and keeps the legacy layout). Never make `<ws>` the
working directory of a build tool or interpreter (untrusted code).

**First call in a folder without `migration/`:** this is a new run. Tell the user the folder you will work in, check
it contains C or C++ sources and is a git repository (`git init` if the user agrees; gate rollback needs one), then run
`python3 -I $S/tickets.py --ws <ws> init --lang c|cpp`: it creates `migration/` (a git repository of its own) with the seven
tickets, the Makefile, `layout.json` and `.gitignore`, and makes the first commit. Then follow the first READY ticket below.

## Start of every session: find where you are

Run `python3 -I $S/workqueue.py --parts <ws>/migration/parts.tsv verify` and `status` (and the same for
`parts-stage2.tsv` if it exists), read `<ws>/migration/RUN-NOTES.md`, then take the **first** branch
below whose condition holds. `verify` failing means a Part marked `done` has no file or no commit: run
`verify --fix` and continue. `verify` also needs a gate-pass row for every `done` Part in
`<ws>/migration/gate-passed.tsv` (written by `gate.py`); for Parts accepted before that file existed pass
`--no-gate-record` (jsmn Stage 1).

### 0. Tickets (every session)

`make -C <ws>/migration status` (rulebook line, tickets, Parts per stage) and `make -C <ws>/migration tickets` (every
ticket with its tasks); they wrap `tickets.py --ws <ws> status [--tasks]` and `workqueue.py status`. If
`<ws>/migration/tickets/` does not exist, run `tickets.py --ws <ws> init` (`--lang cpp` for a C++ Pilot: adds the
reference-build and test-patch tasks) first: it writes the seven tickets of the run
(Mapping, Stage 1 Judge, stress test, Stage 1 Faithful port, Stage 2 Judge, Stage 2 Idiomatic pass, report) with their
blocking edges, and the Makefile. For a run started before the Makefile existed: `tickets.py --ws <ws> makefile`.

Work the `READY` ticket: `tickets.py mark NN in_progress` when you start it. **Tick each task yourself the moment its
evidence exists**, not at the end: `tickets.py --ws <ws> tick NN "<text of the task>"` (the text must match exactly one
open task). A task you decide not to do is edited to `- [x] <task> (skipped: <reason>)`. `mark NN done` refuses while a
task is `[ ]` and needs `--brief "<what was produced, commit, numbers; max 8 lines>"`, which is written below the
`-----------` line. `tickets.py --ws <ws> show NN` prints the ticket (Tasks with their how-steps, Acceptance) and the
briefs of its blockers; `mark NN in_progress` prints the same briefs. The ticket state and `parts.tsv` are the resume points; commit ticket changes with the step they belong
to.

**Repository (decision 73).** `<ws>/migration/` is its own git repository (`tickets.py init` runs `git init` there and
commits). Everything the Skill produces is committed there and nowhere else: `tickets.py mark` commits the ticket
state, `run.py` commits each accepted Part and the stage summary, and the governor commits its own steps with
`make -C <ws>/migration commit MSG="..."` (or `git -C <ws>/migration add -A && git -C <ws>/migration commit`). The
original project's own git is never touched. Gate rollback, `workqueue verify` and `run.py` address that repository.
`migration/.gitignore` keeps `rust/target/`, `harness/work/` and `gate/` out. A legacy Pilot (`pilots/<name>`) keeps
its old rule: commits go to the repository that contains it (decision 69).

### 1. `<ws>/migration/parts.tsv` is absent → Stage 0 Mapping (map and rules)
1. `make -C <ws>/migration spend-now`; note the timestamp.
2. `inventory.py <source> --lang c|cpp [--compile-commands ...] [--exclude GLOB]... --out <ws>/migration/inventory.tsv`
   (C++ without a compile database: clang's AST gives exact overloads, `calls.tsv` and `facts.tsv`, falling back to the
   regex pass per file; `--cxx-flag` passes `-std=` or `-include`, `--no-ast` forces regex. Vendored directories such as `fuzzing/`, `unity/`, `vendor/`, `third_party/` are skipped; add `--exclude` globs for
   anything else that is not library code. Test and example files are never Parts.)
3. `depmap.py <ws>/migration/inventory.tsv --source-root <source> --out-dir <ws>/migration`
4. `tier.py <ws>/migration/inventory.tsv <ws>/migration/order.txt --out <ws>/migration/parts.tsv`
5. Build the Judge and prove it can fail: `references/judge.md`. The crate does not exist yet: validate the Stage 1
   Judge against a scratch crate (raw c2rust transpile per feature set, or a stub for C++), judge.md §4a.
6. Draft `<ws>/migration/RULEBOOK.md` from `references/rulebook-template.md` and
   `references/idiom-table-c.md` (Opus). **C++ Pilot:** also `references/rulebook-template-cpp.md` (replaces §0, §3,
   §5, §7, §10 and adds the `Reference build` slot) and `references/idiom-table-cpp.md`; fill the reference build
   first (`references/judge.md` §8), because the Judge and every Deviation depend on it. Every `config-macro:` tag gets a §3 row; every UB or platform
   behaviour gets a §8 Deviation row with status `proposed`.

**Policy approval (decision 65), only when the user chose it** (`run.py --deviations policy`, or said so in the
session; without that every row waits for the user, decision 11). A §8 row whose Rust behaviour reproduces what the original does (observed with
the Reference build or on the C build, or defined by the language standard) follows the user's rule "match the origin
as closely as possible": its status is `approved (policy)` and it does not wait. Every other row stays `proposed`:
no single origin result (undefined behaviour, a platform dependency, an overload the compiler resolves
differently), a fix instead of a kept bug, or a row that changes the public API.

**Stop** only if a `proposed` row remains: show the user those rows and wait. Done when every §8 row is `approved`,
`approved (policy)` or `vetoed` and the rulebook status line says `deviations-approved`. The user reviews the
`approved (policy)` rows afterwards, not before; a veto becomes a change ticket.

### 2. Rulebook approved, no stress test recorded → stress test (Opus reviews)
Translate the 3 hardest `judgment` Parts (fewer if the Pilot has fewer) **twice** each with
`references/stage1-prompt.md`, into scratch copies of the crate (never into the accepted target).
Diff the two translations of each Part. Every difference that two agents decided differently is a
missing rule: propose it under `## Rule proposals` in `RUN-NOTES.md`; Opus applies accepted ones.
Done when the rulebook status line says `stress-tested (<date>)` and RUN-NOTES records the diffs.

### 3. `parts.tsv` has `todo`, `in_progress` or `failed` Parts → Stage 1 (Faithful port)
1. Protect the Rulebook and the Judge for the fan-out (decision 44). The session was started from the repo root with
   `claude --settings skill/c-cpp-to-rust/assets/settings.json`, which loads the static deny-rules and the `guard.py`
   hook. Run `tickets.py mark 04 in_progress` (Stage 2: ticket 06), then **verify**: `guard.py check
   <ws>/migration/RULEBOOK.md` must print REFUSE. The hook refuses only tool calls of subagents (decision 62): you
   may still edit the Rulebook and the Judge yourself, and the implementers may not. If `check` prints ALLOW, stop and
   tell the user.
   A crate with several Parts needs a `rust/src/lib.rs` that declares them (`pub mod <stem>;`) and the cargo features of
   rulebook §3, written by the governor before the first batch (`tier.py --crate-root-part NAME` makes one Part the
   root).
2. `workqueue.py next --batch N`; for each Part run the loop in `references/stage1-prompt.md`
   (snapshot → implementer → gate → review with `references/review-prompt.md` → commit → mark).
   Parts with no unfinished deps may run in parallel; each Part writes only its own `target_path`.
3. After each batch: `make -C <ws>/migration spend PHASE=<phase> SINCE=<timestamp> NOTE="..."`.

Done when every row is `done` or `stubbed` and `verify` passes.

### 4. All Stage 1 Parts `done`/`stubbed` → Stage 2 (Idiomatic pass)
1. Rulebook §6 (Stage 2 API) must be filled (Opus). If empty, stop and ask for it.
2. Build and validate the Stage 2 Judge (adapter, Rust driver, translated tests, planted mutant):
   `references/judge.md` §5. The guard allows these writes while ticket 06 is not `in_progress`; commit them on their
   own, then `tickets.py mark 05 done`.
3. Create `<ws>/migration/parts-stage2.tsv` (copy of `parts.tsv`, all `todo`, attempts 0, no commit).
4. Run the loop of step 3 with `references/stage2-prompt.md`, gate `--stage 2`. If the crate root carries a rulebook-
   approved `#![allow(<lint>)]`, pass `--allow-lint <lint>` to every gate call; without it the `allow` check fails and
   only the baseline hides it.

Done when every Stage 2 row is `done` and the gate passes at Stage 2 for every feature set (PLAN §6).

### 5. Everything `done` → report
Report: Parts done / stubbed / failed per stage; the Deviations table with statuses; open `TODO(port)`
count (`grep -rn "TODO(port)" <crate>/src`); tokens from `SPEND.md` for this Pilot; one recommended
next step.

## Unattended run (`scripts/run.py`)

Once tickets 01-03 are done and the rulebook is approved, one command runs a stage: `python3 -I
$S/run.py <ws> --stage 1 [--max-attempts 5] [--budget-output-tokens N] [--go]`.
Without `--go` it only prints the plan. Preconditions, stops, resume and logs: `references/run-loop.md`. The model
steps it does not do (mapping, Judge, rulebook, §6, stubs) stay with the governor.

## Models

- Opus: rulebook, Deviations, Stage 2 API, stress-test diff review, first review per Pilot.
- Sonnet: governing (this file: the governor session), Judge building, reviews, and the implementers (Part bodies and
  fixes, the implementer prompts). The default implementer is Sonnet 5.5 (`claude-sonnet-5-5`): an Agent launch says
  `model: sonnet`. Decision 64: in tinyjson Haiku failed the Stage 1 gate (30 compile errors), needed three fix rounds
  on rules that were in the prompt and reported `STATUS: done` on a Stage 2 file with 12 compile errors; Sonnet wrote
  the scanners right first time.
- Haiku: only when the user asks for it (a cost experiment). The default Haiku is Haiku 5.5 (`claude-haiku-5-5`).
- Scripts: everything mechanical. If a step repeats by hand twice, it belongs in a script.

## Rules that hold in every branch

- **Done** = gate passed and the Part's commit exists. One commit per accepted Part.
- Commands use absolute paths, `git -C <dir>`, or a subshell `( cd <dir> && ... )`; never leave the shell's cwd inside
  `<ws>`. The guard finds the repo from `$CLAUDE_PROJECT_DIR`, so a stray `cd` no longer breaks it, but a build tool
  with `<ws>` as cwd still loads untrusted code (judge.md §7).
- Analysis scripts for a Pilot go to `<ws>/experiments/` (docstring: the question and the answer; commit them). The
  repo-level `experiments/` belongs to the Skill-editing session and is read-only here.
- A review `RULE` or `GAP` that arrives mid-fan-out: commit or revert the ledger first (`gate-passed.tsv`,
  `parts*.tsv`; a gate rollback leaves them dirty), then `tickets.py mark 04 todo` (or 06), RULEBOOK commit with a §12
  row, Judge commit, `mark ... in_progress`, red run on the old crate, then the implementer.
- **Every implementer run is preceded by `workqueue.py mark <part> in_progress`**, also when the Part is already
  `in_progress` (a retry after a review, a Judge change or a `RULE`): the counter is the only record of attempts and
  the limit below reads it. `gate.py --snapshot` also appends a row to `migration/attempts.tsv` (one row = one
  implementer run: snapshot only at the start of a run); `mark done` raises `attempts` to that row count and refuses 0,
  `verify` flags a lower value.
- A denied implementer launch is the user's or the classifier's answer: do not repeat the same call. Say so in
  `RUN-NOTES.md`, then either write that Part in this session (the governor may) or stop and tell the user. The
  allow rules in `assets/settings.json` (`Agent(model:sonnet)`) exist so this does not happen.
- An implementer's `STATUS: done` is not evidence. Before the gate, run `cargo check` on the crate yourself; the
  gate decides acceptance, the report decides nothing.
- Up to 5 attempts per Part; then `stubbed` (real signatures, `todo!()` bodies, RUN-NOTES entry).
- The rulebook and the Judge are read-only for the implementers during a batch. The governor edits them freely, as
  their own commit before the next run (new corpus inputs from a review `GAP`, a `RULE` amendment). Two checks
  catch an implementer that touches them: `guard.py` refuses a subagent's edit (decision 62), and `gate.py` fails an
  attempt when a hashed file changed since the snapshot (decision 63; `--accept-judge-change` for a governor edit
  made between snapshot and gate).
- Report numbers only from commands actually run; failing results are reported, not retried silently.
