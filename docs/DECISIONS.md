# Decision log

Each entry: decision, reason, who/what it came from. Newest last.

## 2026-10-09/10 — design interview rounds 1–3

1. **Pilots before the Goal project.** Order: jsmn, cJSON, json-parser, miniz (C), then C++: tinyxml2, pugixml, jsoncpp.
   s25client and widelands are set aside until the skill passes Pilots.
   _Reason:_ user requirement (cheap proof first); the $20 budget cannot cover a 155k-line C++ port.
2. **Generic Skill core plus a per-project adapter.** Developed in `skill/c-to-rust/` in this repo, symlinked into
   `~/.claude/skills/` only after jsmn passes end to end.
   _Reason:_ user requirement ("generic core"); keeps early iterations out of the global setup.
3. **Two stages: Faithful port, then Idiomatic pass.** Final goal is idiomatic Rust with Same behaviour, not the same
   API.
   _Reason:_ user requirement; industry pattern (C2Rust approach, Ship-of-Theseus paper 2607.28835).
4. **Part = a class, or the file holding one.** Not a single function.
   _Reason:_ user requirement ("logical part of the program").
5. **Separate Rust crate per Pilot, no mixed C/C++ + Rust build.** User asked for the cheapest option because the
   interest is C++.
   _Consequence:_ original C++ tests cannot be linked to the Rust crate, so the Judge for C++ is the Differential
   harness plus tests translated into Rust.
   _Revisit:_ before any Goal project.
6. **Judge = original tests (C, Stage 1 only, via C ABI) + Differential harness; translated Rust tests from the
   Idiomatic pass on (and from Stage 1 for C++).**
   _Reason:_ uutils runs the upstream suite and differential fuzzers as black-box oracles.
7. **Resumable state in `migration/` inside each Source project's working copy:** `RULEBOOK.md`, `parts.tsv`,
   `RUN-NOTES.md`; one git commit per accepted Part; "done" = gate passed and commit exists.
   _Reason:_ Anthropic migration kit ("done means the output file exists") and ENCRUST (snapshot/rollback per unit).
8. **c2rust 0.22.1 installed** (`~/.cargo/bin/c2rust`, built against Homebrew LLVM 20.1.8) as an optional Stage 1 fast
   path for C only.
   _Reason:_ user instruction to install it if needed; it is C-only, so the LLM path stays the default.
9. **Budget:** Claude Code subscription, "$20" is a rough time/effort budget that includes planning and skill writing.
   Spend is measured in tokens in `SPEND.md`.
   _Reason:_ user answer; subscriptions have no dollar meter.

## 2026-10-10 — design interview round 4 and follow-ups

10. **Idiomatic pass is accepted when:** zero `unsafe`; `cargo clippy` no warnings; `rustfmt` clean; no
    `unwrap()`/`expect()` on input-driven paths (elsewhere only with an invariant comment); all Judge checks pass.
    _Reason:_ user accepted the recommendation; the C ABI is gone at this stage, so `unsafe` has no justification.
11. **Undefined or platform-specific behaviour:** match the original wherever it is defined. For UB, define the safe
    behaviour in the rulebook and list each one under "Deviations" in `RULEBOOK.md` so the user can veto it.
    _Reason:_ "absolutely the same" is impossible for true UB; user accepted.
12. **Model tiers:** small model for Part bodies and fixes; strongest model for the rulebook, stress test and reviews;
    scripts for everything else. Tokens logged per Pilot in `SPEND.md`.
    _Reason:_ quota is the budget; Anthropic migration guidance.
13. **Build order of the Skill:** scripts first (inventory/dependency map, gate runner, queue runner, spend logger),
    tested on jsmn with c2rust; LLM prompts and rulebook last.
    _Reason:_ scripts are cheapest to test and each removes tokens from later steps.
14. **The Skill ships a built-in idiom table** in `references/` (libc → `std`, C++ → `std`, known traps such as
    `std::map` order vs `HashMap`, unsigned wraparound vs debug-build panics) and the rulebook template loads it. The
    Skill must read and apply it on every run.
    _Reason:_ user requirement ("the skill should know it and use it"). Seed sources: Anthropic RULEBOOK template,
    `rewrite-rs/skills` (BSD-3-Clause). Do **not** copy from `arustydev/ai` (AGPL-3.0); use only as inspiration.
15. **The Skill ships a translation-memory cache:** accepted source/Rust snippet pairs and compile error → fix,
    searchable by text (SQLite FTS5 by default, DuckDB as an option). It is rebuilt from files in `migration/` and git,
    never the source of truth.
    _Reason:_ user requirement ("the skill should know it and use it"); fewer retries. Parts remain resumable from
    `parts.tsv` and git alone (decision 7).

