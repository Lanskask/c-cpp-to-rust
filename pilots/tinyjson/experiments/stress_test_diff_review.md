# Ticket 03 stress test: review of the twice-translated groups

Reviewed 2026-10-12. Translations under `pilots/tinyjson/migration/harness/work/stress/{A1,A2,B1,B2,C1}/src/lib.rs`;
reference = `pilots/tinyjson/src/src/tinyjson.h` (lines cited as `h:NNN`). Facts marked "probe" were measured in this
review: a C++ probe against the header (GCC 16.1, `-std=c++17 -include locale -O0`) and a Rust probe calling the A1/A2
helpers from scratch copies of their crates (scratchpad, not committed). "Corpus" = `migration/harness/inputs` (5534
files) through `dump.cpp`/`dump.rs`; "tests" = the 22 translated upstream cases.

## Group A: `class parser` (h:622-1077)

Both agents also filled four stub bodies outside the group (`u32_sstream`, `trim`, `U32ToU8`, `U8ToU32`) without
having seen their source (h:29-68 was not in the slice). Those rows are marked "out of region".

| #    | Difference                                                        | A1                                                                                                                                                                                                                  | A2                                                                                                             | Faithful (C++)                                                                                                                                                                                          | Judge observes?                                                                                               | Class             |
|------|-------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------|-------------------|
| A-1  | `parse` input decoding of invalid UTF-8 (h:630-633)               | `U8ToU32` that stops at the first ill-formed sequence (whole input, no window)                                                                                                                                      | `U8ToU32` that returns `Err` (`invalid utf32 string: invalid start byte` / `incomplete sequence`)              | both wrong: C++ never errors here; EOF lands at the start of the 29-byte window holding the bad byte (D5, e1). A1 is the D5 fallback, A2 invents an error                                               | yes: corpus `utf8` class (633 inputs incl. offset sweeps)                                                     | DEVIATION-RELATED |
| A-2  | `parse_member` result building (h:744-769)                        | pushes `c as u8` and `escape_char()? as u8`, no `U32ToU8`                                                                                                                                                           | collects `Vec<u32>`, returns `U32ToU8(&v)`                                                                     | A2 (h:744, 769). Probe: key `"é"` gives `c3 a9` in C++ and A2, `e9` in A1                                                                                                                               | yes: 11 corpus inputs have a non-ASCII key; `streams.latin1.parse_member` reaches 0x80-0xFF                   | BEHAVIOUR         |
| A-3  | `parse_bool`/`parse_null` lowercasing of code points (h:818, 837) | `(c as u8).to_ascii_lowercase()`: truncates every code point to its low byte                                                                                                                                        | lowercases only `c <= 0x7F`, keeps the rest                                                                    | A2 (D10). Probe: `tru` + U+0165 is `invalid boolean string` in C++; A1 would read `true`                                                                                                                | no: 0 corpus inputs put a code point above U+00FF in a bool/null token (Latin-1 values survive `as u8`)       | BEHAVIOUR         |
| A-4  | `to_bool` lowercasing (h:913-918)                                 | lowercases (CPP30)                                                                                                                                                                                                  | compares as is                                                                                                 | A1. Probe: `to_bool("TRUE")` is `true` in C++                                                                                                                                                           | no: only `streams.to_bool(data)` reaches it unlowercased, and 0 corpus inputs are a mixed-case `true`/`false` | BEHAVIOUR         |
| A-5  | `to_integer` / `stoll` (h:929-941, D8, I9)                        | hand scanner: `InvalidArgument`/`OutOfRange "stoll"`, tail check; but `-9223372036854775808` → `OutOfRange`, `\v5` → `InvalidArgument` (Rust `is_ascii_whitespace` lacks `\v`), non-UTF-8 bytes → `InvalidArgument` | `trim` + `i64::from_str_radix`, every failure `RuntimeError "Unexpected number(integer) format."`; `" 5 "` → 5 | both wrong. Probe (C++): `" 5 "` runtime, `""`/`"-"` invalid_argument `stoll`, `99999999999999999999` out_of_range `stoll`, i64 min OK, `"\v5"` 5. A1 matches 4 of 6, A2 2 of 6                         | yes: number corpus, `streams.to_integer` on raw input; 13 inputs contain i64 min                              | BEHAVIOUR         |
| A-6  | `to_double` / `stod` (h:943-954, D8, I8)                          | `trim` + `f64::from_str`; failure `InvalidArgument "stod"`; `TODO(port)`                                                                                                                                            | same parse; failure `RuntimeError "Unexpected number(double) format."`                                         | both wrong. Probe (C++): `1.5x` runtime `Unexpected number(double) format.`, `abc` invalid_argument `stod`, `1e400` out_of_range `stod` (both return `inf`); no prefix scanner, no hex floats, no `pos` | yes: number corpus (129), translated tests h:448-460 region                                                   | DEVIATION-RELATED |
| A-7  | `get_next_non_space` return type (h:1007)                         | changed the stub signature to `-> i32`                                                                                                                                                                              | keeps `-> u32`                                                                                                 | A2 (`char32_t`); `harness/rust/adapter.rs:249-250` expects `u32`                                                                                                                                        | yes: the Judge build fails (exit 2)                                                                           | BEHAVIOUR         |
| A-8  | `U8ToU32` (h:56-68), out of region                                | returns the decoded prefix as `Ok` on an ill-formed sequence; drops a truncated tail                                                                                                                                | `Err` with invented texts; no continuation/overlong/surrogate check (`C3 28` → U+00E8)                         | both wrong. Probe: C++ throws `invalid utf32 string: wstring_convert::from_bytes` on `a\xFF`, but `a\xC3` (valid incomplete tail) returns `[a]`                                                         | yes: `streams.U8ToU32` on every input                                                                         | BEHAVIOUR         |
| A-9  | `u32_sstream::get` at end, out of region                          | advances while `pos < len`                                                                                                                                                                                          | advances while the value is not `u32::MAX`                                                                     | same for every stream without a literal 0xFFFFFFFF                                                                                                                                                      | no: driver streams hold UTF-8 or Latin-1 values only                                                          | COSMETIC          |
| A-10 | `parse_value` dispatch (h:662-699)                                | `match` on integer literals                                                                                                                                                                                         | `match` with guards                                                                                            | both                                                                                                                                                                                                    | n/a                                                                                                           | COSMETIC          |
| A-11 | `skip_char` message for `expected` (h:1029-1034)                  | hand UTF-8 encoding                                                                                                                                                                                                 | `char::from_u32(..).unwrap_or('?')`                                                                            | both for every call site (all ASCII)                                                                                                                                                                    | n/a                                                                                                           | COSMETIC          |
| A-12 | `parse_hex` after the loop                                        | folds digits with `to_digit`                                                                                                                                                                                        | `U32ToU8` + `from_str_radix` with unreachable error paths                                                      | both wrong the same way (shared defect S-1)                                                                                                                                                             | yes (S-1)                                                                                                     | COSMETIC          |
| A-13 | `escape_char` `u` arm                                             | value of the arm                                                                                                                                                                                                    | early `return`                                                                                                 | both                                                                                                                                                                                                    | n/a                                                                                                           | COSMETIC          |
| A-14 | `isspace`/`isxdigit` helpers                                      | private `parser::is_space`/`is_xdigit` with D10 markers                                                                                                                                                             | free functions, no marker                                                                                      | both (D10 sets)                                                                                                                                                                                         | n/a                                                                                                           | COSMETIC          |
| A-15 | markers and `PORT STATUS`                                         | `DEVIATION(port)` at D3, D4, D5, D10; status lists D2, D9 it never touches                                                                                                                                          | only D3 (without `:` spelling), `DECISION(port)` (not a §4 marker)                                             | neither follows §4 exactly                                                                                                                                                                              | no                                                                                                            | COSMETIC          |
| A-16 | `trim` whitespace constant, out of region                         | local `const`                                                                                                                                                                                                       | module `const`                                                                                                 | both (h:29-40)                                                                                                                                                                                          | n/a                                                                                                           | COSMETIC          |
| A-17 | `parse_number` integer test (h:900)                               | `b"0123456789-".contains`                                                                                                                                                                                           | `is_ascii_digit() \|\| == b'-'`                                                                                | both                                                                                                                                                                                                    | n/a                                                                                                           | COSMETIC          |

