# Decision log

Each entry: decision, reason, who/what it came from. Newest last.

## 2026-10-09/10 — design interview rounds 1–3

1. **Pilots before the Goal project.** Order: jsmn, cJSON, json-parser, miniz (C), then C++: tinyxml2, pugixml, jsoncpp.
   s25client and widelands are set aside until the skill passes Pilots.
   _Reason:_ user requirement (cheap proof first); the $20 budget cannot cover a 155k-line C++ port.
2. **Generic Skill core plus a per-project adapter.** Developed in `skill/c-to-rust/` in this repo, symlinked into `~/.claude/skills/` only after jsmn passes end to end.
   _Reason:_ user requirement ("generic core"); keeps early iterations out of the global setup.
3. **Two stages: Faithful port, then Idiomatic pass.** Final goal is idiomatic Rust with Same behaviour, not the same API.
   _Reason:_ user requirement; industry pattern (C2Rust approach, Ship-of-Theseus paper 2607.28835).
4. **Part = a class, or the file holding one.** Not a single function.
   _Reason:_ user requirement ("logical part of the program").
5. **Separate Rust crate per Pilot, no mixed C/C++ + Rust build.** User asked for the cheapest option because the interest is C++.
   _Consequence:_ original C++ tests cannot be linked to the Rust crate, so the Judge for C++ is the Differential harness plus tests translated into Rust.
   _Revisit:_ before any Goal project.
6. **Judge = original tests (C, Stage 1 only, via C ABI) + Differential harness; translated Rust tests from the Idiomatic pass on (and from Stage 1 for C++).**
   _Reason:_ uutils runs the upstream suite and differential fuzzers as black-box oracles.
7. **Resumable state in `migration/` inside each Source project's working copy:** `RULEBOOK.md`, `parts.tsv`, `RUN-NOTES.md`; one git commit per accepted Part; "done" = gate passed and commit exists.
   _Reason:_ Anthropic migration kit ("done means the output file exists") and ENCRUST (snapshot/rollback per unit).
8. **c2rust 0.22.1 installed** (`~/.cargo/bin/c2rust`, built against Homebrew LLVM 20.1.8) as an optional Stage 1 fast path for C only.
   _Reason:_ user instruction to install it if needed; it is C-only, so the LLM path stays the default.
9. **Budget:** Claude Code subscription, "$20" is a rough time/effort budget that includes planning and skill writing. Spend is measured in tokens in `SPEND.md`.
   _Reason:_ user answer; subscriptions have no dollar meter.

## 2026-10-10 — design interview round 4 and follow-ups

10. **Idiomatic pass is accepted when:** zero `unsafe`; `cargo clippy` no warnings; `rustfmt` clean; no `unwrap()`/`expect()` on input-driven paths (elsewhere only with an invariant comment); all Judge checks pass.
    _Reason:_ user accepted the recommendation; the C ABI is gone at this stage, so `unsafe` has no justification.
11. **Undefined or platform-specific behaviour:** match the original wherever it is defined. For UB, define the safe behaviour in the rulebook and list each one under "Deviations" in `RULEBOOK.md` so the user can veto it.
    _Reason:_ "absolutely the same" is impossible for true UB; user accepted.
12. **Model tiers:** small model for Part bodies and fixes; strongest model for the rulebook, stress test and reviews; scripts for everything else. Tokens logged per Pilot in `SPEND.md`.
    _Reason:_ quota is the budget; Anthropic migration guidance.
13. **Build order of the Skill:** scripts first (inventory/dependency map, gate runner, queue runner, spend logger), tested on jsmn with c2rust; LLM prompts and rulebook last.
    _Reason:_ scripts are cheapest to test and each removes tokens from later steps.
14. **The Skill ships a built-in idiom table** in `references/` (libc → `std`, C++ → `std`, known traps such as `std::map` order vs `HashMap`, unsigned wraparound vs debug-build panics) and the rulebook template loads it. The Skill must read and apply it on every run.
    _Reason:_ user requirement ("the skill should know it and use it"). Seed sources: Anthropic RULEBOOK template, `rewrite-rs/skills` (BSD-3-Clause). Do **not** copy from `arustydev/ai` (AGPL-3.0); use only as inspiration.
15. **The Skill ships a translation-memory cache:** accepted source/Rust snippet pairs and compile error → fix, searchable by text (SQLite FTS5 by default, DuckDB as an option). It is rebuilt from files in `migration/` and git, never the source of truth.
    _Reason:_ user requirement ("the skill should know it and use it"); fewer retries. Parts remain resumable from `parts.tsv` and git alone (decision 7).

## 2026-10-09 — executor review of PLAN.md (before E0)

16. **Measure spend from session transcripts, not the "tokens left" counter.**
    The counter visible to the model is the context-window budget of one session; it does not include subagents and is not the subscription quota. Claude Code writes per-message `usage` (input, output, cache-creation, cache-read tokens) into `~/.claude/projects/<project>/<session>.jsonl`, and subagent transcripts sit next to it. `spend.py` sums those between two timestamps.
    _Reason:_ measured — this repo's planning session (031b9674) logged 119 messages, 164k output tokens and 16.5M cache-read tokens; none of that is visible in the per-session counter.
17. **Gate rollback is path-scoped, never `git stash`.**
    `gate.py` records `HEAD` and restores only the Part's `target_path` (`git checkout <HEAD> -- <path>` plus `git clean` of that path) on failure.
    _Reason:_ best practice; `git stash` in a repo shared by all Pilots, the Skill and the docs would also stash unrelated work (decision log, `SPEND.md`, notes) and can conflict on pop.