## 2026-10-09 — executor review of PLAN.md (before E0)

16. **Measure spend from session transcripts, not the "tokens left" counter.**
    The counter visible to the model is the context-window budget of one session; it does not include subagents and is
    not the subscription quota. Claude Code writes per-message `usage` (input, output, cache-creation, cache-read
    tokens) into `~/.claude/projects/<project>/<session>.jsonl`, and subagent transcripts sit next to it. `spend.py`
    sums those between two timestamps.
    _Reason:_ measured — this repo's planning session (031b9674) logged 119 messages, 164k output tokens and 16.5M
    cache-read tokens; none of that is visible in the per-session counter.
17. **Gate rollback is path-scoped, never `git stash`.**
    `gate.py` records `HEAD` and restores only the Part's `target_path` (`git checkout <HEAD> -- <path>` plus
    `git clean` of that path) on failure.
    _Reason:_ best practice; `git stash` in a repo shared by all Pilots, the Skill and the docs would also stash
    unrelated work (decision log, `SPEND.md`, notes) and can conflict on pop.
18. **Upstream clones are not committed.** `pilots/*/src/` is git-ignored; the upstream URL and pinned commit go in
    `pilots/<name>/UPSTREAM` so a fresh session can re-clone the exact source.
    _Reason:_ a nested `.git` inside the repo is either a broken submodule or a silent vendor copy; the pin keeps the
    Judge reproducible.
19. **jsmn cost is not extrapolated alone.** jsmn is ~500 lines and splits into 1–2 Parts, so it exercises the queue and
    fan-out barely. The E3 extrapolation must say this; the first usable per-Part cost figure comes from cJSON (E4).
    _Reason:_ a 1–2 sample estimate scaled to ~50k lines (pugixml, jsoncpp) is not meaningful.
20. **Linking original C tests to the Rust crate's C ABI (decision 6) is allowed and does not break decision 5.**
    Decision 5 forbids a mixed *product* build; test-only linking of a Rust `staticlib` into the upstream C test runner
    is part of the Judge.
    _Reason:_ clarifies an apparent contradiction between decisions 5 and 6.
21. **`queue.py` is named `workqueue.py`.** A script called `queue.py` shadows the stdlib `queue` module for anything
    run from or importing the scripts directory (`concurrent.futures`, `multiprocessing`, `logging.handlers` import it).
    _Reason:_ standard Python practice: never name a module after a stdlib module.
22. **Transcript usage is de-duplicated by `message.id`.** Claude Code writes one JSONL line per content block, each
    repeating the full `usage`.
    _Reason:_ measured — session 031b9674 has 119 usage lines but 53 distinct messages; summing lines would overcount
    about 2×.

## 2026-10-10 — E1 results

23. **c2rust is a working Stage 1 fast path for jsmn**; its output is the Faithful-port baseline for E2.
    _Reason:_ measured (E1, `experiments/e1_c2rust_jsmn.py`): output builds; translated upstream tests pass 16/16 in all
    4 macro configs, same as the C original.
24. **Compile-time configuration macros become cargo features in the Faithful port** (jsmn: `strict` for `JSMN_STRICT`,
    `parent-links` for `JSMN_PARENT_LINKS`), and the Judge runs every upstream-tested combination. `inventory.py` must
    report `#ifdef` config macros as a tag (`config-macro`).
    _Reason:_ measured — c2rust translates after preprocessing, so one run fixes one config; upstream tests all four, so
    Same behaviour covers all four.
25. **Generated crates are built with `cargo +<pinned toolchain> --manifest-path`**, never by changing into the crate
    dir.
    _Reason:_ measured — rustup reads `rust-toolchain.toml` only from the cwd; this keeps the "no build tool with a
    pilot dir as cwd" rule and still honours c2rust's nightly pin.

## 2026-10-10 — E2 results (executor)

26. **The jsmn Faithful port is a hand-merge, not raw c2rust output.** `pilots/jsmn/rust/src/lib.rs` keeps the c2rust
    semantics and C ABI (`jsmn_init`, `jsmn_parse`, `#[repr(C)]` structs) but is written readably with
    `#[cfg(feature = "strict")]` / `#[cfg(feature = "parent-links")]` guards, using the diffs of the four c2rust configs
    as the guide (decision 24).
    _Reason:_ c2rust emits four near-duplicate translations (each with `current_block` goto-simulation state); one crate
    with features is what the Judge needs. Verified by the Judge below, not by inspection.