## Group B: copy constructor, `operator=`, `operator==`, `operator!=`, destructor

| #   | Difference                                        | B1                                                         | B2                                                                                             | Faithful (C++)                                                                                                                 | Judge observes?                                                                    | Class     |
|-----|---------------------------------------------------|------------------------------------------------------------|------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------|-----------|
| B-1 | `array == null`, `object == null` (h:297-317)     | `true` (explicit Q1 branch in an if/else chain over pairs) | `false`: one `match` arm per case; a mismatch leaves the `match` and returns `false`           | B1. Probe: both pairs print `1` in C++ (fallthrough to `case null`, h:313-317). All other pairs agree (checked against D1, D2) | yes: driver `matrix` section keeps the Q1 pairs (`eq_safe`), §10 extra test for Q1 | BEHAVIOUR |
| B-2 | `assign` (h:261-291)                              | `*self = other.clone()`                                    | `drop_payload()` then a copy of the copy-constructor switch                                    | both (D13: old payload dropped, same new value)                                                                                | n/a                                                                                | COSMETIC  |
| B-3 | `Drop` `invalid` arm (h:494-495 `default: break`) | no-op                                                      | `unreachable!()` (panic inside `Drop`)                                                         | B1; unreachable either way (§7)                                                                                                | no                                                                                 | COSMETIC  |
| B-4 | `eq` `invalid` arm (h:351-352 `default: false`)   | `false` via the final `else`                               | `unreachable!()`                                                                               | B1; unreachable                                                                                                                | no                                                                                 | COSMETIC  |
| B-5 | null checks before `Box::from_raw`                | none (relies on `_type`)                                   | `if !is_null()` in every arm                                                                   | both (`delete nullptr` is a no-op)                                                                                             | no                                                                                 | COSMETIC  |
| B-6 | markers                                           | `DEVIATION(port): D13`, `QUIRK(port): Q1`, D1, D2          | D2 site also tagged `QUIRK(port): Q2` (wrong id), `DECISION(port)`, D13 not in marker spelling | B1 closer to §4                                                                                                                | no                                                                                 | COSMETIC  |
| B-7 | `Clone` construction                              | builds `_value` from the `match`                           | builds a null `json`, then assigns `_value`                                                    | both                                                                                                                           | n/a                                                                                | COSMETIC  |

