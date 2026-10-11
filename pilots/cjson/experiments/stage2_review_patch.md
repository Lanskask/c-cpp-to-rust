# Stage 2 review: patch for RULEBOOK.md and the Judge files

Source: `stage2_port_review.md` (Opus review of both Parts, 2026-10-11). The user approved all rule rows
(2026-10-11). Revision 2 (2026-10-11): a second Opus pass checked every item against the pinned C source, the crate,
`gate.py` and the Judge files and changed 1.1, 1.2, 1.4, 1.7, 1.8, 2.1-2.4 and Part 3 below; 1.3, 1.5 and 1.6
stand. Apply with ticket 06 at `todo`: `tickets.py mark 06 todo`, edit, two commits (RULEBOOK, then Judge),
`mark 06 in_progress`. Rows below use the table order of the RULEBOOK: Q rows have three columns
(`id | quirk | where`), D rows have five (`id | original | Rust | Judge case | status`).

## Part 1: RULEBOOK.md (and one line in RUN-NOTES.md)

### 1.1 Section 5, after the line `rust/src/lib.rs only declares the two modules and the crate attributes.`

Correction after the governor's check (2026-10-11): `gate.py` matches `#[allow(..)]` and `#![allow(..)]` alike, by
lint name (`ALLOW` regex, `--allow-lint`). The crate-level `#![allow(non_snake_case)]` in `lib.rs` is therefore fine
for the gate once the gate is called with `--allow-lint non_snake_case`; without that flag the check fails (both
recorded gate runs showed `allow fail lib.rs:2 non_snake_case`, absorbed by the baseline). No code change in `lib.rs`.
The rulebook only has to say that the allow is legal:

```diff
 - Part → target file: `cJSON` → `rust/src/cJSON.rs`, `cJSON_Utils` → `rust/src/cJSON_Utils.rs` (`parts.tsv`);
   `rust/src/lib.rs` only declares the two modules and the crate attributes.
+- Stage 2: `lib.rs` may carry `#![allow(non_snake_case)]` (the module names are the Part file names, so the lint
+  cannot be satisfied without renaming them); the gate is run with `--allow-lint non_snake_case`. No other
+  `#[allow]`/`#![allow]` in `rust/src/`, including a second `non_snake_case` (the gate cannot tell them apart;
+  the review checks it).
```

### 1.2 Section 8, row D2

Original column: change `1175/1188` to `1175/1188/1203` (the writes through the unchecked `malloc`).

Rust column: starts with `OOM aborts (D1)`, then:

```diff
-Stage 2: OOM aborts (D1)
+Stage 2: OOM aborts (D1); an `allocate` hook returning `false` at one of these sites aborts the process at the C
+write site where C writes through NULL; at `1175` the write happens only inside the loops (`1188`, `1203`), so the
+abort happens there. `cJSON_Utils.c:211` (`cJSONUtils_strdup`) is checked in C and is not a D2 site (C returns NULL
+and goes on).
```

Judge column of D2 changes from `no (no Utils test installs failing hooks)` to `partly: the Utils allocator section
checks the sizes and order of the allocations at the D2 sites (counting trace) and the `1175` failure path that never
writes (`GeneratePatches([], [1])`); the abort itself is not testable (C writes through NULL)`.

### 1.3 Section 8, row D20 (original column: append; Rust column: append)

```diff
 `strcmp` on a `NULL` `valuestring` or key in Utils: `compare_json` (`cJSON_Utils.c:626`), `create_patches` (`1164`),