27. **Upstream C tests are linked to the Rust staticlib, not rewired as translated Rust tests.** `tests.c` is compiled
    with `-DJSMN_HEADER` (declarations only) plus the config defines and linked against `libjsmn.a`. This replaces PLAN
    §7 E2(c).
    _Reason:_ simpler and stricter: the real upstream tests run unchanged, no nightly toolchain (`c_variadic`), and it
    is exactly the test-only linking allowed by decision 20. Translated-to-Rust tests remain required from Stage 2
    (decision 6).
28. **A differential corpus must contain every single byte value in each syntactic position.** `make_inputs.py` emits
    bytes 0..255 as primitive, string char and escape char, plus seeded mutations (932 inputs).
    _Reason:_ measured (`experiments/e2_jsmn_judge_sensitivity.py`): a one-character change (`~` rejected) passed the
    upstream tests (16/16) and passed a 164-input hand-picked corpus; only the exhaustive-byte corpus caught it.
29. **Gate baseline is recorded with `gate.py --snapshot` into `migration/gate/<part>.json` (git-ignored).** It stores
    HEAD (for path-scoped rollback) and the checks that already fail. Without a snapshot the baseline is empty and HEAD
    is the current commit.
    _Reason:_ "baseline-aware" needs state from before the change, and the gate is called after it; per-Part files keep
    parallel Parts independent.
30. **Skill scripts are standalone files with no shared module.** Each is run with `python3 -I`, which does not put the
    script directory on `sys.path`; the small TSV reader is duplicated on purpose.
    _Reason:_ keeps the `-I` rule for untrusted directories; avoids a packaging step for the Skill.
31. **`inventory.py` uses clang's JSON AST only for translation units listed in a compile DB**, regex for everything
    else (headers included), and says which in `# engine:`. On jsmn with a DB covering only `tests.c`, the result is
    `clang-ast+regex-fallback`, so header-only libraries get regex symbols.
    _Reason:_ measured on jsmn; the regex engine reproduces line spans and tags on the fixture and on jsmn, but is
    unverified on C++ (templates, nested namespaces). Treat C++ inventory as unverified until E6.

## 2026-10-10 — E3 planning (Opus)

32. **The Stage 2 Judge goes through an adapter and is validated on the Faithful port first.**
    `rust/tests/adapter/mod.rs` is the only file that calls the crate; the translated upstream tests
    (`rust/tests/upstream.rs`) and the Rust driver (`rust/examples/dump.rs`) call only the adapter. The Stage 1 adapter
    (unsafe calls into the C ABI, outside `src/`) must pass `run.py --stage 2` on every feature set and go red on the E2
    mutant before Stage 2 starts; Stage 2 may change only the adapter's body, never its types or signatures.
    _Reason:_ Stage 2 drops the C ABI, so the Stage 1 C test runner and C-built Rust driver can no longer link (decision
    27). Writing the replacement against the API being rewritten would let one agent change code and Judge together; the
    adapter keeps the Judge fixed and the change reviewable in one small diff. Anthropic kit step 0: validate the Judge
    against broken code.
33. **Stage 2 has its own queue, `migration/parts-stage2.tsv`** (same columns; all `todo` at the start).
    _Reason:_ `parts.tsv` has no stage column; resetting it would erase Stage 1 commits and attempts that `verify` and
    `tm.py` rely on. A second file needs no script change in `workqueue.py`.
34. **Deny-rules are loaded with `claude --settings skill/c-to-rust/assets/settings.json` from the repo root**, not
    installed into the Pilot directory; rule proposals go to `RUN-NOTES.md` (`## Rule proposals`) because `RULEBOOK.md`
    is deny-protected during fan-out.
    _Reason:_ Claude Code reads project settings from the session's working directory, and a Pilot directory may not be
    the working directory (untrusted code rule). The kit's rule "the rulebook is read-only inside a loop" is enforced
    mechanically only if no one in the loop can edit it.
35. **jsmn Stage 2 API** (RULEBOOK §6): C `-1` sentinels become `Option<usize>`, `tokens == NULL` becomes
    `Parser::count`, error codes become `enum Error` with `code()`, cargo features stay compile-time, input stays
    `&[u8]` with the NUL stop kept.
    _Reason:_ every choice keeps all values the driver prints recoverable through the adapter, so Same behaviour stays
    checkable; `Option` replaces sentinels per idiom T6/E1.