## Group C: `to_string`, `object_to_string`, `array_to_string` (h:544-617), C1 only

| #   | Finding in C1                                                                                                                              | Faithful?                                                                                                                   | Judge observes?                                                                                   | Class     |
|-----|--------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------|-----------|
| C-1 | keys and nested results are appended whole; comments say `BUG(port): Q5` but the code does not stop at the first NUL                       | no: h:582, 586, 606 go through `.c_str()` (Q5); a top-level string keeps the NUL (h:555), which C1 does                     | yes: 34 corpus inputs hold a NUL; `build.object_add_member` uses input bytes as keys; §10 Q5 test | BEHAVIOUR |
| C-2 | doubles: NaN/inf first, then `format!("{:.6}")`                                                                                            | yes as far as measured: equal to C `printf("%f")` on 14 edge values (halfway cases, `-0.0`, `1e300`, `1e-7`) in this review | yes: every serialised double                                                                      | —         |
| C-3 | getters inside arms wrapped in `if let Ok(..) else Vec::new()` / `unwrap_or_else`                                                          | same output (the arm fixes the type, so the getter cannot fail)                                                             | no                                                                                                | COSMETIC  |
| C-4 | markers: `BUG(port): Q4 -` without `QUIRK(port): Q4`; no `DEVIATION(port): D9` at the formatter; `DECISION(port)`; status lists D2, D3, D5 | §4 spelling not followed                                                                                                    | no                                                                                                | COSMETIC  |

