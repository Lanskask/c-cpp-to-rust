# Idiom table: C → Rust

Loaded by every Stage 1 and Stage 2 prompt (rows matched by inventory tag) and by the rulebook template.
A Pilot's `RULEBOOK.md` may **override** a row; it never silently invents a second mapping. A construct
missing here is flagged by the reviewer and added (Opus) between batches.

Written 2026-10-10 from general knowledge (PLAN §10). **Every row is `verified: no`** until a Pilot test
exercises it; then set `verified: <pilot>@<commit>` and name the test or harness input that exercised it.
A row whose mapping a Pilot disproves is rewritten, not deleted, and the disproof goes in `trap`.

Columns: `id` (cite it in code review and `RUN-NOTES.md`), `C`, `Stage 1` (Faithful port: same
structure, C ABI, `unsafe` allowed), `Stage 2` (Idiomatic pass: zero `unsafe`, decision 10),
`trap` (where a naive mapping changes behaviour), `tags` (inventory tags that pull this row into a prompt;
`*` = always included), `verified`.

## T. Types

| id | C | Stage 1 | Stage 2 | trap | tags | verified |
|---|---|---|---|---|---|---|
| T1 | `int`, `unsigned`, `long`, `short` | `c_int`, `c_uint`, `c_long`, `c_short` | `i32`/`u32`; `long` → `i64` | `long` is 32-bit on Windows (LLP64), 64-bit on macOS/Linux (LP64): platform behaviour, list under Deviations if a value can exceed 2³¹ | * | no |
| T2 | `size_t`, `ssize_t`, `ptrdiff_t` | `usize`, `isize` | `usize`/`isize` | C code mixing `size_t` and `int` relies on implicit conversion; make every cast explicit and check it cannot truncate | * | no |
| T3 | `char` used as a byte | `c_char` | `u8` | `char` is signed on macOS and x86 Linux, unsigned on Linux aarch64: `c >= 0x80` comparisons flip sign. Check each comparison gives the same result under both before switching to `u8` | * | no |
| T4 | `char *` NUL-terminated string | `*const c_char` | `&[u8]` (parsers, binary) or `&str` only where the C code requires valid UTF-8 | C stops at the first NUL even when a length is also passed (e.g. jsmn); keep the NUL stop if the original has it | * | no |
| T5 | pointer + length pair | `*const T, usize` | `&[T]` / `&mut [T]` | a length larger than the real buffer is UB in C and unrepresentable in Rust: Deviation | * | no |
| T6 | nullable pointer argument | raw pointer + `is_null()` | `Option<&T>` / `Option<&mut T>` | a NULL that selects a *mode* (jsmn `tokens == NULL` = count only) is API behaviour: keep the mode, possibly as a separate method | * | no |
| T7 | `_Bool`, int used as boolean | `bool` / `c_int` | `bool` | `if (x)` on non-0/1 ints; a C function returning a truthy int whose exact value callers read is not a bool | * | no |
| T8 | `enum` | `type E = c_uint;` + `const`s (c2rust style) | `#[repr(i32)] enum` with explicit discriminants if values are observable; `TryFrom<i32>` for incoming values | a C enum variable may hold any integer; casting an unknown value into a Rust enum is UB, so convert with `TryFrom` | * | no |
| T9 | `struct` | `#[repr(C)]` + `#[derive(Copy, Clone)]` | plain struct, owned fields, `Default` if the C code zero-initialises it | field order/padding is observable only through the C ABI; Stage 2 may reorder | * | no |
| T10 | `union` | `#[repr(C)] union` (reads are `unsafe`) | `enum` when a tag field exists; byte array + `from_le_bytes`/`to_le_bytes` for type punning | type punning reads the *platform* byte order; a byte-array mapping must pick one endianness: Deviation if big-endian matters | union | no |
| T11 | bitfields | manual masks on an integer field | manual masks or a small newtype with accessors | bit order and storage unit are implementation-defined; only matters if the layout is serialised | * | no |
| T12 | function pointer | `Option<unsafe extern "C" fn(..)>` | `fn` pointer, generic `F: Fn`, or a trait object | a NULL function pointer that means "default behaviour" becomes `Option` | * | no |
| T13 | `void *` user data / callbacks | `*mut c_void` | generic `T` or a closure capturing the data | — | * | no |
| T14 | flexible array member / trailing allocation | raw pointer arithmetic | `Vec<T>` / `Box<[T]>` | — | malloc | no |
| T15 | out-parameter `T *out` | `*mut T` | return value, tuple, or struct | a C function that writes `*out` *and* returns an error code may write a partial value on error; keep it if a caller reads it | * | no |