36. **jsmn Stage 1 is not redone; E3 exercises the LLM Stage 1 path as the stress test instead.** Stage 1 is `done` via
    c2rust + hand-merge (decision 26). The stress test (SKILL.md step 2) translates `jsmn.h` twice with
    `stage1-prompt.md` and the LLM engine into scratch copies of the crate, gates both, and diffs them.
    _Reason:_ the LLM engine is the default and the only one for C++ (decision 8), and nothing has run
    `stage1-prompt.md` yet; jsmn is the cheapest place to find missing rules. The accepted c2rust-based port stays the
    baseline.
37. **Idiom-table rows carry `tags` and `verified` columns.** `tags` (inventory tags, `*` = always) selects the rows a
    prompt receives; `verified` stays `no` until a Pilot test exercises the row (`<pilot>@<commit>` + test name).
    _Reason:_ PLAN §5 (prompt gets only matching rows) and PLAN §10 (rows from general knowledge are unverified).

## 2026-10-10 — E3 governor session

38. **Subagent spawning is allowed through `autoMode.allow` in the user's `~/.claude/settings.json`, not through the
    Skill's deny-rules.** The entry reads: "Spawning Agent subagents (sonnet, opus, haiku) that run the c-to-rust Skill
    steps in /Users/a1/Programming/My_Projects/settlers_to_rust: they only edit files in the repo, run cargo/python3
    -I/make targets and commit to master". Fallback when a spawn is still refused: the session does that step inline and
    says so; it never retries the same spawn or works around the classifier.
    _Reason:_ the user's `defaultMode` is `auto`, and its classifier refused two subagent spawns in the E3 session
    ("Auto-Mode Bypass", "Create Unsafe Agents") while accepting five others. `skill/c-to-rust/assets/settings.json`
    has no `Agent`/`Task` rule, so it is not the cause and cannot fix it. The `--settings` file merges with the global
    one, so a Skill session started with it is still under auto mode. Measured: 2 refusals in 7 spawns (Haiku A
    accepted, Haiku B refused twice, Sonnet B/C/D and Opus x3 accepted); the exact trigger is not known.

## 2026-10-10 — E3 close-out answers (user)

39. **E4 (cJSON) is a go, measured against three allowed interventions and a budget.** The first fresh-session run on
    cJSON may need the user for exactly: (a) starting the deny-ruled session, (b) approving the Deviations table,
    (c) a budget check. Anything else the user had to do is logged in WORK-LOG as a Skill bug and fixed before E5.
    Budget: stop and ask when SPEND.md shows 70% of 700k output tokens for E4 (about twice jsmn's 322k including
    planning), and in any case if the cost is estimated above $15. No dollar meter exists, so the token cap is the proxy
    and `SPEND-MODELS.tsv` (decision 40) is what turns tokens into an estimate.
    _Reason:_ PLAN §8 asks the user after E3; jsmn alone is too small to extrapolate from (decision 19), so E4 is the
    measurement.
40. **`spend.py` also writes per-model usage to `SPEND-MODELS.tsv`** (input, output, cache create, cache read,
    messages per model and logged phase). `SPEND.md` keeps its format.
    _Reason:_ review gap G18: Haiku and Sonnet cost cannot be separated from output tokens alone, and the E4
    extrapolation needs that split.
41. **Pilot order is cJSON → tinyxml2; miniz only if budget remains; json-parser is dropped.** The PLAN's E5 is
    amended accordingly.
    _Reason:_ plan success requires tinyxml2 through Stage 1, and C++ is the largest unknown (the C++ idiom table is
    empty), so it should be tested early.
42. **Vocabulary decisions.** Stages are always written with number and name: Stage 0 Mapping, Stage 1 Faithful port,
    Stage 2 Idiomatic pass. "Judge" is the only word for the acceptance of Same behaviour (not "referee"); the Gate
    is the script that adds the Stage 2 acceptance rules. "Stub" and "Governor" are glossary terms; "orchestrator" is
    replaced by "governor" in the Skill docs.
    _Reason:_ the user wants every stage understandable by name; the E3 sessions showed that "Judge", "gate",
    "adapter" and "orchestrator" were each used for two things.
43. **The Skill is installed by name for this repo** (`.claude/skills/c-to-rust` → `../../skill/c-to-rust`,
    force-added because `.claude` is ignored), and Markdown is kept at 120 columns by
    `experiments/e3_md_reflow.py` (never joins lines, refuses a file if any word changes).
    _Reason:_ a session can now call `/c-to-rust`; the E4 run is meant to start from the Skill, not from a prompt.

