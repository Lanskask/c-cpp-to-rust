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
