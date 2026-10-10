# C/C++ → Rust translation: research findings

Gathered 2026-10-09 for the "settlers_to_rust" project. Input for planning a translation
skill/pipeline. **No tool has been built yet.** This file is information only.

Verification labels used below:
- **[read]**: I read the primary text this session.
- **[abstract]**: only the arXiv abstract was fetched.
- **[snippet]**: from a search-result snippet or blog; not checked against a primary source.
- **[memory]**: my own prior knowledge, not checked this session.

---

## 0. The most important finding for this project

**Both target projects are C++, not C. Almost everything published is C-only.**

| | s25client (local) | widelands (GitHub) |
|---|---|---|
| Language | C++ (5.4 MB C++ per GitHub; ~155k lines C++ outside `external/`, of which `libs/s25main` is ~115k) | C++ 9.8 MB, plus 4.3 MB Lua (game scripts/data) |
| C++ features measured locally | 124 files with `template<`, 66 headers with `virtual`, 132 files including boost, 143 test `.cpp` files | not measured (not cloned) |
| Licence | see `s25client/LICENSE` | GPL-2.0 |
| Tests | `tests/` exists (143 cpp files; boost.test and turtle mocks appear in `external/`) | not checked |

Consequences:
1. **C2Rust is C-only [memory].** ENCRUST, C2SaferRust, Laertes, Crown and CRustS all start from
   C2Rust output, so none of them can be applied to C++ as-is. LLM4C2Rust claims C/C++ [abstract],
   but I have not checked how it handles classes, templates or inheritance.
2. The benchmarks are tiny next to these projects. ENCRUST's largest is optipng, 106k LoC of C.
   Most benchmark programs are 0.03k–14k LoC. Rustine: 27–13,200 LoC.
3. ORBIT explicitly **excludes multi-threaded and GUI programs** because of non-determinism
   [read]. Both targets are GUI games (SDL/OpenGL), so the verification story must be
   designed separately.
4. No paper I found reports a C++ game or any C++ project of this size translated automatically.
   The nearest evidence is the Anthropic Bun port, which is Zig→Rust (see §3).

→ The tool should probably be built as an **agentic, structure-preserving, module-by-module
rewrite with a mechanical judge**, not as a C2Rust-based pipeline. Section 4 explains why.

---

## 1. The article (downloaded)

Files in `articles/`:
- `2604.04527v1.html`: original HTML
- `2604.04527v1.pdf`: PDF, 8 pages
- `2604.04527v1.txt`: plain text extracted from the HTML (used for this summary)

**ENCRUST: Encapsulated Substitution and Agentic Refinement on a Live Scaffold for Safe C-to-Rust
Translation.** Sim, Cho, Shokri, Fu, Ravindran (Korea Univ. etc.), arXiv 2604.04527, cs.SE,
6 Apr 2026. [read]

### Pipeline
Input: C project → **C2Rust** produces a compilable all-`unsafe` Rust crate (`rust/`).

**Phase 1: Encapsulated Substitution** (per function, deterministic orchestration + LLM)
1. *Preprocess*: decompose into function records; call graph; leaf-first order (callees
   before callers); SCCs broken deterministically by name; C↔Rust matched by name
   (C2Rust keeps names).
2. *Struct translation*: for each C-origin struct with raw-pointer fields, keep the original
   `#[repr(C)]` struct and **append a parallel safe struct** `S^` with `From<&S>` (copy, never
   own: `CStr::to_owned`, `to_vec`, `Box::new(*p)`) and `to_raw()`.
3. *Per-function translation with the **wrapper pattern***: the original function becomes
   (a) a thin `extern "C"` wrapper with the **same name/signature/ABI** whose body is only `let`
   conversions that shadow the parameter names, plus one call, and (b) `f_safe` with safe types,
   which is the only thing the LLM writes. Untranslated callers keep working unchanged.
   Retry ≤5 times with compiler/test errors fed back; on persistent failure restore a file
   snapshot and keep the unsafe body.
4. *TDWE (type-directed wrapper elimination)*: deterministic pass rewrites call sites to call
   `f_safe` directly (6-layer fallback for conversions), gated by compile + tests, with
   rollback. One pre-check: skip functions whose address is cast (`f as *const ()`) because a
   compile+test gate cannot see that UB.