44. **The Rulebook and Judge files are protected by a ticket-aware guard, not by static deny-rules.**
    `assets/settings.json` keeps only always-protected paths (upstream source, `skill/**`, `docs/**`, `experiments/**`,
    `PLAN.md`, `CONTEXT.md`, `.claude/**`, `gate-passed.tsv`, history-rewriting git and network commands) and wires
    `scripts/guard.py` as a PreToolUse hook. The guard refuses `RULEBOOK.md`, `parts*.tsv`, `harness/**`,
    `rust/Cargo.toml`, `rust/tests/*.rs` and `rust/examples/**` of a Pilot only while its ticket 04 or 06 is
    `in_progress`. A Judge change during fan-out is: mark 04 todo, commit, mark in_progress. Bash writes are checked
    crudely (a write marker plus a protected path); writing through an interpreter is not detected.
    _Reason:_ the cJSON translation session found that static deny-rules block Mapping and the Judge build, which must
    write exactly those files, and the Skill never said how to switch them off inside one autonomous session. Two
    settings files would need a restart at every stage boundary.

45. **The guard locates the repo from `$CLAUDE_PROJECT_DIR`, and Pilot analysis scripts live in `<ws>/experiments/`.**
    The hook command is `python3 -I "$CLAUDE_PROJECT_DIR/skill/c-to-rust/scripts/guard.py"`; `guard.py` takes its root
    from that variable (else its own location), never from the shell's cwd, and resolves relative paths (also in Bash
    commands) from the cwd it is given. The repo-level `experiments/` stays read-only in Translation sessions.
    _Reason:_ one `cd pilots/cjson/src` in a persistent shell made the relative hook path unresolvable, and a failing
    hook blocked every Bash, Write and Edit call. The user's rule "save analysis scripts under `experiments/`" is met
    per Pilot without letting a Translation session rewrite the Skill's own sensitivity experiments.

46. **Stage 1 Judge is validated against a scratch crate before ticket 04 (judge.md §4a).** For C with c2rust: a raw
    transpile per feature set in `<ws>/experiments/scratch-crate/`; for C without c2rust: a minimal hand-merged
    slice; for C++: a `todo!()` stub that the Judge must reject with a diff. The scratch crate is never the accepted
    target. _Reason:_ the cJSON Translation session (WORK-LOG 28) found that ticket 02 must link the upstream tests to
    a crate that only ticket 04 produces. jsmn escaped this because its Judge was built beside a hand-merged crate.

47. **Harness output directory is `migration/harness/work/`, not `build/`.** `judge.md` §4/§7, jsmn `run.py`, and
    the Pilots' harness `.gitignore` use `work/`; the cJSON harness was switched the same way. _Reason:_ the user's
    global deny rule `Read(./build/**)` (`~/.claude/settings.json`) made Bash commands containing a `build/` path fail
    in the Translation session, so the Judge could not run. A Skill-side rename works on any machine with that rule and
    needs no change to the user's global settings.

48. **Path deny-rules for repo-level folders are anchored with a leading `/`.** `assets/settings.json` now reads
    `Edit(/skill/**)`, `Edit(/docs/**)`, `Edit(/experiments/**)`. _Reason:_ the unanchored `Edit(experiments/**)` also
    matched `pilots/*/experiments/`, so the Translation session could not save its Pilot scripts there, against
    decision 45. A leading `/` means "relative to the project root" in Claude Code permission patterns; this is from
    the documentation and is confirmed only by the Translation session's next restart (WORK-LOG 32).

49. **`tickets.py mark NN done` refuses while a task is `[ ]`; `--tick-all` is the explicit override.** _Reason:_ the
    cJSON session marked tickets 01-03 done with every checkbox open, so the checklist said nothing; a skipped
    criterion is written `- [x] <task> (skipped: <reason>)`.

50. **Every run gets `<ws>/migration/Makefile` (`status`, `tickets`) and a `tickets.py tick` command.** `init` writes
    the Makefile from `assets/migration.Makefile`; `tickets.py makefile` adds it to older runs. `tick NN TEXT` ticks
    the one open task whose text contains TEXT, so the governor ticks a criterion the moment its evidence exists.
    _Reason:_ the user asked for one command per migration folder to see where the run stands, and cJSON tickets
    were marked done with every box open (decision 49); a one-line `tick` makes ticking cheaper than skipping it.

51. **`harness.py` compares inputs in parallel (`--jobs`, default CPU count; `--jobs 1` is the old serial loop).**
    _Reason:_ the corpus step is about 85% of a Judge run and is process spawning (measured, WORK-LOG 36); inputs are
    independent. Rows keep input order, so the report is byte-identical to the serial one (checked on the cJSON
    corpus and by `tests/test_harness.py`). Not done: caching the original side's outputs.