-`generate_merge_patch` (`1427`) dereference NULL for a string item without a value (e.g.
-`cJSON_CreateStringReference(NULL)`) or a keyless object child
+`generate_merge_patch` (`1427`) dereference NULL for a string item without a value (e.g.
+`cJSON_CreateStringReference(NULL)`) or a keyless object child; also `cJSONUtils_FindPointerFromObjectTo` on a keyless
+child of an object (`cJSON_AddItemToArray(object, item)`): `pointer_encoded_length(NULL)` (`cJSON_Utils.c:242-244`)
```

Rust column: add `FindPointerFromObjectTo` encodes a missing key as the empty string (`/` + rest of the pointer).
Code site gets `DEVIATION(port): D20` at `cJSON_Utils.rs:168` (`unwrap_or(&[])`).

### 1.4 Section 8, new row D22 (after D21)

```text
| D22 | A String item whose `valuestring` is NULL (`cJSON_CreateStringReference(NULL)`) as a patch member:
        `op` is dereferenced by `strcmp` (`cJSON_Utils.c:750`), `path` of a non-`test` op by `path->valuestring[0]`
        (`839`), `from` of `move` by `cJSONUtils_strdup(NULL)` (`438`, `71`); C defines `test` with a NULL `path`
        (status 1, `get_item_from_pointer(NULL)` = NULL, `305-308`, `834`) and `copy` with a NULL `from` (status 5,
        `922`, `927`)
      | Stage 1: same calls (UB preserved). Stage 2: keeps the two defined cases and gives every UB case the NULL
        pointer result of C's `get_item_from_pointer`: `op` status 3, `path` status 13 for remove/replace and 9 for
        add/move/copy, `move` with a NULL `from` status 5. A NULL `path` never takes the root branch (`839`);
        `move`/`copy` with a NULL `path` resolve `from` first and then return 9, and the cleanup deletes the value
        (`1027-1030`), so a `move` loses its member, as C does for any unresolvable `path`. No panic
      | no: every D22 case crashes in C and in the Stage 1 crate; the two C-defined neighbours (NULL-path `test`
        returns 1, NULL-from `copy` returns 5) are pinned in translated tests
      | approved 2026-10-11 (user) |
```

Code consequence for the implementer: pass the `valuestring` as an `Option<&[u8]>` to `get_item_from_pointer` and
`detach_path` instead of reading it as `""` (`cJSON_Utils.rs:684-686`, `749-755`); drop the `TODO(port)` there;
mark the UB cases `DEVIATION(port): D22`.

### 1.5 Section 9, new row Q27 (after Q26)

```text
| Q27 | `detach_path` detaches with the case-insensitive `cJSON_DetachItemFromObject` whatever the case flag: under
        `cJSONUtils_ApplyPatchesCaseSensitive`, `remove`/`replace` at `/FOO` and `move` from `/FOO` detach the first
        key equal to `FOO` ignoring ASCII case (e.g. `foo`); the parent lookup, `test`, `copy` and the add step stay
        case-sensitive. Measured 2026-10-11 on the pinned source: `{"foo":1}` + cs `replace /FOO 9` gives
        `{"FOO":9}` status 0; cs `remove /FOO` gives `{}`; cs `move /FOO -> /x` gives `{"x":1}`
      | `cJSON_Utils.c:464-467` (called from `890`, `918`) |
