---
name: c-to-rust
description: Port a C or C++ project to Rust in two gated stages (Faithful port, then Idiomatic pass), resumable from files and git. Use when translating, porting or migrating C/C++ code to Rust, or when resuming such a port from a `migration/` directory.
---

# c-to-rust

Translate a Source project into Rust **Part by Part**, each Part accepted only when the **Judge**
passes. Vocabulary (Part, Faithful port, Idiomatic pass, Same behaviour, Judge, Differential harness):
the project's `CONTEXT.md`. State lives on disk; this session's memory is never the source of truth.

Paths below are relative to the repository root; `<ws>` is the Pilot workspace (`pilots/<name>`),
`S` is `skill/c-to-rust/scripts`. Run every script as `python3 -I $S/<script>.py`; every script has
`--help`. Never make `<ws>` the working directory of a build tool or interpreter (untrusted code).

## Start of every session: find where you are

Run `python3 -I $S/workqueue.py --parts <ws>/migration/parts.tsv verify` and `status` (and the same for
`parts-stage2.tsv` if it exists), read `<ws>/migration/RUN-NOTES.md`, then take the **first** branch
below whose condition holds. `verify` failing means a Part marked `done` has no file or no commit: run
`verify --fix` and continue. `verify` also needs a gate-pass row for every `done` Part in
`<ws>/migration/gate-passed.tsv` (written by `gate.py`); for Parts accepted before that file existed pass
`--no-gate-record` (jsmn Stage 1).

### 0. Tickets (every session)

`python3 -I $S/tickets.py --ws <ws> status`. If `<ws>/migration/tickets/` does not exist, run `tickets.py --ws <ws>
init` first: it writes the seven tickets of the run (Mapping, Stage 1 Judge, stress test, Stage 1 Faithful port,
Stage 2 Judge, Stage 2 Idiomatic pass, report) with their blocking edges. Work the `READY` ticket, `tickets.py mark NN
in_progress` when you start it and `mark NN done` when its acceptance list holds. The ticket state and `parts.tsv`
are the resume points; commit ticket changes with the step they belong to.

### 1. `<ws>/migration/` is absent → Stage 0 Mapping (map and rules)
1. `make spend-now`; note the timestamp.
2. `inventory.py <ws>/src --lang c|cpp [--compile-commands ...] [--exclude GLOB]... --out <ws>/migration/inventory.tsv`
   (vendored directories such as `fuzzing/`, `unity/`, `vendor/`, `third_party/` are skipped; add `--exclude` globs for
   anything else that is not library code. Test and example files are never Parts.)
3. `depmap.py <ws>/migration/inventory.tsv --source-root <ws>/src --out-dir <ws>/migration`
4. `tier.py <ws>/migration/inventory.tsv <ws>/migration/order.txt --out <ws>/migration/parts.tsv`
5. Build the Judge and prove it can fail: `references/judge.md`. The crate does not exist yet: validate the Stage 1
   Judge against a scratch crate (raw c2rust transpile per feature set, or a stub for C++), judge.md §4a.
6. Draft `<ws>/migration/RULEBOOK.md` from `references/rulebook-template.md` and
   `references/idiom-table-c.md` (Opus). Every `config-macro:` tag gets a §3 row; every UB or platform
   behaviour gets a §8 Deviation row with status `proposed`.

**Stop.** Show the user the Deviations table and wait. Done when every §8 row is `approved` or
`vetoed` by the user and the rulebook status line says `deviations-approved`.

### 2. Rulebook approved, no stress test recorded → stress test (Opus reviews)
Translate the 3 hardest `judgment` Parts (fewer if the Pilot has fewer) **twice** each with
`references/stage1-prompt.md`, into scratch copies of the crate (never into the accepted target).
Diff the two translations of each Part. Every difference that two agents decided differently is a
missing rule: propose it under `## Rule proposals` in `RUN-NOTES.md`; Opus applies accepted ones.
Done when the rulebook status line says `stress-tested (<date>)` and RUN-NOTES records the diffs.