52. **A ticket has Tasks (with how-steps), Acceptance, and a Done brief below `-----------`.** `init` writes `## Tasks`
    (what to do; each task has indented how-steps: the command, the file, the reference section), `## Acceptance`
    (what proves it) and "Blocked by" with names. `mark NN done` needs `--brief TEXT` (at most 8 lines of 120
    columns), written under `-----------`; `show NN` and `mark NN in_progress` print the briefs of the blockers only.
    Old tickets (no `## Tasks`, no brief) still work and show "(no brief yet)". _Reason:_ the user lost track of what
    the LLM was doing; a checkbox title alone told neither of them how. How-steps point to the existing reference
    section instead of repeating it, so SKILL.md and judge.md stay the single source; the brief is a claim for
    orientation, and `parts.tsv`, `gate-passed.tsv` and git stay the resume truth.

53. **Judge section 5a: three input classes the corpus cannot build must be listed as covered or not testable before
    Stage 2 starts.** Allocation failure, NULL members of API-built values, and input without a trailing NUL. Also: the
    gate gets `--allow-lint` when the crate root carries a rulebook-approved `#![allow]`, a mid-fan-out `RULE`/`GAP`
    follows a fixed order (ledger, `mark todo`, RULEBOOK + §12, Judge, red run), and a crash counts as red.
    _Reason:_ the cJSON Stage 2 review found five BLOCKs on paths the 3881-input corpus never reached, and the first
    recorded gate runs showed `allow fail` hidden by the baseline (WORK-LOG 39, `stage2_review_patch.md`).

54. **C++ inventory: vendored test frameworks are skipped, operators get one name, test/example dirs stay in as dead-code
    roots.** `inventory.py` skips `catch.hpp`, `doctest.h` and friends by file name (`VENDORED_FILES`) and names
    `operator bool`, `operator==`, `operator()` as one symbol (`json::operator bool`) instead of the conversion type.
    `depmap.py` matches `json::get` by its bare call name, treats samples/examples as roots next to tests, and never
    lists operators, constructors or destructors as dead (they are called by syntax). Not done: `test/` and
    `samples/` are **not** skipped by default, because they are the callers that make a header-only API live
    (`tier.py` already keeps them out of the Parts). _Reason:_ tinyjson ticket 01 (1564 of 1630 symbols from
    `test/catch.hpp`; 39 of 42 symbols "dead"). Measured on tinyjson with no `--exclude`: 69 symbols, 4 dead-code
    candidates (was 39 of 42).

55. **C++ Pilots get their own branch: `rulebook-template-cpp.md`, `idiom-table-cpp.md`, judge.md §8, `tickets.py init
    --lang cpp`.** The C++ template replaces the sections that assume C (§0 no C ABI, §3, §5 overload naming, §7
    exceptions, §10) and adds a `Reference build` slot (compiler, standard library, flags, optimisation level of the
    driver, upstream-test patches). judge.md §8 says how to pick it and that a measurement from a subagent is
    unverified until run. The base template §8 gets the default rule "match the original as closely as possible; kept
    bug over fixed; state what the original does first". The idiom table has 29 rows (X1-X29) generalised from
    tinyjson's CPP1-CPP36 and is a **seed**: every row is `verified: no` until a test compiles both sides.
    _Reason:_ tinyjson tickets 01 requests 4-7, the user's rule "Rust result must match the C++ origin as closely as
    possible", and the user's approval of requests 4-7 (2026-10-11). Not done: the clang AST front end, `mapping.tsv`
    and the semantic failure packet; they wait for the verification of their claims (WORK-LOG 47).

56. **`ast_facts.py`: for `--lang cpp` without a compile database, clang's JSON AST is the inventory engine; the regex
    pass is the per-file fallback.** `inventory.py` runs it as a subprocess over the library files (tests and
    examples stay regex), and `depmap.py` reads its `calls.tsv` for every function clang parsed instead of matching
    call names. The dump is filtered by namespace (`-ast-dump-filter`) because an unfiltered dump of a unit that
    includes libc++ was 489 MB on tinyjson (measured); a project with no namespace is capped at 150 MB and falls back.
    `inventory.py` also deletes a stale `calls.tsv` before a regex run. Not done: tests and examples through the AST
    (their global functions fall outside the namespace filter), constructor calls as edges, templates.
    _Reason:_ measured on tinyjson (regex 45 symbols and 25 call edges, AST 61 function bodies and 61 call edges,
    35 throws, 24 `new`, 6 `delete`, 13 `json::json` overloads told apart) and tinyxml2 (regex 205 functions, AST
    416). Also five more idiom rows (X30-X34, tinyjson constructs no row covered) and a fix for the cJSON inventory
    naming 97 functions `CJSON_PUBLIC` (a macro wrapping the return type).