```

Code: `QUIRK(port): Q27` marker at `detach_path_in` (`cJSON_Utils.rs:358-362`).

### 1.6 Section 9, row Q18: append to the quirk column

```text
`sort_list` never repairs the merged list's first `prev`: the first child keeps the `prev` it had (NULL when it
headed a split half, a stale middle node when it headed an already-sorted first half). A later add to that object
(an `add` patch after a `test` that sorted it, `cJSON_AddItemToObject` after `SortObject`) appends nothing (`prev`
NULL, `cJSON.c:2050-2055`) or links after the stale node and drops the members behind it (BUG). Measured
(2026-10-11, pinned C build and Rust, equal): `{"b":1,"a":2}` + `[test "" {"a":2,"b":1}, add /c 3]` gives status 0,
`{"a":2,"b":1}`; `{"a":1,"c":3,"b":2}` + `[test "" {...}, add /x 9]` gives status 0, `{"a":1,"b":2,"x":9}`.
```

### 1.7 Markers and trailers the code needs (implementer, not RULEBOOK)

- `DEVIATION(port): D15` at `print_buffered` (`cJSON.rs:1727`); `DEVIATION(port): D19` at `compare_pointers`
  (`cJSON_Utils.rs:105-128`).
- PORT STATUS trailers: `cJSON.rs:2695` gets `D15` in its deviation list (`...,D14,D15,D17,...`);
  `cJSON_Utils.rs:1277` changes from `todos=1 deviations=D2,D12,D19,D20` to
  `todos=0 deviations=D2,D12,D19,D20,D22` once the `TODO(port)` at `cJSON_Utils.rs:684` is gone.

### 1.8 RUN-NOTES.md and RULEBOOK section 12

RUN-NOTES, under `## Rule proposals`, one line: `Rows 6-11 of stage2_port_review.md applied to RULEBOOK 2026-10-11 by
user approval (Q27, Q18, D22, D20, D2, section 5).`

RULEBOOK section 12 (Amendments) gets a row, because earlier rule changes were recorded there:
`| 2026-10-11 | Opus review (stage2_port_review rows 6-11) | 5, 8, 9 | lib.rs allow; D2 Stage 2 text; D20 site; D22;
Q27; Q18 first prev | approved (user) |`.

Also, section 6.0 item 3 states the `prev` invariant without exception: add `(a Utils sort leaves the first child's
prev stale, Q18)`.

## Part 2: Judge files (separate commit; the gate must go red on the current crate first)

Expected on the current crate: the new tests for BLOCK 1-4 fail and the new Utils allocator section shows DIFFs.
Everything else must still match (166 translated tests plus the harness sections that exist today).

### 2.1 `harness/make_inputs.py`: new `judge` cases

`make_inputs.py` builds `bytes` cases joined with `SEP` inside the `judge` list; add three (patch arrays elided here,
take them from the review):

```python
("judge-q18-sort-add2", b'{"b":1,"a":2}' + SEP + b'[{"op":"test","path":"","value":{"a":2,"b":1}},'
                                                 b'{"op":"add","path":"/c","value":3}]'),
("judge-q18-sort-add3", b'{"a":1,"c":3,"b":2}' + SEP + b'[{"op":"test","path":"","value":{"a":1,"b":2,"c":3}},'
                                                        b'{"op":"add","path":"/x","value":9}]'),
# Q19: array index wrap-around in decode_array_index_from_pointer (2^64 + 1 -> 1, 2^64 -> 0)
("judge-q19-wrap", b'{"foo":[1,2,3]}' + SEP + b'[{"op":"add","path":"/foo/18446744073709551617","value":9},'
                                              b'{"op":"remove","path":"/foo/18446744073709551616"}]'),