Separators (`,` before every item but the first, CPP31), `" : "`, `true`/`false`/`null`, integer `Display`, unescaped
strings (Q4) and the unreachable `invalid` arm match the header.

## Defects both translations share

- **S-1 (A) D3 not applied.** Both `parse_hex` decode the 4 digits (probe: `A` → 65 in A1 and A2). D3 says every
  `\uXXXX` yields U+0001; A1 even carries the D3 marker above the decoding loop. Judge: yes (5 translated tests, `\u`
  corpus inputs).
- **S-2 (A, out of region) `U32ToU8` error text.** Both: `invalid utf32 string: U+D800`; C++ (probe):
  `invalid utf32 string: wstring_convert::to_bytes`. Judge: yes (`streams.U32ToU8(wide)` reaches surrogates).
- **S-3 (A) `stod` scanner missing** (A-6) and **D5 window missing** (A-1): neither agent had the measured rules,
  which live in RUN-NOTES and e1, not in the rulebook.
- **S-4 (B) SAFETY comments.** Both D2 branches dereference raw pointers in `unsafe` blocks without `// SAFETY:` (§4).
  Judge: no.
- **S-5 (stub)** The stub's `#[derive(Clone, Debug)]` on `json` contradicts CPP14 (hand-written `Clone`); both B agents
  had to remove `Clone` from it. Fix the stub/scratch crate before ticket 04.

## Rule proposals

One proposal per BEHAVIOUR or DEVIATION-RELATED difference (or group of them). Each quotes the existing row it amends
in full.

### P1. `operator==` fallthrough shape (B-1) — §2, amends row C4

Existing row:

```text
| C4         | `operator==` keeps its switch fallthrough as an explicit chain in case order `array, object, null, string, boolean, number_integer, number_double, default`; never merge or reorder the arms (Q1, D1, D2)                      | the fallthrough decides the result for mismatched types                           |
```

Proposed text (replaces the Override cell):

> `operator==` keeps its switch fallthrough as an explicit chain in case order `array, object, null, string, boolean,
> number_integer, number_double, default`; never merge or reorder the arms (Q1, D1, D2). A Rust `match` arm cannot
> fall through, so write the chain as `if`/`else if` over `(self._type, o._type)` pairs, not as one `match` arm per
> case that returns `false` on a mismatch. The complete result table: same type → compare the payloads; left `array`
> or `object`, right `null` → `true` (Q1); left `number_integer`, right `number_double` → D2 bit-pattern compare;
> every other pair → `false` (right type earlier in case order: `default`; later: D1). The `invalid` arm returns
> `false` like `default:`, it does not panic.

### P2. Code-point strings stay `u32` until `U32ToU8` (A-2) — §2.1, amends row CPP26

Existing row:

```text
| CPP26 | `char32_t` input streams: `u32_istream = std::basic_istream<char32_t>`, `u32_sstream = std::basic_stringstream<char32_t>` (`tinyjson.h:14-15`); every parser function takes `u32_istream&` and uses `peek()`/`get()` with EOF `0xFFFFFFFF` (`660-1075`)                                                                                                                                             | `pub struct u32_sstream { buf: Vec<u32>, pos: usize }` with `new(&[u32])`, `peek() -> u32`, `get() -> u32`, `EOF: u32 = u32::MAX`; parser functions take `&mut u32_sstream`                                                                               | `U32Stream`, same methods; stays public (tests call the parser functions directly)                                                                              | the position after an error is observable (Q13); the C++ `int c = peek()` turns EOF into `-1` (D7)                                                                                                                               | proposed |
```

Proposed addition to the Stage 1 cell:

> A local `std::u32string` is a `Vec<u32>` and is turned into bytes only where C++ calls `U32ToU8` (`parse_member`
> h:769, `parse_bool` h:822, `parse_null` h:841, `parse_string` h:881, `parse_number` h:897). Never cast a code point
> to `u8` (`c as u8`): it drops the UTF-8 encoding and the `U32ToU8` error.

