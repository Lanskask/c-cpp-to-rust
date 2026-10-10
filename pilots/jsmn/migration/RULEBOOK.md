# RULEBOOK: jsmn (https://github.com/zserge/jsmn @ 25647e6)

Status: deviations-approved (2026-10-10, user approved D1-D3; the RP14 amendment of the D2 Rust column confirmed by the
user 2026-10-10, §8); stress-tested (2026-10-10): translations A-D, rules RP1-RP13 applied (§12, RUN-NOTES "Stress test
closed"); Stage 2 review rules RP14-RP18 applied (2026-10-10, §12).
Idiom table: `skill/c-to-rust/references/idiom-table-c.md` (rows apply unless §2
overrides them).
Template: `skill/c-to-rust/references/rulebook-template.md`.

## 0. Scope and posture

- One Part: `jsmn` (`jsmn.h` → `rust/src/lib.rs`). Tests and examples are not
  Parts.
- Stage 1 (Faithful port) is **done**: c2rust 0.22.1 output for the four
  configurations, merged by hand (decisions 23, 26), commit `f8e1244`, gate
  passes 12 checks. Any Stage 1 re-run (stress test) keeps the
  C ABI: `#[no_mangle] pub unsafe extern "C" fn jsmn_init/jsmn_parse`,
  `#[repr(C)]` `jsmntok_t` and
  `jsmn_parser`, constants `JSMN_*` with the C values. Exact Stage 1 items
  (match `rust/src/lib.rs`; the adapter depends on them):
    - `pub type jsmntype_t = c_uint;` plus
      `pub const JSMN_UNDEFINED/OBJECT/ARRAY/STRING/PRIMITIVE: jsmntype_t`.
      No Rust `enum` in Stage 1 (§2 T8).
    - `pub type jsmnerr = c_int;` plus
      `pub const JSMN_ERROR_NOMEM/INVAL/PART: jsmnerr`
      (−1, −2, −3). `jsmn.h` declares the named type `enum jsmnerr`, so Stage 1
      emits the alias, as the Faithful port and c2rust do; the constants are
      typed
      `jsmnerr`, not bare `c_int`.
    - `jsmntok_t` fields:
      `type_0: jsmntype_t, start: c_int, end: c_int, size: c_int`,
      `#[cfg(feature = "parent-links")] parent: c_int`. The field is `type_0`
      (c2rust spelling), not `type_` or `r#type`.
    - `pub unsafe extern "C" fn jsmn_parse(parser: *mut jsmn_parser, js: *const c_char, len: size_t, tokens: *mut jsmntok_t, num_tokens: c_uint) -> c_int`
      and `pub unsafe extern "C" fn jsmn_init(parser: *mut jsmn_parser)`. Every
      parameter has the C type of `jsmn.h`: `num_tokens` is `c_uint`, not `usize`.
    - `jsmn_parser` fields: `pos: c_uint, toknext: c_uint, toksuper: c_int`
      (`jsmn.h`: `unsigned int`, `unsigned int`, `int`). Not `c_int` or `usize`
      for `pos`/`toknext`.
    - Input bytes are read as `u8` (`*js.add(pos as usize) as u8`), as the
      Faithful port does; this is D3.
    - Arithmetic, per site (idiom I1, prompt rule 4):
        - `wrapping_*` on every `+`, `-`, `++`, `--` of `pos` or `toknext` and
          on
          every expression built from them (`pos + 1`, `toknext - 1`): C
          `unsigned int` arithmetic wraps by definition, and `toknext - 1` with
          `toknext == 0` is reached by the Judge (`:` or a closing bracket
          before
          any token; C gives `(int)(0u - 1) == -1`). Write
          `(*parser).toknext.wrapping_sub(1) as c_int`.
        - Plain operators on the `int` values `count`, `size`, `start + 1` and
          the
          loop indices `i`: C signed overflow is UB and needs ≥ 2³¹ bytes or
          tokens (D1, not reachable by the Judge). Plain operators match the
          Faithful port and c2rust; do not write `wrapping_*` there.
        - `unsigned int` → `int` assignments (`start = pos`, `end = pos + 1`,
          `toksuper = toknext - 1`) are `as c_int` casts.
- Stage 2 (Idiomatic pass): the API in §6, zero `unsafe`, acceptance §10. Same
  behaviour on every
  harness input and every translated upstream test, in all four feature sets.
- Inner loop:
  `cargo check --manifest-path pilots/jsmn/rust/Cargo.toml [--features ...]`.
  Acceptance:
  `make pilot-jsmn STAGE=<n>` only.

## 1. Dependencies and ecosystem

| Area          | Decision                              | Why                                                                                            |
|---------------|---------------------------------------|------------------------------------------------------------------------------------------------|
| Crates        | none (no `libc`, not even in Stage 1) | jsmn uses no libc calls                                                                        |
| Bytes vs text | input is `&[u8]`; never `&str`        | jsmn accepts any bytes, including invalid UTF-8 and NUL                                        |
| Toolchain     | stable; no `rust-toolchain.toml`      | the merged crate needs no nightly feature (decision 27)                                        |
| `no_std`      | not required                          | jsmn is freestanding, but nothing in the Judge needs `no_std`; `std` keeps `std::error::Error` |

## 2. Overrides of the idiom table

| Idiom row | Override for jsmn                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Reason                                                                                                                                                                                       |
|-----------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| T4        | Input stays `&[u8]`, and a NUL byte ends the input even inside the slice (Q1)                                                                                                                                                                                                                                                                                                                                                                                                                                                           | defined behaviour of the original                                                                                                                                                            |
| T5        | `tokens` + `num_tokens` is not a pointer + length pair: `num_tokens` limits allocation only (Q9). Stage 2 takes the whole caller array as `&mut [Token]` and `num_tokens` as a separate `usize` (§6 `parse_with_limit`); reads of allocated tokens are bounded by the slice length, never by `num_tokens`                                                                                                                                                                                                                               | C reads `tokens[toksuper]`, `tokens[toknext - 1]` and the backward scans without checking `num_tokens`; mapping `num_tokens` to the slice length turns defined C results into `Inval` (RP14) |
| T6        | `tokens == NULL` (count-only mode) becomes the separate method `Parser::count`; `count`, `parse` and `parse_with_limit` share one private implementation taking `tokens: Option<&mut [Token]>` and `num_tokens: usize` (`count` passes `None, 0`: C ignores `num_tokens` when `tokens == NULL`)                                                                                                                                                                                                                                         | the mode is API behaviour; one implementation keeps the modes from drifting                                                                                                                  |
| T8        | Stage 1: the idiom-table Stage 1 column (`type jsmntype_t = c_uint` + `const`s), never a Rust `enum` in a `#[repr(C)]` field. Stage 2 only: `TokenKind` keeps the C discriminants 0, 1, 2, 4, 8 (`#[repr(i32)]`)                                                                                                                                                                                                                                                                                                                        | the dump prints `type=%d`; in Stage 1 C code writes the field, and a value outside 0, 1, 2, 4, 8 in a Rust enum is UB                                                                        |
| C1        | `goto found` in `jsmn_parse_primitive`: labelled block or early exit in Stage 2 (Stage 1 keeps the `found` flag)                                                                                                                                                                                                                                                                                                                                                                                                                        | idiom C1                                                                                                                                                                                     |
| C4        | In `jsmn_parse` a C `break` leaves the `switch`, then the `for` increment `pos++` runs. Stage 1 keeps that shape: one `pos` increment, at the tail of the `while` body; an arm whose C code `break`s (the `{`/`[` and `}`/`]` arms with `tokens == NULL`) reaches the end of its `match` arm (put the rest of the arm in `if !tokens.is_null() { .. }`). No `continue`, no `pos` increment inside an arm, no Rust `break` for a C `break`. The Faithful port's `{`/`[` arm (`pos += 1; continue`) behaves the same but is not the model | matches the C control flow (prompt: "keep the C structure"); a Rust `break` inside `match` leaves the loop; per-arm increments drift between arms                                            |
| I1        | Stage 2: positions are plain `usize`; `pos - 1` after a primitive cannot underflow (`pos > start` there); write that invariant as a comment. `toksuper = toknext - 1` (`:` arm) is `toknext.checked_sub(1)` (idiom I10): `None` at `toknext == 0`, as C's `(int)(0u - 1) == -1`                                                                                                                                                                                                                                                         | the Faithful port's `wrapping_*` is only needed for C's `unsigned int`                                                                                                                       |
| E1        | `enum Error { NoMem, Inval, Part }`, `code()` = −1, −2, −3                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | dump and tests print / compare the codes                                                                                                                                                     |

**C4, `#ifdef` around `case` labels (Stage 1).** `jsmn.h` puts
`#ifdef JSMN_STRICT`
around case labels in two places: the strict labels `-`, `0`-`9`, `t`, `f`, `n`
vs the
non-strict `default:` (sharing one primitive tail), plus a strict-only
`default: return JSMN_ERROR_INVAL`, in `jsmn_parse`; and
`#ifndef JSMN_STRICT case ':'`
in `jsmn_parse_primitive`. Each C label group under an `#ifdef` becomes its own
`match` arm with the same `#[cfg]` (prompt HARD RULE 2), in C order. A body that
the
C labels share is written once per cfg'd arm: every feature set compiles exactly
one copy, as C does. Do not merge the arms into one `_` arm with `matches!`/`if`
for the strict labels, and do not use a `cfg!(..)` match guard: both turn a
compile-time label set into a runtime test. HARD RULE 8 ("one copy") is about
goto labels, not about this. The Faithful port has this shape.

```rust
#[cfg(feature = "strict")]
b'-' | b'0'..=b'9' | b't' | b'f' | b'n' => {
/* strict key check, then the primitive tail: parse, r < 0, count, size */
}
// QUIRK(port): Q5
#[cfg(not(feature = "strict"))]
_ => { /* the same primitive tail */ }
#[cfg(feature = "strict")]
_ => return JSMN_ERROR_INVAL,
```

In `jsmn_parse_primitive` the `:` label is its own arm, first, as in C:
`// QUIRK(port): Q5`, then
`#[cfg(not(feature = "strict"))] b':' => { found = true; break; }`,
then the `b'\t' | .. | b'}'` arm with the same body.

## 3. Configuration macros → cargo features (decision 24)

| Macro               | Kind                                | Cargo feature  | Judge runs it | Notes                                                        |
|---------------------|-------------------------------------|----------------|---------------|--------------------------------------------------------------|
| `JSMN_STRICT`       | behaviour                           | `strict`       | yes           |                                                              |
| `JSMN_PARENT_LINKS` | behaviour + layout (`parent` field) | `parent-links` | yes           |                                                              |
| `JSMN_STATIC`       | linkage                             | —              | no            | idiom M4: Rust visibility                                    |
| `JSMN_HEADER`       | packaging (declarations only)       | —              | no            | used only by the Judge to link C tests against the staticlib |

Feature matrix for the gate: `""`, `strict`, `parent-links`,
`strict,parent-links` (all four are upstream-tested).
Features stay compile-time in Stage 2 (no runtime flags): the Judge and the
upstream tests are per-configuration.

## 4. Escape hatch and markers

As in the template §4. Stage 1: `unsafe` only for pointer dereference in the
C-ABI functions. Stage 1 accesses fields through the raw pointer
(`(*parser).pos`) and does not create a `&mut` from a raw pointer
(`let p = &mut *parser;`) that lives while the same pointer is passed to
another function: two live `&mut`/`*mut` paths to one struct is aliasing UB.
Stage 2:
none, anywhere under `rust/src/`. Markers used here: `QUIRK(port): Q<n>`,
`DEVIATION(port): D<n>`.

Stage 1 markers are fixed: exactly these, each as one comment line directly
above the site, and no other `QUIRK`/`DEVIATION` comments.

| marker                | sites                                                                                                                                                                                                                                                      |
|-----------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `QUIRK(port): Q1`     | each loop whose condition tests the NUL byte: `jsmn_parse` main loop, `jsmn_parse_primitive` loop, `jsmn_parse_string` loop, `\u` hex loop (4)                                                                                                             |
| `QUIRK(port): Q2`     | the field resets in `jsmn_alloc_token`; the strict `{`/`[` `return JSMN_ERROR_INVAL` (2)                                                                                                                                                                   |
| `QUIRK(port): Q3`     | each `return JSMN_ERROR_NOMEM`: `{`/`[` arm, `jsmn_parse_string`, `jsmn_parse_primitive` (3)                                                                                                                                                               |
| `QUIRK(port): Q4`     | each count-mode (`tokens == NULL`) test: `}`/`]` arm, string closing quote, primitive `found` tail (3)                                                                                                                                                     |
| `QUIRK(port): Q5`     | the `#[cfg(not(feature = "strict"))] _ =>` arm of `jsmn_parse`; the `#[cfg(not(feature = "strict"))] b':' =>` arm of `jsmn_parse_primitive`; the marker line goes directly above the `#[cfg]` line; the strict arms carry no Q5 (§2 C4 `#ifdef` shape) (2) |
| `QUIRK(port): Q6`     | the strict `return JSMN_ERROR_PART` after the primitive loop (1)                                                                                                                                                                                           |
| `QUIRK(port): Q7`     | `count` initialised from `toknext` (1)                                                                                                                                                                                                                     |
| `QUIRK(port): Q8`     | the `}`/`]` arm; the `,` arm (2)                                                                                                                                                                                                                           |
| `DEVIATION(port): D3` | the primitive `c < 32 \|\| c >= 127` check; the `\u` hex check (2)                                                                                                                                                                                         |

Stage 2 markers: every row of the Stage 1 table applies with the same count, at the same C site in the Stage 2 methods
(counted per source line, whatever the feature set), plus the rows below. No other `QUIRK`/`DEVIATION` comments. D1
has no marker: it is carried by the `usize` types of the `Parser` and `Token` fields and of the returned count, not by
a site. A plain `tokens[i]` whose `i` was just returned by `alloc_token` or checked by a `get` in the same arm carries
an invariant comment, not a marker (idiom E6).

| marker                | sites                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
|-----------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `QUIRK(port): Q9`     | the `toknext >= num_tokens` guard in `alloc_token` (1)                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `DEVIATION(port): D2` | each `get`/`get_mut` whose index comes from parser state and so can pass `tokens.len()` on a misused reused parser: `tokens.get_mut(toknext)` in `alloc_token` after the Q9 guard; `tokens[toksuper]` in the `{`/`[` arm, the `"` arm, the `,` arm, the strict primitive key check, the strict primitive size update and the non-strict `_` primitive size update; the `parent` walk of the parent-links `}`/`]` arm; `tokens[..toknext]` in the default `}`/`]` arm, the default `,` arm and the final open-token check (11) |

## 5. Naming and paths

- Target: `rust/src/lib.rs` (one-Part crate).
- Stage 1 keeps C names. Stage 2 names are the ones in §6.
- Judge adapter: `rust/tests/adapter/mod.rs` (see
  `skill/c-to-rust/references/judge.md`). Translated
  upstream tests: `rust/tests/upstream.rs`. Rust-side driver:
  `rust/examples/dump.rs`, which includes the
  adapter with `#[path = "../tests/adapter/mod.rs"] mod adapter;`.
- Only the adapter knows the crate's API. Tests and the driver never call the
  crate directly.

## 6. Stage 2 public API

```rust
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TokenKind { #[default] Undefined = 0, Object = 1, Array = 2, String = 4, Primitive = 8 }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: Option<usize>,   // C: -1 = None
    pub end: Option<usize>,     // C: -1 = None
    pub size: usize,
    #[cfg(feature = "parent-links")]
    pub parent: Option<usize>,  // C: -1 = None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { NoMem, Inval, Part }          // + Display, std::error::Error
impl Error { pub fn code(self) -> i32 }        // -1, -2, -3

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Parser {
    pos: usize,
    toknext: usize,
    toksuper: Option<usize>
}
impl Parser {
    pub fn new() -> Self;                                                       // jsmn_init
    pub fn parse_with_limit(&mut self, js: &[u8], tokens: &mut [Token], num_tokens: usize)
        -> Result<usize, Error>;                                                // jsmn_parse
    pub fn parse(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<usize, Error>; // num_tokens = tokens.len()
    pub fn count(&mut self, js: &[u8]) -> Result<usize, Error>;                // tokens == NULL
    pub fn pos(&self) -> usize;
    pub fn toknext(&self) -> usize;
    pub fn toksuper(&self) -> Option<usize>;                                    // C: -1 = None
}
```

- `len` is `js.len()`. `tokens` is the caller's whole token array; `num_tokens` is C's `num_tokens`, an allocation
  limit only (Q9, §2 T5). `parse(js, t)` is `parse_with_limit(js, t, t.len())`, the common case.
- `alloc_token` fails (`None`, so the caller returns `Err(Error::NoMem)` with C's Q3 state) when
  `toknext >= num_tokens` (the C guard, Q9) or when `toknext >= tokens.len()` (D2: C would write past the array).
- Every other token access (`toksuper`, `toknext - 1`, the backward scans, `parent` links, the final open-token check)
  indexes `tokens` and is bounded by `tokens.len()` only, never by `num_tokens`; an index `>= tokens.len()` is D2 and
  returns `Err(Error::Inval)`. A reused parser called with `num_tokens < toknext` therefore behaves as C: brackets
  close, sizes grow, only a new allocation fails.
- Private shape: `fn run(&mut self, js: &[u8], tokens: Option<&mut [Token]>, num_tokens: usize)`; `alloc_token`,
  `parse_string` and `parse_primitive` take the same `num_tokens`. `count` calls `run(js, None, 0)`: C ignores
  `num_tokens` when `tokens == NULL`.
- Adapter (`rust/tests/adapter/mod.rs`): `P::parse(js, Some(n))` calls
  `self.parser.parse_with_limit(js, &mut self.buf, n)`, the whole persistent buffer plus the limit, as `dump.c` passes
  its whole `tok` array with `num_tokens = n`. Never `&mut self.buf[..n]`. Adapter signatures do not change
  (decision 32).
- Parser state after an `Err` is exactly the C state (E4): callers resume after
  `Part` with a longer slice.
- `Token::default()` is `Undefined, None, None, 0`. A C token that was never
  allocated is never
  observable (the dump prints only `[0, toknext)`), so the C zero fill
  (`start = 0`) needs no mirror.

## 7. Error model

- `JSMN_ERROR_NOMEM` is a **precondition-guard** (the caller's token slice is
  full), not allocation
  failure: keep it with the same parser state.
- jsmn allocates nothing, so there are no allocation-guards.
- After `Err(_)`, `pos`, `toknext`, `toksuper` and the tokens written so far
  match C exactly (Q3).

## 8. Deviations (decision 11)

| id | Original behaviour                                                                                                                                                                                                                                                                                          | Rust behaviour                                                                                                                                                                                                                                                                                                                                                    | Reachable by Judge?                                        | Status                                                                                    |
|----|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------|-------------------------------------------------------------------------------------------|
| D1 | Input ≥ 2³¹ bytes or ≥ 2³¹ tokens: `int` positions/counts overflow (signed overflow is UB; `(int)pos` is implementation-defined); `pos` is `unsigned int` and wraps at 2³²                                                                                                                                  | `usize` positions and counts, no overflow                                                                                                                                                                                                                                                                                                                         | no (harness reads ≤ 1 MiB)                                 | approved                                                                                  |
| D2 | API misuse: `num_tokens` larger than the real array, `js == NULL`, `len` past the buffer, a parser reused with a token array shorter than `toknext`/`toksuper` or a `parent` link: out-of-bounds access (UB). Not D2: `num_tokens < toknext` with an array that holds the tokens (defined in C, kept as Q9) | Slices make `js == NULL` and `len` past the buffer unrepresentable. `num_tokens > tokens.len()`: allocation stops at `tokens.len()` with `Err(Error::NoMem)` (RP14 amendment). A reused `Parser` whose `toksuper`, `toknext − 1` or a `parent` link indexes past `tokens.len()` returns `Err(Error::Inval)` at the first such access, state as left at that point | no                                                         | approved; RP14 amendment (`num_tokens > tokens.len()` → `NoMem`) confirmed by the user 2026-10-10|
| D3 | `char` signedness (signed on macOS, unsigned on Linux aarch64) in `c < 32 \|\| c >= 127` and the hex checks                                                                                                                                                                                                 | `u8`; both C signednesses give the same result for every byte (bytes ≥ 0x80 are rejected either way)                                                                                                                                                                                                                                                              | informational: the Judge ran on macOS (signed `char`) only | approved                                                                                  |

## 9. Kept quirks

| id | Behaviour                                                                                                                                                                                                                                                                                                            | Where                                                         |
|----|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------|
| Q1 | A NUL byte ends the input even when `len` is larger                                                                                                                                                                                                                                                                  | every scan loop                                               |
| Q2 | Allocating a token does not reset its `kind`: in strict mode `{"a":1,{` allocates a token, then returns `Inval` with that token's `kind` unchanged from the caller's buffer and `start = -1`                                                                                                                         | `jsmn_alloc_token`, strict `{`/`[` arm                        |
| Q3 | On `NoMem`, `{`/`[` leaves `pos` at the bracket; strings and primitives reset `pos` to the token start                                                                                                                                                                                                               | `jsmn_parse`, `parse_string`, `parse_primitive`               |
| Q4 | Count mode does not match brackets; a string leaves `pos` at its closing quote; a primitive steps `pos` back by one                                                                                                                                                                                                  | count path                                                    |
| Q5 | Non-strict mode accepts any printable byte in a primitive, ends a primitive at `:`, accepts unquoted keys, `{"a" "b"}`, `[1 2]` and trailing commas                                                                                                                                                                  | non-strict `default` arm                                      |
| Q6 | Strict mode: a primitive that runs to the end of input is `Part`, not success                                                                                                                                                                                                                                        | `parse_primitive`                                             |
| Q7 | The returned count starts at `toknext` of the incoming parser (resumed parses count earlier tokens)                                                                                                                                                                                                                  | `jsmn_parse` entry                                            |
| Q8 | `parent-links` mode finds the enclosing container through `parent`; the default mode scans tokens backwards. Both are kept per feature, including their different error cases                                                                                                                                        | `}`/`]` and `,` arms                                          |
| Q9 | `num_tokens` only limits allocation: allocated tokens are read and updated through the caller's array whatever `num_tokens` is, so a reused parser given `num_tokens < toknext` still closes brackets, grows sizes and returns a count; only a new token gives `NoMem`. Defined in C when the array holds the tokens | `jsmn_alloc_token`; Stage 2 `alloc_token`, `parse_with_limit` |

## 10. Judge and acceptance

- Gate: `make pilot-jsmn STAGE=<n>` over the four feature sets (§3).
- Stage 1: `cargo check`, `cargo test`, upstream `test/tests.c` linked to the
  staticlib (16 tests,
  `FAILED: 0`), Differential harness (`harness/run.py`: C `dump.c` vs original,
  1202 inputs: every byte value in four positions (1188, decision 28; the
  fourth, primitive continuation `[1<c>]`, is the only one that reaches the
  per-byte check in `strict` mode, see RUN-NOTES "Stage 2 Judge validation")
  plus 14 targeted inputs for G-S2-1..5, and dump phases 5 "dirty" and 6
  "shrink").
- Stage 2: translated `rust/tests/upstream.rs` (all 16 upstream tests,
  `#[cfg(feature)]` where upstream
  uses `#ifdef`), Differential harness with `rust/examples/dump.rs` as the Rust
  side, clippy
  `-D warnings`, rustfmt, zero `unsafe`, no `unwrap()`/`expect()` in `src/`
  (jsmn has no infallible
  input-free path that needs one).
- Before Stage 2 edits `src/`, the Stage 2 Judge must pass on the Faithful port
  through the Stage 1
  adapter and must fail on the E2 mutant (`32..127` → `32..126`), see
  `judge.md`.
- Stage 2 attempt 2 (RP14) is gated only after the Judge gains the `dirty` and `shrink` dump phases and the G-S2
  corpus inputs (RUN-NOTES "Opus review"). The extended Judge must pass on the Faithful port through the Stage 1
  adapter and fail on commit `35e87c3` (Stage 2 attempt 1, `&mut buf[..n]` adapter) in at least one feature set.

## 11. Per-file obligations

`rust/src/lib.rs` starts with a `//!` doc comment naming the source (`jsmn.h` @
the commit in `UPSTREAM`) and copies no licence header: licence
information lives in `UPSTREAM`. It ends with
`// PORT STATUS: stage=<1|2> confidence=<high|medium|low> todos=<N> deviations=<D-ids or ->`.
Values are fixed: `confidence=high` when `cargo check` is clean in all four
feature sets and `todos=0`; `medium` when clean with `TODO(port)` markers;
`low` otherwise. `todos` counts `TODO(port)` markers. `deviations` is `D3` in
Stage 1 (bytes read as `u8`, §0; Stage 1 keeps C's D1/D2 behaviour) and
`D1,D2,D3` in Stage 2.

## 12. Amendments

| date       | proposer                       | section                         | proposal                                                                                                                                                                                                                  | decision                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
|------------|--------------------------------|---------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 2026-10-10 | E3 stress test (RUN-NOTES RP1) | §2 T8, §0                       | Stage 1 keeps `jsmntype_t = c_uint` + consts; `TokenKind` enum is Stage 2 only                                                                                                                                            | accepted: a Rust enum in a C-written field is UB the Judge cannot see                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 2026-10-10 | E3 stress test (RP2)           | §0                              | exact Stage 1 `jsmn_parse` signature, `num_tokens: c_uint`                                                                                                                                                                | accepted: `usize` for a 32-bit C parameter reads undefined upper bits                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 2026-10-10 | E3 stress test (RP3)           | §4                              | Stage 1 uses raw-pointer field access, no long-lived `&mut *ptr`                                                                                                                                                          | accepted: avoids aliasing `&mut` alongside `*mut` passed to helpers                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 2026-10-10 | E3 stress test (RP4)           | §11                             | `//!` doc header naming the source; no copied licence block                                                                                                                                                               | accepted: removes a free choice; licence is in `UPSTREAM`                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 2026-10-10 | E3 stress test (RP5)           | §0                              | `jsmntok_t` field is `type_0`                                                                                                                                                                                             | accepted: the adapter reads `t.type_0`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 2026-10-10 | E3 stress test (RP6)           | §0                              | exact `jsmn_parser` fields `pos: c_uint, toknext: c_uint, toksuper: c_int`                                                                                                                                                | accepted: `jsmn.h` declares `unsigned int pos, toknext`; A's `c_int` changed every cast and comparison                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 2026-10-10 | E3 stress test (RP7)           | §0                              | per-site arithmetic: `wrapping_*` on `pos`/`toknext` expressions, plain operators on `int` `count`/`size`/`start + 1`/`i`, `as c_int` for unsigned→int                                                                    | accepted with change: RP7 asked for `wrapping_*` on signed sites too; C signed overflow is UB (D1), so plain operators match the Faithful port. `toknext - 1` at 0 is Judge-reachable and must wrap                                                                                                                                                                                                                                                                                                                        |
| 2026-10-10 | E3 stress test (RP8)           | §2 C4                           | one `pos` increment at the loop tail; count-mode arms fall out of the `match`; no `continue`                                                                                                                              | accepted: the C `break` + `for` increment shape; replaces the old C4 text that asked for `pos += 1; continue` and conflicted with "keep the C structure"                                                                                                                                                                                                                                                                                                                                                                   |
| 2026-10-10 | E3 stress test (RP11)          | §4, §11                         | fixed Stage 1 marker sites (Q1-Q8, D3) and fixed PORT STATUS values                                                                                                                                                       | accepted: §9 "Where" is too vague to place markers; A and B both differed from the rulebook                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 2026-10-10 | E3 stress test (RP12)          | §2 C4, §4                       | `#ifdef`-guarded case labels become separate `#[cfg]` arms in C order, shared body written once per arm; no merged `_` arm with `matches!`, no `cfg!` guard; Q5 sits above the two `#[cfg(not(feature = "strict"))]` arms | accepted, B's shape (= Faithful port): HARD RULE 2 puts `#[cfg]` where `jsmn.h` has `#ifdef`, and each feature set then compiles one copy of the tail as C does; C's `cfg!`/`matches!` shape makes the label set a runtime test and leaves the Q5 site ambiguous                                                                                                                                                                                                                                                           |
| 2026-10-10 | E3 stress test (RP13)          | §0                              | Stage 1 emits `pub type jsmnerr = c_int` and types `JSMN_ERROR_*` as `jsmnerr`                                                                                                                                            | accepted: `jsmn.h` declares the named `enum jsmnerr`; the Faithful port and c2rust emit the alias. Nothing in the adapter, `dump.rs` or the Judge uses the name, so it fixes a free choice, not behaviour                                                                                                                                                                                                                                                                                                                  |
| 2026-10-10 | E3-prep Judge validation       | §10                             | corpus 932 → 1188 inputs (primitive-continuation byte class)                                                                                                                                                              | accepted: 932-input corpus missed the E2 mutant in both `strict` sets                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 2026-10-10 | Judge extension (Sonnet)       | §10                             | corpus 1188 → 1202 inputs (14 targeted G-S2-1..5), dump phases 5 and 6, adapter `poison_kinds` (`63a6486`, `a60f56a`)
| 2026-10-10 | E3 stress test close (Opus)    | status line                     | C vs D diff (16 hunks, 85 lines): 10 cosmetic, 6 covered by RP12/RP13, 0 new; status set to `stress-tested (2026-10-10)`                                                                                                  | accepted: the same-model run with RP1-RP13 produced no new disagreement. Limits: n=2 per rule set, one Part, Judge blind to style (RUN-NOTES "Stress test closed")                                                                                                                                                                                                                                                                                                                                                         |
| 2026-10-10 | Stage 2 review (RP14)          | §2 T5/T6, §6, §8 D2, §9 Q9, §10 | Rust maps `num_tokens` to the slice length, so a reused parser with `num_tokens < toknext` returns `Inval` where C succeeds; reviewer proposed widening D2                                                                | rejected as a Deviation, API changed instead: `parse_with_limit(js, tokens, num_tokens)` takes the whole array and the limit, reads are bounded by `tokens.len()` only (kept quirk Q9). Reproduced C vs Rust attempt 1: `[` (8) then `[]` (0): C 1, Rust −2; `[[` (8) then `[[]]` (1): C 2, Rust −2; `[[` then `[[]` (1): C −3, Rust −2. C is defined whenever the real array holds the tokens; the user's goal is C behaviour. D2 Rust column amended for `num_tokens > tokens.len()` (`NoMem`), confirmed by the user 2026-10-10 |
| 2026-10-10 | User (D2 judgement)            | §8 D2, status line              | confirm the RP14 amendment of the D2 Rust column                                                                                                                                                                          | accepted: where C is undefined (array shorter than `num_tokens`, reused parser past the array) Rust uses its memory safety and returns a defined error; where C is defined (`num_tokens < toknext`, Q9) Rust keeps C's result                                                                                                                                                                                                                                                                                                      |
| 2026-10-10 | Stage 2 review (RP15)          | §4                              | Stage 2 marker rows for the `get`/`get_mut` D2 sites; D1 marker note                                                                                                                                                      | accepted with RP14: Q9 (1) and D2 (11, incl. the new `alloc_token` site); D1 has no site                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 2026-10-10 | Stage 2 review (RP16)          | idiom C5                        | Stage 2 `rposition` for backward scans and the resumed scan `slice[..i]`                                                                                                                                                  | accepted (idiom table C5)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 2026-10-10 | Stage 2 review (RP17)          | idiom I10, §2 I1                | unsigned `x - 1` into a `-1` sentinel is `checked_sub(1)` → `Option<usize>`                                                                                                                                               | accepted (new idiom row I10)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 2026-10-10 | Stage 2 review (RP18)          | idiom E6                        | `arr[i]`: plain index with invariant comment when just validated, `get(i).ok_or(err)?` + Deviation marker when `i` comes from caller state                                                                                | accepted (new idiom row E6), with the RP14 rule: bound by the real slice length, never by a count parameter C uses only as a limit                                                                                                                                                                                                                                                                                                                                                                                         |
| 2026-10-10 | Stage 2 review (G-S2-1..5)     | §10                             | Judge coverage gaps: dirty buffer (Q2), `\u` + NUL, `{}:}`/`[]:]`, `"a":,`/`1:,`, `num_tokens < toknext`                                                                                                                  | accepted as Judge work (scratchpad `opus-judge-spec.md`); G-S2-1..4 reproduced as Rust == C in all four sets (coverage only), G-S2-5 is RP14                                                                                                                                                                                                                                                                                                                                                                               |