```

Append the GetPointer paths `/18446744073709551616` and `/0/18446744073709551617` at the end of the `fixed[]` list
(`dump.c` around 543-545; it runs on every root, not only on `[1,2,3]`) and in the same position in `dump.rs`. The
corpus goes from 3881 to 3884 inputs: update the count in RUN-NOTES.

### 2.2 `harness/dump.c` and `rust/examples/dump.rs`: dump the links after a sort

In `pair_ops` (and the SortObject/SortObjectCaseSensitive probes), after each in-place sort call
`dump_tree(a, depth)` (prints `first_prev_is_last` and `prev_null`, `dump.c:81-101`), so the stale first `prev` of Q18
becomes observable. Same call in both drivers, same label. Six more full-tree dumps on all inputs is an unmeasured
cost: call `dump_tree` after a sort only for the inputs of the subset of 2.3 (it covers both Q18 cases), and time it
together with 2.3.

### 2.3 `harness/dump.c` and `rust/examples/dump.rs`: Utils allocator section

`hooks_pass(a, la)` has no second input and the adapter's `CountingHooks` only counts, so the section is a new
function, not an extension:

- Add `utils_hooks_pass(a, b)`, called from `main` after `pair_ops`, for the input basenames starting with `patch-` or
  `judge-` only (114 inputs today: 77 + 37), sweep capped at `MAXFAIL 48`.
- Duplicate the target before `hooks_on`, so only the Utils call is counted.
- Per pair print the counting trace (`a<size>` / `f` per event) and the fail-the-Nth sweep with `live=` for
  `ApplyPatches` (case-insensitive and case-sensitive), `MergePatch`, `GenerateMergePatch`; for `GeneratePatches` and
  `FindPointerFromObjectTo` (D2 sites) the counting trace only.
- Add a trace switch to `h_malloc`/`h_free` in `dump.c` and to `CountingHooks` in the adapter (a Judge change, same
  commit).
- Fixed probes, run for the single input `judge-utils-oom` so that a Rust abort reds one input and not 3884:
  `FindPointerFromObjectTo` on `{"a":{"b":1}}`, target `b`, `fail_at=1` (the size-1 strdup at `cJSON_Utils.c:211`,
  BLOCK 1); `GeneratePatches([], [1])` with `fail_at=2` (`CreateArray` at `1290` is #1, `new_path`, 22 bytes, is #2;
  print the trace first to confirm `a22` is #2; BLOCK 2).

The Opus scratch differential found C and Rust equal on 6000 pairs (216,944 lines) once BLOCKs 1-2 are fixed; that run
is evidence, not part of the Judge. Time the first Judge run with the new section and record the corpus-step seconds
in `RUN-NOTES.md` (judge.md section 6); the corpus step is about 22 s with `--jobs 8` today.

### 2.4 `rust/tests/upstream.rs`: extra translated tests

- FindPointerFromObjectTo with the size-1 allocation failing returns NULL (BLOCK 1).
- `generate_patches([], [1])` with the path buffer allocation failing returns the single `add /-` patch (BLOCK 2).
- NULL-path `test` returns status 1 and NULL-from `copy` returns status 5; use `create_string_reference(None)` as
  the member (BLOCK 3, D22). Both results are defined in C, so these two tests run in both layouts and are never
  skipped. The D22 UB cases (NULL `op`, NULL `path` of remove/replace/add, NULL `from` of `move`) get no test that
  runs under `C_LAYOUT=true` (C crashes); under `C_LAYOUT=false` they may be pinned with the status values of D22.
- BLOCK 4 cannot go red through `parse`: `do_parse` always appends a NUL (`tests/adapter/mod.rs:150-156`). Add the
  adapter helpers `parse_unterminated(&[u8])` and `parse_with_opts_unterminated(&[u8], bool, bool)` that pass the slice
  without the NUL (in the Stage 1 adapter they append it, the same C semantics). Tests: `parse_unterminated(b"[1")`
  gives the same error as `parse(b"[1")` (2); `{"a"` gives 4; `[` gives 1; `parse_with_opts_unterminated(b"1", true)`
  succeeds with end 1, and `b"[1] "` with end 4 (section 6.0 item 6).
- BLOCK 1 and 2 tests: parse first, then `hooks_on(1)` / `hooks_on(2)` (`hooks_on(n)` and
  `create_string_reference(None)` already exist, adapter `690` and `453`).

New adapter helpers are added to `tests/adapter/mod.rs` without changing existing public signatures.

## Part 3: order (governor)

1. Settle the uncommitted ledger first: `gate-passed.tsv` has Stage 2 rows at `34f7efe` and `parts-stage2.tsv` shows
   cJSON `in_progress 1` and Utils `todo 0`. Commit them as history or revert; "attempt 2 for both Parts" below does
   not match Utils.
2. `tickets.py mark 06 todo`, then the RULEBOOK and RUN-NOTES commit (with the section 12 row).
3. The Judge commit: `make_inputs.py`, `dump.c`, `dump.rs`, adapter helpers, `upstream.rs`.
4. `tickets.py mark 06 in_progress`.
5. Run the Judge for `""`. Expect SIGABRT: the translated-test binary aborts on the BLOCK 1/2 tests (C writes through
   NULL, Rust aborts), so you get a crash, not a list of failures. Confirm the other 166 pass with
   `cargo test -- --skip <block tests>`. BLOCK 3/4 fail normally; only `judge-utils-oom` should DIFF. Time the
   corpus step and record it in RUN-NOTES.
6. Implementer (Haiku or an escalated retry): BLOCK 1-5, the markers and trailers of 1.7, with Q27 and D22 as context.
   NIT 17-21 (including the `rust/Cargo.toml` Stage 1 description and `test-internals` feature, governor-owned) go
   into the brief or into RUN-NOTES as open.
7. Re-gate both Parts in both sets with `--allow-lint non_snake_case --no-rollback` (`--also` paths are relative to
   `pilots/cjson/`, e.g. `rust/src/cJSON_Utils.rs`), then a second review.