57. **`workqueue.py mark <part> done` refuses `attempts 0`; every implementer run is preceded by `mark in_progress`.**
    `--no-attempt` allows it for a Part accepted before the counter existed. _Reason:_ the counter only moves on
    `mark in_progress`, and cJSON Stage 2 ended with attempts 1 (cJSON) and 0 (cJSON_Utils) after three implementer
    runs, so the 5-attempt limit could never fire (WORK-LOG 48). Not done: counting runs in `gate.py` (it cannot tell
    an implementer run from a re-gate of the same attempt).

58. **Attempts are counted from `gate.py --snapshot` rows (`migration/attempts.tsv`), not only from `mark in_progress`.**
    `mark done` sets `attempts` to the larger of the counter and the recorded runs; `verify` flags a done Part whose
    counter is lower. A snapshot means "an implementer run starts" (stage prompts already pair them). The cJSON
    Stage 2 ledger was corrected to 3 and 3 from the three implementer commits (01845a3, e67e701, 3e23a08; the gate
    passed both Parts in each). _Reason:_ decision 57 only catches `attempts 0`; a Part re-run while `in_progress`
    still undercounted unless the governor remembered to re-mark it. Known limit: a snapshot taken without an
    implementer run counts as a run; jsmn Stage 1 stays at 0 (accepted before the counter existed, not derivable).

59. **The ticket script and Makefile are generalised into `skill/to-tickets/` and installed into the mattpocock
    `to-tickets` skill.** `scripts/tickets.py` builds the tickets of a JSON spec (`init --spec`), writes one file per
    ticket, a copy of itself and a Makefile (`status`, `tickets`, `show NUM=`), and keeps the state commands of the
    c-to-rust script (`tick`, `mark` with blocker, open-task and `--brief` checks, `show`). `make install-to-tickets`
    copies it and a SKILL.md section (between markers, idempotent) into the plugin's cache copy. _Reason:_ the
    c-to-rust tickets lost track of work without checkboxes, how-steps and briefs (decisions 49, 52); the user asked
    for that to be part of `to-tickets`. The plugin cache is replaced on a plugin update, so the canonical copy lives
    in this repo and the installer is re-run after an update. Not done: a fork of the c-to-rust script is not merged
    with it (two copies of the render and mark logic; the c-to-rust one keeps its fixed seven-ticket chain).

60. **`gate.py`: a PASS prints `N of M checks passed` and lists the checks the baseline excused; `--strict-judge` makes
    Stage 1 behave like Stage 2 for `test`/`harness`.** Snapshot dirs are created with parents (part id `src/x`),
    rollback skips a path that does not exist at the snapshot HEAD (an untracked `--also` Judge file), and
    `--min-tests` is now passed to the checks (it was parsed and ignored). _Reason:_ tinyjson ticket 04 printed
    `gate: PASS (1 checks passed)` with 5/22 tests red and 5561 corpus diffs because the stub baseline failed both
    Judge checks (WORK-LOG 59). Default Stage 1 still excuses them: a half-translated crate may not compile while
    other Parts are `todo` (decision 29). Not done: refusing `mark done` when a Judge check was excused; the governor
    runs the last Stage 1 gate with `--strict-judge`.

61. **`assets/settings.json` allows `Agent(model:haiku|sonnet|opus)`; the user applied it.** _Reason:_ the auto-mode
    classifier denied implementer subagent launches (tinyjson WORK-LOG 58: "Sonnet launch denied again"); allow rules
    are evaluated before it (Claude Code docs, via the claude-code-guide agent, not yet seen working). The edit was
    refused twice for this session ([Auto-Mode Bypass], [Self-Modification]) and was not worked around. A session must
    start with `--settings skill/c-to-rust/assets/settings.json` to get it. Open: whether the classifier honours it.

62. **`guard.py` refuses only tool calls made by subagents; the main (governor) session may edit everything under
    `pilots/P/` except what the static deny-rules hold.** The hook input's `agent_id` (documented PreToolUse field,
    present when the hook fires inside a subagent) selects the calls to check; without it the call passes. The
    static deny-rules are unchanged (upstream `src/**`, `UPSTREAM`, `gate-passed.tsv`, which only `gate.py` writes, and
    the Skill files). Step 1 of the Stage 1 loop verifies with `guard.py check PATH` (REFUSE), not by an `Edit`.
    _Reason:_ the user's rule (WORK-LOG 65): in a Translation session everything in the Pilot except the original code
    is editable, only Skill files are blocked. The Judge still needs protection from the Haiku implementers, which is
    what `agent_id` buys; the 04/06 `mark todo` ritual now applies to subagents only. Unverified in practice: that
    `agent_id` arrives on real hook calls of Agent-tool subagents (docs only); first fan-out must show an implementer
    `Edit` of `RULEBOOK.md` refused. A subagent that is not launched through the Agent tool (a separate `claude -p`) has
    no `agent_id` and is not protected.

