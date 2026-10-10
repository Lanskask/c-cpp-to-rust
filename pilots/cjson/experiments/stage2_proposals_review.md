# Review of Stage 2 rule proposals P1-P5 (cJSON)

Date 2026-10-10. Planning review before the Stage 2 (Idiomatic pass) of the cJSON pilot. Inputs: `RULEBOOK.md` (all
sections), `RUN-NOTES.md` ("Stage 2 Judge", "Rule proposals"), `rust/tests/adapter/mod.rs`, `judge.md`,
`stage2-prompt.md`, `src/cJSON.c`, `cJSON.h`, `cJSON_Utils.{h,c}`, `src/tests/misc_tests.c`, `compare_tests.c`,
`harness/dump.c`. Nothing else was changed. The Judge was not run; the only run was the prebuilt original C driver
(`harness/work/stage2-dev/dump_orig`) over the corpus, to check P5.

Binding priority (user): Rust behaviour as close to C as possible; every unavoidable divergence is an explicit,
user-approved Deviation.

## 1. Verdicts

| id | verdict | reason                                                                                                         |
|----|---------|----------------------------------------------------------------------------------------------------------------|
| P1 | modify  | correct but incomplete: misses per-node fields, the `prev` invariant, `Invalid` nodes, parse end on failure    |
| P2 | modify  | depends on D1: if D1 is revised (recommended) the allocator sweep stays compared; else accept as worded        |
| P3 | modify  | list is right for today's adapter, but 4 items are reachable through the public API and 1 has the wrong D id   |
| P4 | modify  | the "or" is already decided (big Judge threads exist); add a release-build check and a non-recursive delete    |
| P5 | modify  | the fact is wrong: `cJSON_Parse(NULL)` leaves the error pointer **unchanged** (stale), not NULL                |

**P1.** Add to the list:

- Per-node fields on every kind: `valueint`, `valuedouble` are stored, not derived (Q5: a parsed `true` has
  `valueint` 1, `cJSON_CreateTrue` 0, `cJSON.c:1394`, `2472-2503`); `valuestring` also on `Raw`; the key `string`
  survives detach (Q16).
- The list invariant: `dump.c:95` prints `first_prev_is_last` and `prev_null` for every node, so the C `prev` rule
  (`cJSON.c:2041-2055`) is compared on all 3881 inputs. A tree without `prev` cannot pass.
- An `Invalid` kind (type word 0) is constructible publicly: `remove` at path `""` overwrites the root with
  `{..., cJSON_Invalid, ...}` (`cJSON_Utils.c:839-849`, Q23). A Rust `enum Kind` without `Invalid` breaks Q23.
- On failure the parse end equals the error offset (`cJSON.c:1209-1212`); the adapter's `Parsed.end` reads it.
- Identity is needed by the library itself, not only by tests: `cJSONUtils_FindPointerFromObjectTo` compares
  `object == target` (`cJSON_Utils.c:208`); `DetachItemViaPointer`/`ReplaceItemViaPointer` take a node.
- Status codes: `apply_patch` produces 0-11 and 13 (`cJSON_Utils.c:815-1036`), never 12; `ApplyPatches` returns 1
  when `patches` is not an array (`1046`). "0..13" is fine as a range.

**P2.** See D1 below. If the user keeps D1, accept P2 verbatim (text in section 3).

**P3.** Corrections, all checked in the C source:

