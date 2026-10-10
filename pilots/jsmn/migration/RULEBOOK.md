# RULEBOOK: jsmn (https://github.com/zserge/jsmn @ 25647e6)

Status: deviations-approved (2026-10-10, user approved D1-D3). Stress test pending.
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
  `jsmn_parser`, constants `JSMN_*` with the C values.
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

| Idiom row | Override for jsmn                                                                                                                                               | Reason                                                                          |
|-----------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------|
| T4        | Input stays `&[u8]`, and a NUL byte ends the input even inside the slice (Q1)                                                                                   | defined behaviour of the original                                               |
| T6        | `tokens == NULL` (count-only mode) becomes the separate method `Parser::count`; both methods share one private implementation taking `Option<&mut [Token]>`     | the mode is API behaviour; one implementation keeps the two modes from drifting |
| T8        | `TokenKind` keeps the C discriminants 0, 1, 2, 4, 8 (`#[repr(i32)]`)                                                                                            | the dump prints `type=%d`                                                       |
| C1        | `goto found` in `jsmn_parse_primitive`: labelled block or early exit in Stage 2 (Stage 1 keeps the `found` flag)                                                | idiom C1                                                                        |
| C4        | In `jsmn_parse`, the `{`/`[` arm with `tokens == NULL` leaves the `switch`, then `pos++` runs: in Rust advance `pos` and `continue` (as the Faithful port does) | `break` inside `switch` is not `break` of the loop                              |
| I1        | Stage 2: positions are plain `usize`; `pos - 1` after a primitive cannot underflow (`pos > start` there); write that invariant as a comment                     | the Faithful port's `wrapping_*` is only needed for C's `unsigned int`          |
| E1        | `enum Error { NoMem, Inval, Part }`, `code()` = −1, −2, −3                                                                                                      | dump and tests print / compare the codes                                        |

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
C-ABI functions. Stage 2:
none, anywhere under `rust/src/`. Markers used here: `QUIRK(port): Q<n>`,
`DEVIATION(port): D<n>`.

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
    pub fn parse(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<usize, Error>;
    pub fn count(&mut self, js: &[u8]) -> Result<usize, Error>;                // tokens == NULL
    pub fn pos(&self) -> usize;
    pub fn toknext(&self) -> usize;
    pub fn toksuper(&self) -> Option<usize>;                                    // C: -1 = None
}
```

- `len` is `js.len()`; `num_tokens` is `tokens.len()`.
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

| id | Original behaviour                                                                                                                                                                        | Rust behaviour                                                                                                                                                                                                 | Reachable by Judge?                                        | Status   |
|----|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------|----------|
| D1 | Input ≥ 2³¹ bytes or ≥ 2³¹ tokens: `int` positions/counts overflow (signed overflow is UB; `(int)pos` is implementation-defined); `pos` is `unsigned int` and wraps at 2³²                | `usize` positions and counts, no overflow                                                                                                                                                                      | no (harness reads ≤ 1 MiB)                                 | approved |
| D2 | API misuse: `num_tokens` larger than the real array, `js == NULL`, `len` past the buffer, a parser reused with a token array shorter than `toknext`/`toksuper`: out-of-bounds access (UB) | Slices make the first three unrepresentable. A reused `Parser` whose `toksuper` or `toknext − 1` indexes past `tokens.len()` returns `Err(Error::Inval)` at the first such access, state as left at that point | no                                                         | approved |
| D3 | `char` signedness (signed on macOS, unsigned on Linux aarch64) in `c < 32 \|\| c >= 127` and the hex checks                                                                               | `u8`; both C signednesses give the same result for every byte (bytes ≥ 0x80 are rejected either way)                                                                                                           | informational: the Judge ran on macOS (signed `char`) only | approved |

## 9. Kept quirks

| id | Behaviour                                                                                                                                                                                    | Where                                           |
|----|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------|
| Q1 | A NUL byte ends the input even when `len` is larger                                                                                                                                          | every scan loop                                 |
| Q2 | Allocating a token does not reset its `kind`: in strict mode `{"a":1,{` allocates a token, then returns `Inval` with that token's `kind` unchanged from the caller's buffer and `start = -1` | `jsmn_alloc_token`, strict `{`/`[` arm          |
| Q3 | On `NoMem`, `{`/`[` leaves `pos` at the bracket; strings and primitives reset `pos` to the token start                                                                                       | `jsmn_parse`, `parse_string`, `parse_primitive` |
| Q4 | Count mode does not match brackets; a string leaves `pos` at its closing quote; a primitive steps `pos` back by one                                                                          | count path                                      |
| Q5 | Non-strict mode accepts any printable byte in a primitive, ends a primitive at `:`, accepts unquoted keys, `{"a" "b"}`, `[1 2]` and trailing commas                                          | non-strict `default` arm                        |
| Q6 | Strict mode: a primitive that runs to the end of input is `Part`, not success                                                                                                                | `parse_primitive`                               |
| Q7 | The returned count starts at `toknext` of the incoming parser (resumed parses count earlier tokens)                                                                                          | `jsmn_parse` entry                              |
| Q8 | `parent-links` mode finds the enclosing container through `parent`; the default mode scans tokens backwards. Both are kept per feature, including their different error cases                | `}`/`]` and `,` arms                            |

## 10. Judge and acceptance

- Gate: `make pilot-jsmn STAGE=<n>` over the four feature sets (§3).
- Stage 1: `cargo check`, `cargo test`, upstream `test/tests.c` linked to the
  staticlib (16 tests,
  `FAILED: 0`), Differential harness (`harness/run.py`: C `dump.c` vs original,
  932 inputs including every
  byte value in three positions, decision 28).
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

## 11. Per-file obligations

`rust/src/lib.rs` ends with
`// PORT STATUS: stage=<1|2> confidence=<...> todos=<N> deviations=<D-ids or ->`.

## 12. Amendments

| date | proposer | section | proposal | decision |
|------|----------|---------|----------|----------|