## I. Integer and float semantics

| id | C | Stage 1 | Stage 2 | trap | tags | verified |
|---|---|---|---|---|---|---|
| I1 | unsigned arithmetic (wraps, defined) | `wrapping_add/sub/mul` | `wrapping_*` where wrap is reachable; plain ops with an invariant comment where it is not | plain `+`/`-` panics on overflow in debug builds, which the Judge runs: a panic is a Judge failure, not a pass | * | no |
| I2 | signed overflow (UB) | `wrapping_*` (c2rust's choice) | `checked_*` returning the rulebook's error, or plain ops with a proof comment | true UB: list every reachable site under Deviations | * | no |
| I3 | shift by ≥ width, negative shift (UB) | `wrapping_shl` | `checked_shl` / explicit guard | Deviation if reachable | * | no |
| I4 | integer promotion (`unsigned char + int`, `uint16 * uint16`) | explicit `as` casts in the promoted type | same | `u16 * u16` promotes to *signed* int in C and can overflow (UB); compute in `u32` | * | no |
| I5 | `/`, `%` | same operators | same | both truncate toward zero (match). `INT_MIN / -1` is UB in C, panics in Rust: Deviation if reachable | * | no |
| I6 | float → int cast | `as` | `as` | out-of-range is UB in C, saturates in Rust: Deviation if reachable | * | no |
| I7 | `double` printing `%g`, `%.17g`, `%f` | `libc::snprintf` (Stage 1 may call libc) | hand-written formatter matching C output; Rust `{}` prints shortest round-trip, `{:e}` differs from `%e` | the most likely source of harness DIFFs in serialisers (cJSON). Test with ±0, subnormals, 1e21, NaN, inf | * | no |
| I8 | `strtod`, `atof` | `libc::strtod` | hand-written prefix scanner + `str::parse::<f64>` on the scanned prefix | Rust `parse` rejects leading space, hex floats, partial prefixes; C `strtod` is locale-dependent (decimal comma): match the "C" locale only | * | no |
| I9 | `strtol`, `atoi` | `libc::strtol` | prefix scanner + `i64::from_str_radix` + explicit clamp | `strtol` clamps to `LONG_MAX` and sets `errno`; `atoi` overflow is UB | errno | no |

## C. Control flow

| id | C | Stage 1 | Stage 2 | trap | tags | verified |
|---|---|---|---|---|---|---|
| C1 | `goto found` / forward jump out of a loop | boolean flag or labelled block `'found: { .. break 'found; }`; the code after the label is written once, after the loop, never copied to each jump site | labelled block or early `return` | c2rust emits a `current_block` state machine; rewrite it in Stage 1 only if the Judge still passes | goto | no |
| C2 | `goto cleanup` (single exit, frees resources) | keep structure, flag + one exit | RAII (`Drop`, owned values) + `?` | the cleanup order may be observable (output flushed on close) | goto | no |
| C3 | `goto retry` / backward jump | `loop { .. continue; }` | same | — | goto | no |
| C4 | `switch` with fallthrough | `match` with combined arms; duplicate the fallthrough body if arms differ | same | a `break` inside `switch` inside a loop leaves the `switch`, not the loop (jsmn's `{` arm) | * | no |
| C5 | `for (i = n - 1; i >= 0; i--)` with signed `i` | `while i >= 0 { ..; i -= 1; }` with `i: c_int`; keep tests on the exit value (`i == -1`) and a later loop that resumes from the same `i`; no `.rev()` range, no extra flag |  `(0..n).rev()` or `while let` | with an unsigned index the C loop never ends; check the original's type | * | no |
| C6 | `setjmp` / `longjmp` | **unknown tier**: ask the user or stub | `Result` + `?` (if the jump is error escape) | cannot be translated mechanically | setjmp | no |
| C7 | `assert()` | `assert!` / `debug_assert!` matching `NDEBUG` use | `debug_assert!` for invariants; `Result` if input-driven (decision 10) | an `assert` with side effects disappears under `NDEBUG` | * | no |
| C8 | `abort()`, `exit()` inside a library | same via `libc` / `std::process` | return `Err`; never exit from library code | changes observable behaviour on bad input: Deviation | * | no |

## E. Errors and sentinels

| id | C | Stage 1 | Stage 2 | trap | tags | verified |
|---|---|---|---|---|---|---|
| E1 | negative return code / error `enum` | same `c_int` constants | `Result<T, Error>` with an `enum Error` carrying the C codes (`fn code(self) -> i32`) | the numeric code is often observable (printed, compared): keep a 1:1 mapping | * | no |
| E2 | `NULL` return = failure | null pointer | `Option<T>` or `Result<T, Error>` | **error-recovery rule**: a NULL from allocation failure and a NULL from bad input are different kinds (`allocation-guard` vs `precondition-guard`); they must not share one Rust construct | malloc | no |
| E3 | `errno` | `std::io::Error::last_os_error()` / `libc` errno | `Result<_, Error>`; no global error state | `errno` is only meaningful right after the failing call | errno | no |
| E4 | error *with* partial result (parser stops at position N) | same | `Err` plus accessor to the partial state (e.g. parser position, tokens written so far) | Same behaviour includes error positions (CONTEXT.md); a `Result` that drops the partial state loses observable behaviour | * | no |
| E5 | success value encoded with the error (`>= 0` count, `< 0` error) | same `c_int` | `Result<usize, Error>` | — | * | no |

## A. Allocation and ownership

| id | C | Stage 1 | Stage 2 | trap | tags | verified |
|---|---|---|---|---|---|---|
| A1 | `malloc` + `free` of one object | `libc::malloc`/`free` or `Box::into_raw` | `Box<T>` or plain owned value | — | malloc | no |
| A2 | `malloc(n * sizeof T)`, `calloc` | `libc::calloc` | `vec![T::default(); n]` | `calloc` zero-fills; `malloc` contents are indeterminate (reading them is UB, never reproduce) | malloc | no |
| A3 | `realloc` growth | `libc::realloc` | `Vec::push` / `reserve` | pointers into the old block dangle after `realloc`; use indices in Stage 2 | malloc | no |
| A4 | malloc failure check (`if (!p) return NULL`) | keep | unreachable: Rust aborts on OOM. Delete the branch and list it under Deviations | do not delete *precondition* guards that look the same (E2) | malloc | no |
| A5 | `strdup`, string building | `libc` | `Vec<u8>` / `String` | — | malloc | no |
| A6 | user allocator hooks (`cJSON_InitHooks`) | keep as function-pointer globals | drop the hooks; Deviation (observable only through custom allocators) | — | malloc | no |
| A7 | linked list / tree with `next`/`prev`/`child` pointers | raw pointers | `Vec<Node>` + indices (arena), or `Vec<Child>` per node | iteration order and "detach then re-attach" semantics must survive; `Rc<RefCell>` only with a rulebook reason | malloc | no |
| A8 | caller-owned buffer filled by the callee | `*mut T` + capacity | `&mut [T]` | "buffer too small" is a precondition result (jsmn `JSMN_ERROR_NOMEM`), not allocation failure: keep it | * | no |
| A9 | `memcpy` | `core::ptr::copy_nonoverlapping` | `copy_from_slice` | overlapping `memcpy` is UB in C | * | no |
| A10 | `memmove` | `core::ptr::copy` | `slice::copy_within` | — | * | no |
| A11 | `memset(p, 0, sizeof *p)` | `core::ptr::write_bytes` | `*p = T::default()` | — | * | no |

## L. libc functions

| id | C | Stage 1 | Stage 2 | trap | tags | verified |
|---|---|---|---|---|---|---|
| L1 | `strlen` | `libc::strlen` / `CStr` | slice `.len()` or `iter().position(\|&b\| b == 0)` | — | * | no |
| L2 | `strcmp`, `strncmp`, `memcmp` | `libc` | slice `==` / `.cmp()` | C returns any negative/positive int; if a caller prints or stores the magnitude, compute it the C way | * | no |
| L3 | `strchr`, `strstr`, `memchr` | `libc` | `iter().position`, `windows().position` | `strchr(s, '\0')` finds the terminator | * | no |
| L4 | `isdigit`, `isxdigit` | `libc` or `u8::is_ascii_*` | `u8::is_ascii_digit`, `is_ascii_hexdigit` | `is*` on a negative `char` other than `EOF` is UB (T3) | * | no |
| L5 | `isspace` | `libc::isspace` | `matches!(b, b' ' \| b'\t' \| b'\n' \| b'\x0b' \| b'\x0c' \| b'\r')` | **`u8::is_ascii_whitespace` excludes `\v` (0x0B)**; C `isspace` includes it | * | no |
| L6 | `isalpha`, `isalnum`, `isprint`, `toupper`, `tolower` | `libc` | `u8::is_ascii_*`, `to_ascii_*` | C is locale-dependent; Rust ASCII helpers equal the "C" locale only. Underscore is not alphabetic in either | * | no |
| L7 | `printf`, `fprintf`, `puts` in library code | `libc::printf` | write to a `&mut impl Write` / return `String` | buffering and stdout/stderr interleaving | varargs | no |
| L8 | `sprintf`, `snprintf` | `libc::snprintf` | `write!` into `String`/`Vec<u8>` | `snprintf` returns the length it *would* have written and truncates silently; keep that if callers use the return | varargs | no |
| L9 | `fopen`/`fread`/`fwrite`/`fclose` | `libc` | `std::fs::File`, `impl Read` / `impl Write` | text vs binary mode is a no-op on POSIX, not on Windows | * | no |
| L10 | `qsort` | `libc::qsort` | `sort_unstable_by` | order of equal elements is unspecified in C and may differ from Rust: Deviation if the output shows it | * | no |
| L11 | `abs`, `labs`, `fabs`, `floor`, `pow`, `fmod` | `libc` / `f64` methods | `i32::abs` (panics on `i32::MIN` in debug: I2), `f64::abs`, `floor`, `powf`, `%` | `abs(INT_MIN)` is UB | * | no |
| L12 | `rand`, `time`, `clock` | `libc` | inject a source (parameter or trait) | non-deterministic: exclude from the Differential harness | * | no |

## M. Macros and preprocessor

| id | C | Stage 1 | Stage 2 | trap | tags | verified |
|---|---|---|---|---|---|---|
| M1 | object-like `#define N 42` | `pub const N: T = 42;` | same, typed | the macro's type is whatever the expression promotes to | * | no |
| M2 | function-like macro | `#[inline] fn` (or `macro_rules!` if it takes types/tokens) | `fn` | arguments evaluated twice in C (`MAX(i++, j)`) | macro-fn | no |
| M3 | configuration macro (`#ifdef FEATURE`) changing behaviour or layout | cargo feature + `#[cfg(feature = "...")]`; the Judge runs every upstream-tested combination (decision 24) | same | c2rust translates after preprocessing: one run = one configuration | config-macro | no |
| M4 | linkage/packaging macro (`JSMN_STATIC`, `JSMN_HEADER`, `*_API`, `extern "C"` guards) | no feature; express with `pub` / private | same | — | config-macro | no |
| M5 | platform macro (`_WIN32`, `__APPLE__`) | `#[cfg(target_os = "...")]` | same | untested platforms are listed under Deviations, not silently dropped | config-macro | no |

## G. Globals and state

| id | C | Stage 1 | Stage 2 | trap | tags | verified |
|---|---|---|---|---|---|---|
| G1 | file-level mutable global | `static mut` | first that fits: `const` → `static` → `OnceLock<T>` → `AtomicT` → `OnceLock<Mutex<T>>` (ENCRUST order) | signal-handler reachable globals: only `const`/`static`/atomic | static-global | no |
| G2 | function-local `static` | `static mut` at module level | move into the owning struct or G1 | — | static-global | no |
| G3 | global error/state shared by calls (`cJSON_GetErrorPtr`) | `static mut` | return it in `Err`; if the API must keep it, thread-local `Cell` | not thread-safe in C; Rust version may be: not a behaviour difference for single-threaded tests | static-global | no |

## V. Varargs

| id | C | Stage 1 | Stage 2 | trap | tags | verified |
|---|---|---|---|---|---|---|
| V1 | variadic function in library code | keep via `libc` calls; defining a C-variadic fn needs nightly `c_variadic` | slice `&[Arg]`, builder, or `fmt::Arguments` | avoid nightly in Stage 1 if any other route exists (decision 27) | varargs | no |
| V2 | variadic test helper (jsmn `tokeq`) | — (original tests stay in C, decision 27) | slice of expected values in the translated Rust test | — | varargs | no |