### 3. `parts.tsv` has `todo`, `in_progress` or `failed` Parts → Stage 1 (Faithful port)
1. Protect the Rulebook and the Judge for the fan-out (decision 44). The session was started from the repo root with
   `claude --settings skill/c-to-rust/assets/settings.json`, which loads the static deny-rules and the `guard.py`
   hook. Run `tickets.py mark 04 in_progress` (Stage 2: ticket 06), then **verify**: an `Edit` of
   `<ws>/migration/RULEBOOK.md` must be refused by the guard. If it is not refused, stop and tell the user.
   A crate with several Parts needs a `rust/src/lib.rs` that declares them (`pub mod <stem>;`) and the cargo features of
   rulebook §3, written by the governor before the first batch (`tier.py --crate-root-part NAME` makes one Part the
   root).
2. `workqueue.py next --batch N`; for each Part run the loop in `references/stage1-prompt.md`
   (snapshot → implementer → gate → review with `references/review-prompt.md` → commit → mark).
   Parts with no unfinished deps may run in parallel; each Part writes only its own `target_path`.
3. After each batch: `make spend PHASE=<phase> SINCE=<timestamp> NOTE="..."`.

Done when every row is `done` or `stubbed` and `verify` passes.

### 4. All Stage 1 Parts `done`/`stubbed` → Stage 2 (Idiomatic pass)
1. Rulebook §6 (Stage 2 API) must be filled (Opus). If empty, stop and ask for it.
2. Build and validate the Stage 2 Judge (adapter, Rust driver, translated tests, planted mutant):
   `references/judge.md` §5. The guard allows these writes while ticket 06 is not `in_progress`; commit them on their
   own, then `tickets.py mark 05 done`.
3. Create `<ws>/migration/parts-stage2.tsv` (copy of `parts.tsv`, all `todo`, attempts 0, no commit).
4. Run the loop of step 3 with `references/stage2-prompt.md`, gate `--stage 2`.

Done when every Stage 2 row is `done` and the gate passes at Stage 2 for every feature set (PLAN §6).

### 5. Everything `done` → report
Report: Parts done / stubbed / failed per stage; the Deviations table with statuses; open `TODO(port)`
count (`grep -rn "TODO(port)" <ws>/rust/src`); tokens from `SPEND.md` for this Pilot; one recommended
next step.

## Models

- Opus: rulebook, Deviations, Stage 2 API, stress-test diff review, first review per Pilot.
- Sonnet: governing (this file: the governor session), Judge building, reviews, escalated retries.
- Haiku: Part bodies and fixes (the implementer prompts).
- Scripts: everything mechanical. If a step repeats by hand twice, it belongs in a script.

## Rules that hold in every branch

- **Done** = gate passed and the Part's commit exists. One commit per accepted Part.
- Commands use absolute paths, `git -C <dir>`, or a subshell `( cd <dir> && ... )`; never leave the shell's cwd inside
  `<ws>`. The guard finds the repo from `$CLAUDE_PROJECT_DIR`, so a stray `cd` no longer breaks it, but a build tool
  with `<ws>` as cwd still loads untrusted code (judge.md §7).
- Analysis scripts for a Pilot go to `<ws>/experiments/` (docstring: the question and the answer; commit them). The
  repo-level `experiments/` belongs to the Skill-editing session and is read-only here.
- Up to 5 attempts per Part; then `stubbed` (real signatures, `todo!()` bodies, RUN-NOTES entry).
- The rulebook and the Judge are read-only during a batch. Judge changes (new corpus inputs from a
  review `GAP`) are their own commit: `tickets.py mark 04 todo` (or 06), make and commit the change, mark it
  `in_progress` again, then the Part is re-gated. The guard enforces this; the commit trail shows it.
- Report numbers only from commands actually run; failing results are reported, not retried silently.