- `cjson_add_item_to_object_should_not_use_after_free_when_string_is_aliased` is an aliasing case (the key argument
  is the item's own `string`, freed at `cJSON.c:2108` after the copy at `2099`), the same class as D7, not D14.
- The `Invalid` node of `typecheck_functions_should_check_type` (`misc_tests.c:155-158`: type `Invalid |
  StringIsConst`) and `cjson_compare_should_compare_invalid_as_not_equal` are reachable publicly: root `remove`
  (Q23) gives type 0, then `cJSON_AddItemToObjectCS` adds bit 512 (`cJSON.c:2095`). Translate them that way in both
  Stages; only `cjson_compare_should_not_accept_invalid_types` (type `Number | String`) stays excluded.
- The null-`prev` guard of `cJSON_DetachItemViaPointer` (`cJSON.c:2260`) is reachable publicly: a new node has
  `prev == NULL` (`memset`, `cJSON.c:243-246`), so detaching it from any array returns NULL. D14 says "the guards
  become unreachable and are removed"; that is false for this site. Keep the guard; add a public-API variant test.
- Two tests are partly guarded but not listed: `cjson_create_string_reference_should_create_a_string_reference`
  (`valuestring == string`, `misc_tests.c:581`) and `cjson_set_valuestring_to_object_should_not_leak_memory`
  (shorter value reuses the buffer, longer gets a new one, `misc_tests.c:711`, `718`; `cJSON.c:455-473`). Safe Rust
  can serve both: `slice.as_ptr() as usize` is safe, a reference stores the caller's `&'static [u8]`, and Stage 2
  copies a longer value into a new `Vec` before dropping the old one, as C does. Move `raw_value_string_addr` and
  `raw_set_valuestring_addr` out of `C_LAYOUT`.
- The circular test leaves the list if D11 is revised (below).

**P4.** Big threads are already the Judge's answer (256 MiB tests, 512 MiB `dump.rs`, RUN-NOTES "Measured"). The
"more than 2 KiB per level" is inferred (2 MiB / 1000 levels), not measured per frame: label it so. Library users,
not the Judge, are the risk: C parses 1000 levels on an 8 MiB main stack; a Rust debug build on a 2 MiB spawned thread
does not. And the 512 MiB driver thread hides any Stage 2 stack regression. Add one release-build check at 2 MiB and
make delete non-recursive (section 2, D10).

**P5.** `cJSON_ParseWithOpts` returns at `cJSON.c:1135-1138` for NULL input, **before** the reset of `global_error`
at `1153-1154`. So `cJSON_Parse(NULL)` and `cJSON_ParseWithOpts(NULL, ...)` keep the previous parse's error pointer;
`cJSON_ParseWithLength(NULL, n)` / `cJSON_ParseWithLengthOpts(NULL, ...)` reset it and then skip the write (`1200`),
giving NULL. Measured with `dump_orig` over the 3881 inputs: the `Parse(NULL)` line printed `err=-1` 2946 times and
`err=outside` 1842 times. `err=-1` appears only when the preceding `ParseWithLength` succeeded. A Stage 2 error that
yields `Unset` for every NULL input fails 1842 harness lines. The rule must be the stale one (Q26 in section 3).

## 2. Deviations touched by the proposals

**D1 (hooks dropped): revise the Stage 2 half.** A failure-injection and accounting seam is feasible with zero
`unsafe`, because the seam need not own memory; it only decides whether a C allocation site succeeds:

```rust
pub trait AllocHooks: Send + Sync {
    /// Called at each C allocation site with C's size; `false` = this allocation fails.
    fn allocate(&self, size: usize) -> bool;
    /// Called at each C free site with the size given to `allocate`.
    fn deallocate(&self, size: usize);
}
pub fn init_hooks(hooks: Option<std::sync::Arc<dyn AllocHooks>>); // process-global, as cJSON.c:186
```

Storage is a `static RwLock<Option<Arc<dyn AllocHooks>>>` (safe, `RwLock::new` is `const`). Memory still comes from
the global allocator. Rules that keep the C observables:

- Sizes are C sizes: node 64 (`sizeof(cJSON)` on LP64, `cJSON.c:243`), `strlen + 1` for strings (`198`), print
  buffers as A3/Q6. Document the `bytes` counter as "C bytes".
- With custom hooks C has no `reallocate` (`cJSON.c:233-237`), so `ensure` growth is `allocate(new)` then
  `deallocate(old)`.
- Delete emits `deallocate` for node, `valuestring` and key, honouring `IsReference`/`StringIsConst`
  (`cJSON.c:259-271`); printed buffers are a `Printed` type whose `Drop` emits `deallocate` (the driver counts the
  `cJSON_free` of print results, `dump.c:438-457`).

Cost: every allocation guard in §7 stays (constructors return `Option`, adds return `bool`, partial results are
dropped), plus one hook call per site; roughly the code the Faithful port already has. Gain: the 13 `cjson_add`
`*_should_fail_on_allocation_failure` tests and the harness allocator sweep (counts per parse/print/duplicate,
`fail-parse[i]` for every allocation, `dump.c:425-468`) stay in the Stage 2 Judge. Residual Deviation: real
out-of-memory aborts in Rust where C returns NULL. `ensure_should_fail_on_failed_realloc` stays ignored in both
Stages (the public `cJSON_Hooks` has only `malloc_fn`/`free_fn`). If the user prefers the approved D1, keep it, but
also name `cJSON_malloc`/`cJSON_free` (`cJSON.c:3197-3205`) as dropped public functions.

**D7 (overlap): keep.** With `&[u8]` input and `&mut` access to the tree, an argument that borrows the node's own
buffer does not compile; the caller must copy first, and then C also succeeds. Every expressible input keeps the C
result (shorter in place, longer new buffer, `cJSON.c:455-473`). Keeping C would need an API "set from own offset"
whose only behaviour is to fail. Extend D7's wording to the aliased-key test (P3).

**D10 (stack): keep, add two duties.** (a) Delete and `Drop` must not recurse: C `cJSON_Delete` recurses without a
limit (`cJSON.c:261`), and a Rust `Box` tree drops recursively, so a deep tree built by adds overflows. An arena
deletes in a loop. (b) The Stage 2 gate builds release once and parses plus prints a `NESTING_LIMIT`-deep document and
duplicates a `CIRCULAR_LIMIT`-deep array on a 2 MiB thread. An explicit-stack parser is not required unless (b)
fails; it would move the code away from the Faithful port for no measured need.

**D11 (cycles): revise if §6 picks an arena.** With node handles, `cJSON_AddItemToArray(b, o)` builds the cycle
exactly as C (only `array == item` is refused, `cJSON.c:2031`), `Duplicate` fails at `CIRCULAR_LIMIT`
(`cJSON.c:2836-2839`) and print fails at `NESTING_LIMIT`. The upstream circular test then runs as written, and the
extra acyclic test stays. Remaining divergence: delete or compare of a live cycle (C: unbounded recursion, UB).
With an owned `enum` tree, D11 stays as approved.

**D14 (corrupted lists): keep, narrowed.** Hand corruption (`item->prev = NULL`, freed `valuestring`,
`misc_tests.c:394-401`; type `Number | String`) cannot be built through a safe API, so these steps stay excluded. But
the guard at `cJSON.c:2260` is reachable (P3) and must stay. A node from another list is accepted by C and unlinked
from that list with exact pointer surgery (`2264-2289`): Stage 2 reproduces the surgery and does not add a
membership check (that would change the result). The guard at `2349-2352` and the `child->prev == NULL` branch of
`add_item_to_array` (`2050-2055`) are unreachable without corruption and may go.

**Representation (input to §6).** Each point above (identity, `prev`, references sharing a child list, `Invalid`,
cycles, non-recursive delete, the D1 seam) is natural in a node store with `Copy` handles, and hard in an owned
`enum` tree. Recommendation: one `Store` owning all nodes (`Vec` slots, generational `NodeId`), the C link fields as
`Option<NodeId>`, the type word as a `u32`-backed flag set. Nodes move between trees of one store; a stale handle gives
`None` where C would be use-after-free.

## 3. Text to paste

RULEBOOK §6, before the remaining TODO:

```markdown
### 6.0 Observability contract (amendment P1, 2026-10-10)

The §6 API, whatever its shape, lets the Judge adapter (`rust/tests/adapter/mod.rs`) report every value below in safe
Rust, with no behaviour of its own beyond handle bookkeeping:

1. Identity: a `Copy + Eq + Hash` handle per node, stable while the node lives, across detach, add and replace;
   equal iff same node (tests; `cJSONUtils_FindPointerFromObjectTo`, `cJSON_Utils.c:208`).
2. Fields of every node of every kind: the type word (kind bits with `Invalid` = 0, plus `IsReference` 256 and
   `StringIsConst` 512, D12); `valueint` and `valuedouble` as stored fields (Q5); `valuestring` (also for `Raw`) and
   the key, each `Option<&[u8]>`.
3. Links `child`, `next`, `prev` with the C invariant: the first child's `prev` is the last child, the last child's
   `next` is none, a new or detached node has `prev` none (`cJSON.c:2027-2058`, `2256-2292`).
4. Parse: end offset on success; on failure the error offset (Q12), also returned as the parse end
   (`cJSON.c:1209-1212`). A thread-local last-error offset follows the C reset rules of Q26 (D9).
5. Patch status codes 0-11 and 13, and 1 for a non-array patch list (Q23), through `PatchError::code()` (§7).
6. String arguments are `&[u8]` read up to the first NUL (C `strlen`); stored strings never contain NUL (Q10). A
   string reference keeps the caller's `&'static [u8]` without a copy; a longer `set_valuestring` value goes into a
   new buffer, a shorter one is copied in place (`cJSON.c:455-473`).
7. The header macros as functions with the macro semantics (M2): `set_int_value` (`cJSON.h:281`, both fields, no
   saturation), `set_number_value`, `set_bool_value` (new type word or 0), children iteration.
8. If D1 is revised: `init_hooks` with `allocate(size) -> bool` / `deallocate(size)` events at every C allocation and
   free site, with C sizes (node 64).
```

RULEBOOK §9, new row:

```markdown
| Q26 | `cJSON_Parse(NULL)` and `cJSON_ParseWithOpts(NULL, ..)` return NULL without touching the error position, which keeps the previous parse's value; `cJSON_ParseWithLength*(NULL, ..)` reset it to none and leave it none | `cJSON.c:1135-1138`, `1153-1154`, `1195-1215` |
```

RULEBOOK §10, replace the "Stage 2:" bullet:

```markdown
- Stage 2: translated tests `rust/tests/<stem>.rs`, Differential harness with `rust/examples/dump.rs`, clippy
  `-D warnings`, rustfmt, zero `unsafe`, no `unwrap()`/`expect()` on input-driven paths (others need an invariant
  comment) (decision 10). The harness compares every section of the dump, including the allocator sweep (D1 seam,
  sizes in C bytes). The release build parses and prints a `NESTING_LIMIT`-deep document and duplicates a
  `CIRCULAR_LIMIT`-deep array on a 2 MiB thread (D10).
- Translated tests excluded in Stage 2 (each prints `SKIPPED:` or is `#[ignore]`):
  - D1: `ensure_should_fail_on_failed_realloc` (no public realloc hook; ignored in Stage 1 too).
  - D7: `cjson_set_valuestring_should_return_null_if_strings_overlap`,
    `cjson_add_item_to_object_should_not_use_after_free_when_string_is_aliased`.
  - D14: `cjson_compare_should_not_accept_invalid_types`; the hand-linked list of
    `cjson_detach_item_via_pointer_should_return_null_if_item_prev_is_null` (a public-API variant runs instead);
    the corrupted-list and freed-valuestring assertions of `cjson_functions_should_not_crash_with_null_pointers`.
  - Only if D1 is kept as approved: the 13 `*_should_fail_on_allocation_failure` tests of `cjson_add`; the C driver
    is then built with `-DDUMP_NO_HOOKS` and nothing covers allocation failure.
  - Only if §6 is not a node store (D11 kept): `cjson_should_not_follow_too_deep_circular_references`, covered by
    `extra_duplicate_should_stop_at_the_circular_limit_without_a_cycle`.
```

The Judge needs one change set before Stage 2 (allowed: it precedes the Stage 2 commit, `judge.md` §5): typecheck
and compare-invalid through root `remove`, the public null-`prev` variant, the two address functions out of
`C_LAYOUT`, and if D1 is revised the adapter's `hooks_*` over `init_hooks`. Rerun Stage 1 green after it.

## 4. Behaviour the adapter contract cannot serve today

1. **Direct field writes.** C callers write `valuedouble`, `valueint`, `type` and `valuestring` (public struct,
   `cJSON.h`). Q3 names one ("a caller who writes `valuedouble` without `valueint` gets the `%g` path"), and
   `cjson_compare_should_compare_raw` wrote `type = cJSON_Raw`. The adapter has no setter, so Q3's second sentence is
   untested in both Stages. Fix: §6 offers `set_valueint`/`set_valuedouble` (memory-safe), or a new Deviation for
   field writes. Writes to `type` and links stay outside (D14).
2. **Writes through returned pointers.** `cJSON_GetStringValue`, `cJSON_SetValuestring` and the print functions
   return mutable `char *`; C callers may edit the value in place. Not in the adapter; Stage 2 will not offer it.
   Needs a Deviation row (proposed D21) for user approval.
3. **`cJSON_malloc`/`cJSON_free`** (`cJSON.c:3197-3205`) are public API outside the adapter; name them in D1 or D21.
4. **Stale error pointer.** Served only if §6 has the thread-local last error (Q26); the adapter must not simulate it.
5. **Stack depth.** The adapter cannot observe stack use, and the 512 MiB driver thread hides regressions (D10 duty).

Next step: put D1 (seam or keep), D11 (node store or not) and the new D21 to the user, because §6 and the Judge
change set both depend on those answers.