**Phase 2: Agentic Refinement** (one LLM agent per task, 17 tools, max 40 iterations)
- Task types in fixed order: `static_mut` → `wrapper_removal` → `struct_migration` →
  `struct_use_migrate` → `function_translate` → `dead_code`.
- Tools: grep, read_file, read_function, get_function_signatures, list_files; replace,
  regex_replace, replace_function, delete_function, batch_replace; find_static_mut_usages,
  self_reflect (counts remaining unsafe); compile, run_tests; checkpoint, rollback, complete_task.
- `complete_task` triggers a **two-stage gate**: debug build, then release build + full test
  vectors. **Baseline-aware**: a failure is accepted only if the test also failed on the
  Phase 1 output.
- Stall detection: hint after 5 consecutive compile failures; sliding-window detection of
  period-1/2/3 tool-call cycles.
- `static mut` replacement rules, in priority order: `const` → `static` →
  `OnceLock<T>` → `AtomicT` → `OnceLock<Mutex<T>>`. If the variable is reachable from a signal
  handler, only const / static / atomic are allowed (BFS over an approximate call graph).
- "Intermediate results are persisted to disk so that an interrupted run can be resumed
  without repeating completed tasks." That is all the paper says about resumability. The
  pseudocode is in a supplementary file I did not see.

### Two invariants worth copying
- **Live Scaffold Invariant**: at every step the whole crate compiles and passes the whole test
  suite. Failure of any step is rolled back, never left in place.
- **Copy, Never Own** (Invariant 2): safe wrappers must copy data out of C-owned memory, never
  take ownership of it. This is the coexistence rule for mixed safe/unsafe code.

### Results (authors' own numbers, 1 model, not independently reproduced)
- 15 programs: 7 GNU Coreutils (65k LoC, 1,210 functions) + 8 Laertes libs (133k LoC,
  1,156 functions); 197,706 LoC, 2,366 functions in total.
- GPT-4o, temperature 0.
- Unsafe LoC reduced by **37.8% vs C2Rust** (Phase 1 alone 34.3%, Phase 2 adds 5.4%).
  This is modest; the code is still ~60% as unsafe as C2Rust output.
- 100% test-vector correctness on all 15. But tests cover only ~314/1,210 Coreutils functions
  and 631/1,156 Laertes functions, so "correct" means "correct on the covered paths".