### P3. `tolower` on a code point (A-3) — §2, amends row L4, L5, L6

Existing row:

```text
| L4, L5, L6 | `std::isxdigit`, `std::isspace`, `std::tolower` are called on `char32_t` code points: ASCII C-locale rules, and every value above 0x7F is "no" / unchanged (D10). Whitespace is `\t \n \v \f \r` and space (L5: `\v` included) | `tinyjson.h:818`, `837`, `917`, `1044`, `1059`                                    |
```

Proposed addition to the Override cell:

> The code form is fixed: `fn tolower32(c: u32) -> u32 { if c <= 0x7F { (c as u8).to_ascii_lowercase() as u32 }
> else { c } }`, `fn isspace32(c: u32) -> bool { matches!(c, 9..=13 | 32) }`, `fn isxdigit32(c: u32) -> bool { c <=
> 0x7F && (c as u8).is_ascii_hexdigit() }`. Never `(c as u8).to_ascii_lowercase()` without the `<= 0x7F` test, and
> never Rust `is_ascii_whitespace` (it lacks `\v`).

### P4. `to_bool` lowercases again (A-4) — §2.1, amends row CPP30

Existing row:

```text
| CPP30 | lambda in `std::transform` (`tinyjson.h:913-918`)                                                                                                                                                                                                                                                                                                                                                   | `for b in &mut s { *b = b.to_ascii_lowercase(); }`                                                                                                                                                                                                        | same                                                                                                                                                            | —                                                                                                                                                                                                                                | proposed |
```

Proposed addition to the trap cell:

> `to_bool` is public and the Judge calls it on raw input: keep the lowercasing even though `parse_bool` already
> lowercased its token. A statement that looks redundant in its only internal caller is still ported.

### P5. `stoll` scanner (A-5) — §2, amends row I9

Existing row:

```text
| I9         | `std::stoll` → hand-written `strtoll` scanner (base 10) + `i64` with `out_of_range` on overflow (D8)                                                                                                                           | `tinyjson.h:929-941`                                                              |
```

Proposed text (replaces the Override cell):

> `std::stoll(str, &pos)` → `fn stoll(s: &[u8]) -> Result<(i64, usize), Error>` on bytes (no UTF-8 check, no
> `trim`): skip leading C `isspace` bytes (`\t \n \v \f \r` and space), one optional `+` or `-`, then decimal digits.
> No digit → `Err(InvalidArgument, "stoll")`. Accumulate in `i128` (or as a negative number) so that
> `-9223372036854775808` parses; a value outside `i64` after all digits are read → `Err(OutOfRange, "stoll")`.
> `pos` = index after the last digit. `to_integer` then returns `Err(RuntimeError, "Unexpected number(integer)
> format.")` when `pos != s.len()` (h:935-938). Measured (probe): `" 5 "` → RuntimeError, `""` and `"-"` →
> InvalidArgument, `99999999999999999999` → OutOfRange, `"\v5"` → 5, i64 min → OK.

### P6. `stod` scanner (A-6) — §2, amends row I8

Existing row:

```text
| I8         | `std::stod` → hand-written `strtod` prefix scanner + `f64::from_str`, with the `pos`, `invalid_argument` and `out_of_range` rules of D8                                                                                        | `tinyjson.h:943-954`                                                              |
```

Proposed text (replaces the Override cell):