63. **`gate.py` hashes the Rulebook and the Judge at `--snapshot` and fails an attempt that changed them.** Hashed:
    `migration/RULEBOOK.md`, `migration/harness/**` (not `inputs/`), the crate's `Cargo.toml`, `tests/**`, `examples/**`.
    A changed file fails the attempt and rolls back the target only; `--accept-judge-change` passes a governor edit;
    `--also` paths are exempt (they are restored, not judged). _Reason:_ decision 62 stops subagents that carry
    `agent_id`; this is the detective layer that also catches a subagent the hook does not see, and it leaves a
    record in the gate output (tinyjson-translation-4 proposal; the lock cost five `mark todo` cycles). Limit: it
    sees only changes between snapshot and gate; a Judge edit committed before the snapshot is the governor's.

64. **The implementer default is Sonnet 5.5, Haiku only on request; a reported `STATUS: done` is not evidence.**
    SKILL.md Models, stage1-prompt and stage2-prompt say `model: sonnet`; the governor runs `cargo check` before the
    gate. _Reason:_ the user's decision of 2026-10-12 as relayed by tinyjson-translation-4 (not seen directly in this
    session) and its run: Haiku 5.5 gave 30 compile errors at the Stage 1 gate, needed three fix rounds on rules that
    were in the prompt, and reported done on a Stage 2 file with 12 errors; Sonnet's scanners were right first time.
    Supersedes the "default Haiku" wording of commit 8a13f8f; the Haiku 5.5 version choice stays for when Haiku is used.

65. **Policy approval of deviations; `mark done` refuses a gate pass that excused failing checks.** (a) A §8 row that
    reproduces the origin is `approved (policy)`; only rows with no origin answer, a fix instead of a kept bug, or a
    public-API change stop for the user, who reviews the rest afterwards. (b) `gate-passed.tsv` has an `excused`
    column; `workqueue.py mark done` refuses a Part whose latest pass at this stage has one unless `--accept-excused`.
    (c) A denied Agent launch is not repeated verbatim: the governor writes the Part itself or stops. _Reason:_
    tinyjson-translation-4 (relaying the user): fresh Translation sessions must run without the user between steps,
    and Stage 1 and 2 printed PASS with excused fmt/unwrap/allow/test/harness. (a) changes decision 11 ("the user
    approves each row before fan-out") on that relay; the user has not confirmed it in this session. Limit of (b):
    a Part of a half-translated crate legitimately has an excused Judge failure and needs `--accept-excused`.

66. **`scripts/run.py` drives a stage through scripts and `claude -p`; it is dry by default.** Without `--go` it
    prints the plan and spends nothing; it stops (exit 3) for a missing precondition, a `proposed` deviation, a
    `stubbed` Part, 70% of the output-token budget, review GAP/RULE entries or a refused `mark done`. The implementer
    and reviewer are `claude -p --model sonnet --permission-mode acceptEdits --allowed-tools ...` runs (flags checked
    against `claude --help`, not run). _Reason:_ tinyjson-translation-4's request (a fresh session runs without the
    user between mechanical steps). Not done: tag-matched idiom rows, c2rust slot, §6 design, stubs. Limits: tested
    only against a fake `claude`; a headless implementer has no `agent_id`, so decision 63's hashes are its guard. The
    classifier allow rule asked for in the same request is a permission edit and was left to the user.

67. **Who approves deviations is a switch: `run.py --deviations ask|policy`, default `ask`.** `ask` stops on every §8
    row that is not `approved`, including `approved (policy)`; `policy` accepts those rows and the user reviews them
    afterwards. SKILL.md allows policy status only when the user chose it. _Reason:_ decision 65 changed decision 11 on
    a peer's relay; the user asked for a flag to choose (this session). Default is the conservative one.

68. **run.py prints one flushed line per step: `[h:mm:ss] Part i/N [k/N finished, p%]`, attempt, snapshot,
    implementer, gate, review verdict, ACCEPTED.** Each model or script step prints its start and its duration.
    _Reason:_ the user's first real run showed a silent driver; a headless stage runs for hours and needs a visible
    heartbeat. Progress counts Parts of the whole queue, not tokens.