18. **Upstream clones are not committed.** `pilots/*/src/` is git-ignored; the upstream URL and pinned commit go in `pilots/<name>/UPSTREAM` so a fresh session can re-clone the exact source.
    _Reason:_ a nested `.git` inside the repo is either a broken submodule or a silent vendor copy; the pin keeps the Judge reproducible.
19. **jsmn cost is not extrapolated alone.** jsmn is ~500 lines and splits into 1–2 Parts, so it exercises the queue and fan-out barely. The E3 extrapolation must say this; the first usable per-Part cost figure comes from cJSON (E4).
    _Reason:_ a 1–2 sample estimate scaled to ~50k lines (pugixml, jsoncpp) is not meaningful.
20. **Linking original C tests to the Rust crate's C ABI (decision 6) is allowed and does not break decision 5.** Decision 5 forbids a mixed *product* build; test-only linking of a Rust `staticlib` into the upstream C test runner is part of the Judge.
    _Reason:_ clarifies an apparent contradiction between decisions 5 and 6.
21. **`queue.py` is named `workqueue.py`.** A script called `queue.py` shadows the stdlib `queue` module for anything run from or importing the scripts directory (`concurrent.futures`, `multiprocessing`, `logging.handlers` import it).
    _Reason:_ standard Python practice: never name a module after a stdlib module.
22. **Transcript usage is de-duplicated by `message.id`.** Claude Code writes one JSONL line per content block, each repeating the full `usage`.
    _Reason:_ measured — session 031b9674 has 119 usage lines but 53 distinct messages; summing lines would overcount about 2×.

## 2026-10-10 — E1 results

23. **c2rust is a working Stage 1 fast path for jsmn**; its output is the Faithful-port baseline for E2.
    _Reason:_ measured (E1, `experiments/e1_c2rust_jsmn.py`): output builds; translated upstream tests pass 16/16 in all 4 macro configs, same as the C original.
24. **Compile-time configuration macros become cargo features in the Faithful port** (jsmn: `strict` for `JSMN_STRICT`, `parent-links` for `JSMN_PARENT_LINKS`), and the Judge runs every upstream-tested combination. `inventory.py` must report `#ifdef` config macros as a tag (`config-macro`).
    _Reason:_ measured — c2rust translates after preprocessing, so one run fixes one config; upstream tests all four, so Same behaviour covers all four.
25. **Generated crates are built with `cargo +<pinned toolchain> --manifest-path`**, never by changing into the crate dir.
    _Reason:_ measured — rustup reads `rust-toolchain.toml` only from the cwd; this keeps the "no build tool with a pilot dir as cwd" rule and still honours c2rust's nightly pin.

## 2026-10-10 — E2 results (executor)

26. **The jsmn Faithful port is a hand-merge, not raw c2rust output.** `pilots/jsmn/rust/src/lib.rs` keeps the c2rust semantics and C ABI (`jsmn_init`, `jsmn_parse`, `#[repr(C)]` structs) but is written readably with `#[cfg(feature = "strict")]` / `#[cfg(feature = "parent-links")]` guards, using the diffs of the four c2rust configs as the guide (decision 24).
    _Reason:_ c2rust emits four near-duplicate translations (each with `current_block` goto-simulation state); one crate with features is what the Judge needs. Verified by the Judge below, not by inspection.
27. **Upstream C tests are linked to the Rust staticlib, not rewired as translated Rust tests.** `tests.c` is compiled with `-DJSMN_HEADER` (declarations only) plus the config defines and linked against `libjsmn.a`. This replaces PLAN §7 E2(c).
    _Reason:_ simpler and stricter: the real upstream tests run unchanged, no nightly toolchain (`c_variadic`), and it is exactly the test-only linking allowed by decision 20. Translated-to-Rust tests remain required from Stage 2 (decision 6).
28. **A differential corpus must contain every single byte value in each syntactic position.** `make_inputs.py` emits bytes 0..255 as primitive, string char and escape char, plus seeded mutations (932 inputs).
    _Reason:_ measured (`experiments/e2_jsmn_judge_sensitivity.py`): a one-character change (`~` rejected) passed the upstream tests (16/16) and passed a 164-input hand-picked corpus; only the exhaustive-byte corpus caught it.
29. **Gate baseline is recorded with `gate.py --snapshot` into `migration/gate/<part>.json` (git-ignored).** It stores HEAD (for path-scoped rollback) and the checks that already fail. Without a snapshot the baseline is empty and HEAD is the current commit.
    _Reason:_ "baseline-aware" needs state from before the change, and the gate is called after it; per-Part files keep parallel Parts independent.
30. **Skill scripts are standalone files with no shared module.** Each is run with `python3 -I`, which does not put the script directory on `sys.path`; the small TSV reader is duplicated on purpose.
    _Reason:_ keeps the `-I` rule for untrusted directories; avoids a packaging step for the Skill.
31. **`inventory.py` uses clang's JSON AST only for translation units listed in a compile DB**, regex for everything else (headers included), and says which in `# engine:`. On jsmn with a DB covering only `tests.c`, the result is `clang-ast+regex-fallback`, so header-only libraries get regex symbols.
    _Reason:_ measured on jsmn; the regex engine reproduces line spans and tags on the fixture and on jsmn, but is unverified on C++ (templates, nested namespaces). Treat C++ inventory as unverified until E6.