> `std::stod(str, &pos)` → `fn stod(s: &[u8]) -> Result<(f64, usize), Error>` on bytes, never `trim` +
> `f64::from_str` of the whole token: skip leading C `isspace`, one optional sign, then the longest prefix of one of:
> `inf`/`infinity`/`nan`/`nan(` [alnum `_`]* `)` (ASCII case-insensitive); `0x`/`0X` hex float (hex digits, optional
> `.`, optional `p`/`P` exponent that is consumed only if digits follow); decimal digits with optional `.` and
> optional `e`/`E` exponent consumed only if digits follow. No digit consumed → `Err(InvalidArgument, "stod")`. The
> value is `f64::from_str` of the decimal prefix (hex: exact computation, correctly rounded). Finite digits that give
> ±inf, or non-zero digits that give 0 or a subnormal → `Err(OutOfRange, "stod")` (D8 measured `1e400`, `1e-310`,
> `1e-400`). `to_double` then returns `Err(RuntimeError, "Unexpected number(double) format.")` when `pos !=
> s.len()`. Probe: `1.5x` → RuntimeError, `abc` → InvalidArgument, `1e400` → OutOfRange.

### P7. Stub signatures are binding (A-7) — §5, amends the second bullet

Existing bullet:

```text
- Stage 1 keeps the C++ names (`json`, `json_t`, `parser`, `u32_sstream`, `get_string`, `U32ToU8`, …) with
  `#![allow(non_camel_case_types, non_snake_case)]`. Overloads and operators get these names (proposed):
```

Proposed sentence appended to the bullet:

> The signatures in the stub crate (`experiments/scratch-crate/src/lib.rs`) are binding in Stage 1: never change a
> parameter type, return type, name or visibility, because `harness/rust/adapter.rs` compiles against them (for
> example `get_next_non_space -> u32`, `peek_next_non_space -> i32`); compare with `-1i32` or `u32::MAX` at the call
> site instead.

### P8. D5 carries the measured window rule (A-1) — §8, amends row D5

Existing row:

```text
| D5  | Input decoding through `wbuffer_convert` (`tinyjson.h:630-633`): invalid UTF-8 (a stray byte, an overlong form, an encoded surrogate, a code point above U+10FFFF) is not an error but an early EOF, at the start of the libstdc++ conversion chunk that holds it. Measured: `["` + 26 `a` + `\xff"]` (32 bytes) fails `invalid json format` (the first `peek` is already EOF); with 27 `a` it fails `expected char '"' not found`; a valid document followed by garbage about 40 bytes later parses OK. A UTF-8 BOM is not skipped (U+FEFF is not whitespace) in both                                                                                                  | emulate the libstdc++ `wbuffer_convert` chunk rule so the early EOF lands at the same byte as in the Reference. The exact rule is measured with the C++ driver in ticket 02 before Stage 1. If it cannot be reproduced exactly, fall back to ending the input at the first invalid byte and record the remaining mismatch as a new Deviation row; `DEVIATION(port): D5`                                                                                                                              | yes: corpus byte classes 0x80-0xFF, compared byte-exactly (error text and parse result)                                                                                                                                                                                                                                                                                                                                                                                                                   | approved (user rule: match C++ origin as closely as possible) |
```

Proposed text (replaces the Rust-behaviour cell):

> Emulate the libstdc++ `wbuffer_convert` window rule (measured exactly, experiment e1, 20160 inputs, 0 mismatches):
> `parse` does not call `U8ToU32`. It decodes the bytes before the first NUL in windows of at most 29 bytes; window 0
> starts at byte 0. A window is decoded as a whole with the Unicode well-formed table (no overlongs, no surrogates,
> nothing above U+10FFFF). If its last bytes are a valid but incomplete sequence of `u` bytes, the next window starts
> at those bytes and holds them plus `29 - u` new bytes. If a window holds an ill-formed byte (also a sequence cut by
> the window end that is ill-formed so far), the stream ends at the **start of that window**: every character of that
> window is dropped, nothing after it is read, no error is raised. An incomplete sequence at the very end of the input
> is dropped. `DEVIATION(port): D5`.

### P9. `U8ToU32` / `U32ToU8` error texts (A-8, S-2) — §2.1, amends row CPP28

Existing row:

```text
| CPP28 | `std::wstring_convert<std::codecvt_utf8<char32_t>, char32_t>` `to_bytes`/`from_bytes` (`U32ToU8`, `U8ToU32`, `tinyjson.h:42-68`)                                                                                                                                                                                                                                                                    | hand-written UTF-8 encoder/decoder with the C++ error message (`"invalid utf32 string: "` + library text, D4)                                                                                                                                             | same                                                                                                                                                            | a surrogate code point (`0xD800-0xDFFF`) or a value above `0x10FFFF` fails (D4); `U8ToU32` has no caller in the header but is public: port it                                                                                    | proposed |
```

Proposed text (replaces the Stage 1 cell):

> Hand-written UTF-8 encoder/decoder. `U32ToU8` fails on a surrogate or a value above `0x10FFFF` with
> `Err(RuntimeError, "invalid utf32 string: wstring_convert::to_bytes")`. `U8ToU32` decodes with the Unicode
> well-formed table and fails on any ill-formed sequence with `Err(RuntimeError, "invalid utf32 string:
> wstring_convert::from_bytes")`; a valid but incomplete sequence at the very end is dropped without error (probe:
> `a\xFF` fails, `a\xC3` → `[U+0061]`).