- Function compile-pass rate 94–99% on Coreutils (C2SaferRust 54–75%).
- Phase 2 completed 790 of 1,133 tasks (69.7%), 21 iterations/task on average.
- TDWE accepted 79.9% of wrapper pairs.
- Baseline EvoC2Rust: produced non-compiling output on all 7 Coreutils (authors' run).

### Limitations stated in the paper
- Correctness only on tested paths. Laertes libs had no tests; the authors built harnesses
  and generated 1,000 inputs per library with AFL++, keeping only inputs where the C binary was
  deterministic.
- Functions whose wrappers TDWE removes are never re-examined by Phase 2, so unsafe code can
  survive inside them.
- Inline asm and `setjmp/longjmp` are out of scope.
- Output is still C-shaped. Idiomatic Rust redesign is not attempted.

---

## 2. What ENCRUST cites (references, grouped)

Full reference list is on the last page of `articles/2604.04527v1.txt`. [read]

**Rule-based / static transpilers (all C only, all start from C2Rust except Corrode)**
| Tool | Idea |
|---|---|
| C2Rust (Galois/Immunant, 2018) | syntactic rewrite; every pointer → raw pointer; output compiles, all unsafe |
| Laertes (Emre et al., OOPSLA 2021) | uses rustc as an oracle; iteratively retypes pointers and fixes errors |
| Crown (Zhang et al., CAV 2023) | SAT-based ownership analysis; pointer → `Box`/reference |
| CRustS (Ling et al., ICSE-C 2022) | 220 TXL source-to-source rules (198 semantics-preserving) |

**LLM, function level**
| Tool | Idea |
|---|---|
| VERT (Yang 2024, arXiv 2404.18852) | C→WASM→rWasm oracle + LLM candidate; property tests + bounded model checking |
| FLOURINE (Eniser 2024, 2405.11514) | differential fuzzing as the equivalence check |
| Syzygy (Shetty 2024, 2412.14234) | translate code and tests together, using dynamic analysis |
| SmartC2Rust (Shiraishi 2024, 2409.10506) | segment to fit context; separate repair signals for compile / semantic / unsafe |
| SACTOR (Zhou 2025, 2503.12511) | unidiomatic then idiomatic step; static-analysis hints; FFI-based mixed C/Rust testing |
| SafeTrans (Farrukh 2025, 2505.10708) | compile-error then few-shot repair |
| TymCrat (Hong & Ryu 2025) | type migration of signatures, many-to-many C↔Rust types |

**LLM, project level**
| Tool | Idea |
|---|---|
| RustMap (Cai 2025, 2503.17741) | dependency + dynamic call-graph ordering; repair from compile and test feedback |
| LLMigrate (Liu 2025, 2503.23791) | Tree-sitter per-function split to fight LLM "laziness"; reassemble in topological order |
| EvoC2Rust (Wang 2025, 2508.04295) | module decomposition → compilable stub skeleton → fill stubs → cascading repair |
| Rustine (Dehghan 2025, 2511.20617) | refactors C first (preprocessor, pointers); two-tier LLM (escalate to a reasoning model on failure); no C2Rust |
| C2SaferRust (Nitin 2025, TSE) | C2Rust + LLM per slice with dataflow/call-graph context; the direct baseline |
| IRENE (Luo 2025, ICSME) | rule-augmented example retrieval + semantic summary + error-driven refinement |

**Supporting**: AFL++ (fuzzing for test inputs); the memory-safety motivation papers (SoK:
Eternal War in Memory, Cling, Intel MPX study); RustBelt; the Rust language paper.

### Papers NOT cited by ENCRUST that I found and that matter more for us
| Paper | Why relevant |
|---|---|
| **ORBIT** (Farrukh et al., 2604.12048) [abstract + partial read] | Coding-agent based, not C2Rust based. Builds a dependency-aware translation graph, generates Rust interfaces, then maps C functions → Rust stubs and verifies. Reports 100% compile / 91.7% test success on 24 CRUST-Bench programs (small). Excludes GUI/multithreaded. |
| **RustPrint** (2605.14634) [abstract] | Converts the repo to architecture documentation first, uses it as a migration blueprint, then compares docs of the Rust output against docs of the C source as a repair signal. 8 repos, 11K–84K LoC. Closest to the project scale we need. |
| **Ship-of-Theseus** (2607.28835, iodine 12.5k SLOC) [abstract] | Semantics-preserving non-idiomatic baseline first, then incremental agentic rewrite into idiomatic Rust, each step gated by compile + behaviour tests. |
| **PtrTrans** (2510.10956) [partly read] | Pointer knowledge graph (ownership, mutability, nullability, lifetime). Fallback: on repair failure, revert and emit a stub with `unimplemented!()` so the project stays compilable. 16 small projects. |
| **C2RustXW** (2603.28686) [abstract] | Multi-level program structure (symbols, dependencies, control/data flow) injected into prompts; rule-based plus LLM repair. |
| **&inator** (2604.17261) [abstract] | Constraint-based *interface* (struct/signature) translation that is correct and borrow-check aware. Scaling to large programs is listed as future work. |
| **LLM4C2Rust** (2604.15485) [abstract] | RAG + small LM; says C **and C++**. Unchecked. |
| **CRUST-Bench** [snippet] | Benchmark of C repos with expert-written Rust interfaces; used by ORBIT. |

---

## 3. Completed C→Rust (and other→Rust) rewrites: what approaches were used

### 3a. Human-led, long-running rewrites [snippet unless noted]
| Project | What | Approach |
|---|---|---|
| **librsvg** (GNOME) | C → Rust, 2016–2020 | **Incremental, module by module inside the C project**, exposing the same C ABI; Rust parts called from C via FFI, replaced piece by piece until C was gone. Federico Mena's GUADEC talks/blog are the main source [snippet]. |
| **sudo-rs** | sudo reimplementation | Clean-room rewrite of a *subset* of features with tests and security audits, not translation. Default in Ubuntu 25.10 / 26.04 LTS [snippet]. |
| **ntpd-rs** | NTP | Clean-room Rust reimplementation, staged deployment (Let's Encrypt staging then production), Ubuntu proposal [snippet]. |
| **rustls**, **zlib-rs**, **rav1d** | TLS, zlib, AV1 decoder | rustls: new implementation. zlib-rs and rav1d: ported against the original test suites / bit-exact output; rav1d started from C2Rust output then hand-refactored [memory, not verified this session]. |
| **Rust for Linux** | kernel drivers | New code in Rust, with C↔Rust bindings (bindgen); not a translation [memory]. |

Pattern: **clean-room rewrite of a subset** or **incremental FFI replacement behind a stable
ABI**, always with a pre-existing test suite or reference implementation as the judge.

### 3b. AI-driven large rewrites
| Project | Source → target | Approach | Source |
|---|---|---|---|
| **Bun** | Zig → Rust, ~1M lines in <2 weeks, 100% existing tests passing before merge, 19 regressions after merge, all fixed | 576-line porting rulebook (`PORTING.md`) plus a one-sentence kickoff; orchestrator script, parallel fixer agents, adversarial reviewers, one whole-workspace compiler pass whose error list becomes a queue | Anthropic blog [read via fetched page] |
| Python → TypeScript, 165k lines over a weekend | hundreds of agents, 8 phase gates, 3 adversarial review rounds, final parity check diffing every command's output against the Python original | same blog |
| Cursor FastRender | Rust browser engine from scratch, ~1M lines, hundreds of agents for a week | coordination patterns and review problems documented in the Cursor blog | awesome list [snippet] |
| Cloudflare vinext | Next.js API on Vite | AI-written; token cost and quality gates described | awesome list [snippet] |
| Turso / Rustwright / etc. | SQLite / Playwright in Rust | parity-test suites; Rustwright itself says full parity is not proven | awesome list [snippet] |

Curated list worth skimming for more: `github.com/malisper/awesome-ai-rust-rewrites`.
Commercial tools listed there: ShiftCodex, VLTR (C/C++ → Rust). I did not evaluate either.

### 3c. Anthropic's migration method (the most reusable process I found)
`github.com/anthropics/code-migration-kit-with-claude-code` [read README only; prompts and
scripts not read yet]. Contains: `CLAUDE.md`, `RUN-NOTES.md`, `prompts/00-feasibility`,
`00b-judge-setup`, `01-dependency-map`, `02-…`, `03-stress-test`, `04-translation-kickoff`,
`05-survey-build`, `06-post-parity`; `templates/RULEBOOK.md`, `templates/inventory.tsv`,
`templates/settings.json` (deny rules), `scripts/depmap_*` (C and Python and TS fixtures
included!), `scripts/queue_runner.mjs`, `skill/` (an installable Claude Code skill),
`examples/run1-toml/`.

Six steps:
0. **Judge first.** "No judge, no exit condition." Test suite must run against both old and
   new code; validate the judge against deliberately broken code.
1. **Map and rules**: deterministic dependency map (works for C/C++), gap inventory
   (`inventory.tsv`: what the target language demands), rulebook (every decision that two agents
   could answer differently is decided once).
2. **Stress-test the rules**: translate a few nasty files twice, diff, and keep only rule
   changes.
3. **Translate everything**: per unit, an implementer + two adversarial reviewers + a fixer,
   fanned out over a mechanical queue. The compiler is not run yet.
4. **Compile**: one survey build; the error list becomes a machine queue sliced by module;
   fixers work *without* compiler access (settings deny it).
5. **Run**: hello world, then smoke tests.
6. **Match behaviour**: shard the inherited test suite, burn failures down by root-cause
   category; adversarial review.

Doctrine quotes worth keeping:
- "You don't fix the code, you fix the process that produced the code."
- "Make review adversarial and verification mechanical. Let scripts (a compiler, a diff, a test
  suite) be the referee."
- "Make the work queue mechanical and resumable. **Done should mean 'the output file exists on
  disk.'**"
- Do not use the largest model for everything: smaller models for fan-out, the largest for
  reviewers and for anything that writes rules other agents follow.
- Front-load human hours on the rulebook and the stress test.

### 3d. vibe-fork (a ready-made Claude Code skill for this kind of job) [README read, repo not inspected]
`github.com/dollspace-gay/vibe-fork`: "sequential translation of a known-working upstream
codebase to a new substrate". Installs a **4-agent loop, two PreToolUse hooks, a route table,
and a binding `goal.md` contract**. Aim: stop the agent from reading upstream once, hand-waving,
and writing plausible code that silently diverges on edge cases. Worth reading as a design
reference for hooks that force "read upstream → write → verify → commit".

---

## 4. Approaches compared, and what that means for a C++ game

| Family | Strengths | Fit for s25client/widelands |
|---|---|---|
| C2Rust → LLM safening (ENCRUST, C2SaferRust, Laertes, Crown) | Compiles and passes tests at every step; the output is always in a working state | **Poor.** C++ not supported by C2Rust [memory]; output stays C-shaped and unsafe (−38% unsafe LoC); no design change possible |
| Skeleton-first (EvoC2Rust, ORBIT, PtrTrans) | Types/interfaces fixed first, then function bodies filled; stubs `unimplemented!()` keep it compilable | **Good**, and the skeleton idea maps to C++ classes → structs/traits. Needs a C++-aware extractor (libclang) |
| Doc/blueprint-first (RustPrint) | Preserves architecture; good at 11–84k LoC | **Good** for module-level design decisions (inheritance → traits/enums, ownership) |
| Rulebook + queue + adversarial review (Anthropic kit, Bun) | Proven at 1M LoC; resumable by construction; language-agnostic | **Best fit** as the outer pipeline |
| Incremental FFI replacement (librsvg) | Always shippable, real-world proven | **Possible** for libs (`libs/common`, `libutil`, `libsiedler2`) via `cxx`/`autocxx` [memory]; harder for the game loop |

### C++ → Rust gap inventory I'd expect (not verified; from general knowledge)
These need rulebook decisions, not per-function LLM guesses:
- class hierarchies with `virtual` → traits/enums; multiple inheritance
- templates (124 files in s25client) → generics vs. monomorphised copies
- boost usage (132 files): signals, serialization, filesystem, etc. need per-library Rust
  equivalents
- RAII/destructors → `Drop`; raw `new/delete`; `std::shared_ptr` cycles → `Rc/Weak` or arenas
- exceptions → `Result`/`panic`
- operator overloading, implicit conversions, preprocessor macros, `#ifdef` platform splits
- global state and singletons (game code typically has many) → the same `static mut` problem
  ENCRUST's Phase 2 tackles
- SDL/OpenGL/audio bindings; Lua scripting (kaguya in s25client; Lua in widelands) → `mlua`
- data files (`.lst`, `.bbm`, savegames): binary format compatibility is a hard correctness
  target and is also a good *oracle*

### Oracles available for a game (to design, not done)
- Existing unit tests (s25client: 143 test cpp files).
- Deterministic replay: settlers-style games are lockstep-deterministic, so *replay files /
  savegames + state hash per game tick* could serve as a differential test between C++ and Rust
  builds. **Unverified assumption**; check whether s25client/widelands have replay/desync-check
  hashes (RTTR has desync detection, widelands has a "game checksum"/sync mechanism [memory]).
- Binary file readers/writers: byte-for-byte round-trip tests against the C++ output.

---

## 5. Requirements for "safe + resumable" (from the sources)

Safe:
1. **Judge before translation** (Anthropic); compile + all tests gate every change (ENCRUST).
2. **Snapshot → attempt → verify → commit or restore**, per unit (ENCRUST, PtrTrans).
3. **Baseline-aware gate**: record which tests fail before changes; reject only regressions
   (ENCRUST). Needed because C++ projects usually have a few failing/flaky tests.
4. **Stubs with real signatures + `unimplemented!()`** keep the crate compiling while work is
   incomplete (PtrTrans, EvoC2Rust, ORBIT).
5. **Copy, never own** at the boundary between old and new memory (ENCRUST), only relevant
   if we run mixed C++/Rust.
6. **Adversarial reviewers** are separate agents; scripts referee (Anthropic).
7. **Forbid shortcuts mechanically**: deny-rules in `.claude/settings.json`
   (e.g. no editing tests, no `#[allow(...)]` blanket, no `unsafe` without `// SAFETY:`),
   and verify they're installed before fan-out (the kit found this step silently skipped once).
8. **Stall detection** for loops (ENCRUST: repeated tool-call cycles, N consecutive compile
   failures → hint / roll back).

Resumable:
1. **On-disk work queue with one row per unit** (`inventory.tsv`-style): id, source path,
   target path, state (`todo/in_progress/done/failed`), attempts, last error, git commit.
2. **"Done = output file exists and gate passed"** (Anthropic); derive state from the
   filesystem + git, not from chat memory.
3. **Commit per accepted unit**; resume = `git log` + queue file + rulebook.
4. **Idempotent orchestrator script** that reads the queue and runs the next unit; any
   Claude session can start it cold. Context loss then only costs the in-flight unit.
5. A `RUN-NOTES.md` / `DECISIONS.md` with rulebook amendments so new sessions share the same
   policy (matches the user's own decision-log convention).

---

## 6. Open questions for the planner (Opus)

1. **Source target**: s25client or widelands? s25client is 5.4 MB C++ with a clear lib split
   (`libs/common` 2.8k lines, `libutil`, `libsiedler2`, `libGamedata` 1.8k, `driver` 0.9k,
   `s25main` 115k). Widelands is ~2× larger plus a Lua layer. The lib split in s25client
   gives natural early milestones.
2. **Faithful port or redesign?** Anthropic's kit is for *structure-preserving* ports.
   Idiomatic Rust (ECS, traits instead of virtual) is a redesign, and the rulebook becomes a
   design document. This decision changes the whole pipeline.
3. **Mixed C++/Rust build during migration (FFI) or big-bang?** The papers use mixed builds;
   Bun-style uses big-bang with an existing test suite. A mixed build needs `cxx`/`bindgen`
   plus CMake integration.
4. **Which judge?** Existing tests, replay hashes, or file-format round trips; needs an
   investigation in the actual repos.
5. **Cost.** ENCRUST: ~2,366 functions with up to 5 retries each. Bun: ~1M lines with
   hundreds of agents. No token cost figures were found for C++-scale projects. Under the
   user's CLAUDE.md, anything that spends money needs a go-ahead before running.
6. **Licences**: both projects are GPL; a Rust port is a derivative and must stay GPL.
7. **Tooling not installed locally**: `c2rust`, `bear`, `cloc`, `tokei`, `ast-grep`.
   Present: `cargo`, `rustc`, `clang`. libclang-based extraction would need LLVM headers
   (not checked).

## 6b. Community Claude skills for C/C++ → Rust (analysed 2026-10-10)

Read: file lists, headings and excerpts only, not every line.

| | `arustydev/ai` → `content/skills/convert-c-rust` | `rewrite-rs/skills` → `port-to-rust`, `port-from-c`, `port-from-cpp` |
|---|---|---|
| Licence | **AGPL-3.0**. Do not copy into our skill | **BSD-3-Clause**. May reuse with attribution |
| Shape | One 1,823-line `SKILL.md` that extends a `meta-convert-dev` skill | One skill per source language (their ADR 0003) plus a process skill; docs in `docs/porting/` |
| Content | Reference tables: primitive/pointer/struct/enum/function-pointer mapping; malloc → ownership; error codes/errno → `Result`; `goto cleanup` → RAII; pthreads; Unity/CMocka tests → Rust tests; macros; pitfalls; c2rust usage | **Process**: parity contract, 5 phases (inventory+seam → characterize with executable tests against the source → port leaf-first → run both and compare → cut over), end states A/B/C, anti-patterns. **C**: ownership/lifetime/nullability/length/encoding traps, bindgen/cbindgen boundary. **C++**: UB, move semantics, implicit conversions, iterator invalidation, exceptions; RAII/`unique_ptr`/`shared_ptr`/templates/MI mappings |
| Scripts, state, resume | None found | None found (docs and agent yaml only) |
| Fit for us | Mine for idea checklists; Unity→Rust tests is relevant to cJSON | Strong conceptual match for the Judge ("characterize" + "run both" = differential harness) and for the C++ construct table |

Verdict: **content sources, not infrastructure.** Neither provides a work queue, gating, rollback or spend tracking; those still come from the Anthropic kit and ENCRUST.

## 7. Things I did NOT do / not verified
- Did not read the Anthropic kit's prompts, scripts, `CLAUDE.md`, or the `skill/` folder
  (only the README). **Read these first when planning**: they are the closest thing to a
  ready-made skeleton for the requested skill.
- Did not read vibe-fork's repo, only its README.
- Did not read the full ORBIT, Rustine, PtrTrans or C2RustXW papers; only abstracts and a few
  indexed passages (they are indexed in the context-mode knowledge base for this session only).
- Did not clone widelands or measure its C++ feature usage.
- Did not check whether C2Rust has any C++ support beyond what I remember (it targets C only).
- Did not verify the sudo-rs / ntpd-rs / zlib-rs / rav1d details beyond search snippets.
- ENCRUST's tables were only partially read (Coreutils totals; Laertes and ablation tables
  skimmed). Numbers above are the authors' own and unreproduced.
