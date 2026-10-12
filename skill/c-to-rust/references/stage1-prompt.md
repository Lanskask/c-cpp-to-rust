# Stage 1 prompt (Faithful port, one Part)

The governor (Sonnet) fills the `{...}` slots and sends the result to the implementer (Haiku; a retry
after 2 failures may escalate to Sonnet). The slots are the **only** context the implementer gets (PLAN §5): no other
files, no chat history. Fill each slot from the named source:

| Slot                                   | Source                                                                                                     |
|----------------------------------------|------------------------------------------------------------------------------------------------------------|
| `{part_id}`, `{target_path}`, `{tier}` | `workqueue.py next` row                                                                                    |
| `{source}`                             | the Part's `source_paths`, full text, with line numbers                                                    |
| `{callee_signatures}`                  | signatures only (no bodies) of functions the Part calls, from `edges.tsv` + already translated Rust files  |
| `{structs}`                            | Rust definitions of structs/enums/consts the Part uses that other Parts already translated                 |
| `{rulebook}`                           | `RULEBOOK.md` §0, §2, §3, §4, §5, §7, §8, §9, §11                                                          |
| `{idiom_rows}`                         | rows of `idiom-table-c.md` whose `tags` column matches the Part's inventory tags, plus all `*` rows        |
| `{c2rust}`                             | c2rust output for this Part per configuration, or `none` (LLM engine)                                      |
| `{previous}`                           | empty on attempt 1; else the last gate line (`gate: FAIL ...`, grouped rustc codes) and the attempt number |
| `{tm_hits}`                            | `tm.py search "<previous error>"`, top 3, or `none`                                                        |

---

```text
You are porting one Part of a C project to Rust. Stage 1: Faithful port.

Part: {part_id}   tier: {tier}   write exactly one file: {target_path}

GOAL
Produce Rust that behaves exactly like the C source for every input, in every cargo feature
configuration listed in rulebook §3. Keep the C structure: same functions, same names, same control
flow, same order of side effects, same values left in out-parameters and structs on every error path.
Translate; do not improve. Defective-looking behaviour is reproduced and marked BUG(port) or
QUIRK(port) as the rulebook says.

HARD RULES
1. Public C symbols keep the C ABI: #[no_mangle] pub unsafe extern "C" fn, #[repr(C)] types,
   constants with the C values. Private helpers may be plain unsafe fn.
2. Each C configuration macro maps as rulebook §3 says (cargo feature or nothing). Use
   #[cfg(feature = "...")] at the same places the C code has #ifdef.
3. Map every construct by the idiom rows and rulebook §2 below, Stage 1 column. A construct with no row:
   most conservative mapping + "TODO(port): <question>".
4. Unsigned arithmetic that can wrap uses wrapping_*; signed arithmetic follows rulebook §8.
5. Write only {target_path}. Tests, the harness, the rulebook, parts.tsv and the upstream source are
   read-only for you.
6. Allowed check: cargo check --manifest-path <crate>/Cargo.toml [--features <set>] for each feature set
   in rulebook §3. Finish when it is clean for all of them; acceptance is decided by the gate.
7. End the file with the PORT STATUS trailer from rulebook §11.
8. The Stage 1 column keeps the C shape even where a Rust rewrite looks equivalent. A goto to a
   shared label: one flag (or labelled block) and one copy of the label's code, never a copy per
   jump. A C index loop: same index variable and type, same exit-value tests, later loops resume
   from the same index; no .rev() ranges or added flags.
9. A helper your functions need that is not in the stub (a libc function the rulebook maps, a C++ `std::` call
   such as `stoll`/`stod`, rulebook I8/I9) is yours: add it to {target_path} as a private fn and implement it
   fully. A `todo!()` body is only for a function the stub already lists and your instructions leave out.

C2RUST OUTPUT (if not "none": start from it; merge the per-configuration variants into one file with
#[cfg(feature)] guards; keep its semantics, replace current_block state machines only where the
replacement is obviously equivalent):
{c2rust}

RULEBOOK
{rulebook}

IDIOM ROWS
{idiom_rows}

ALREADY TRANSLATED: TYPES
{structs}

CALLEES: SIGNATURES ONLY
{callee_signatures}

SOURCE
{source}

PREVIOUS ATTEMPT
{previous}

SIMILAR PAST FIXES
{tm_hits}

When done, reply with three lines only:
STATUS: done | blocked
TODOS: <count of TODO(port) markers>
NOTE: <one line: what was hard, or the open question if blocked>
```

---

## Governor loop for one Part (not sent to the implementer)

1. `workqueue.py mark <part> in_progress`; `gate.py <part> --snapshot ...` (records HEAD + baseline).
2. Send the prompt. The implementer writes `{target_path}`.
3. `gate.py <part> --stage 1 ...`. Pass → `git commit` the target only (`E<n> <pilot>: <part> stage 1`),
   then `workqueue.py mark <part> done --commit <sha>`. Fail → the gate has already restored the target;
   `workqueue.py mark <part> failed --error "<gate line>"`, refill `{previous}`/`{tm_hits}`, retry.
4. After 5 attempts the Part is `stubbed`: write real signatures with `todo!()` bodies so the crate
   compiles, commit, and add a `## <part>` section to `RUN-NOTES.md` with the last error.
5. Two failures with the same rustc code → escalate the next attempt to Sonnet and look for a missing
   rulebook rule (propose it in `RUN-NOTES.md` under `## Rule proposals`).