### P10. Q5 is code, not a comment (C-1) — §9, amends row Q5

Existing row:

```text
| Q5  | Inside `object_to_string`/`array_to_string` keys and nested results go through `.c_str()`, so they stop at the first NUL: `["a\u0000b"]`-like content prints as `["a]` (measured with a `std::string` holding a NUL); a top-level string keeps the NUL (BUG)                                                                                                                                                                                                                                                               | `tinyjson.h:582`, `586`, `606` vs `555`                  |
```

Proposed addition to the Behaviour cell:

> Stage 1 code: a helper `fn c_str(s: &[u8]) -> &[u8]` (the bytes before the first NUL) applied to the key at h:582
> and to the nested `to_string()` result at h:586 and h:606, not to the top-level string at h:555. A `BUG(port)` or
> `QUIRK(port)` marker never replaces reproducing the behaviour.

## Stress test verdict

**Apply before ticket 04** (each one caused a Judge-visible difference, or both agents got it wrong):

- P1 (C4 fallthrough as pair chain), P2 (CPP26 `Vec<u32>` until `U32ToU8`), P5 (I9 `stoll`), P6 (I8 `stod`),
  P7 (§5 stub signatures binding), P8 (D5 window rule in the rulebook itself), P9 (CPP28 error texts), P10 (Q5 code).
- P3 and P4: cheap to apply; the Judge cannot catch them today (see below).
- Fix the stub's `#[derive(Clone, Debug)]` on `json` (S-5).

**Already covered by the Judge** (a wrong translation would turn it red): D3 (S-1: 5 translated tests, `\u` corpus),
P1 (matrix, Q1 test), P2 (11 non-ASCII-key inputs, Latin-1 stream probes), P5/P6 (number corpus, `streams.to_*`),
P7 (adapter build, exit 2), P8 (633 `utf8` inputs), P9 (`streams.U8ToU32`, `streams.U32ToU8(wide)`), P10 (34 inputs
with NUL). **Not covered:** P3 (0 inputs with a code point above U+00FF in a bool/null token) and P4 (0 inputs that
are a mixed-case `true`/`false`); add `TRUE` and `[truť]` to `make_inputs.py`. Cosmetic rows (markers, unreachable
arms) are invisible to the Judge.

**Open questions for the user:**

1. The A agents received only h:622-1077 and invented `U8ToU32`, `U32ToU8`, `trim` and `u32_sstream` (A-8, S-2).
   Should a group's prompt include the source of every stub function it calls, or forbid filling bodies outside the
   group (they stay `todo!()`)?
2. P6 underflow: accept "non-zero digits giving 0 or a subnormal → `out_of_range`" (D8's measured `1e-310`; the
   Judge-green rough crate uses it) as the rule now, or measure the boundary first?
3. C2 has not run: run it (and a second A/B round with P1-P10 applied, ticket 03 acceptance "a same-model rerun
   agrees") before ticket 04, or accept C1 plus P10?
4. Should §4 forbid markers outside its list (`DECISION(port)` appeared in A2, B2, C1) and require the exact
   `DEVIATION(port): D<n>` spelling? This is cosmetic only.
