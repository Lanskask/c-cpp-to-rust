# Idiom table: C++ → Rust

Loaded next to `idiom-table-c.md` for a C++ Pilot (`inventory.py --lang cpp`); the C table still applies to the C
subset of the code. The rules about overrides, `trap` and the `verified` column are the same as in the C table.

**Status: seed.** Every row was generalised from a construct in the first C++ Pilot (tinyjson, `seed` column = its
RULEBOOK §2.1 row) and from the translation rules of the C++ prior art; none was compiled and compared yet, so every
row is `verified: no`. A row that a Pilot disproves is rewritten with the disproof in `trap`. The `seed` rows carry
the `tinyjson.h` line numbers that justified them; a construct that no row names is flagged by the reviewer and
added here (Opus) between batches.

Columns: `id` (prefix `X`, cite it like a C row), `C++`, `Stage 1`, `Stage 2`, `trap`, `seed`, `verified`. All rows
are loaded for every C++ Pilot (tag `lang:cpp`), except those with a tag in the `C++` column.

## Q. Classes and values

| id  | C++                                                      | Stage 1                                                                         | Stage 2                                                   | trap                                                                                            | seed         | verified |
|-----|----------------------------------------------------------|---------------------------------------------------------------------------------|-----------------------------------------------------------|-------------------------------------------------------------------------------------------------|--------------|----------|
| X1  | `void*` + type tag as a hand-made tagged union           | `struct` with `*mut c_void` and a `#[repr]` tag enum, cast only on the arm      | `enum` with one variant per tag                           | a read through the wrong `T` is UB in C++ and Rust: record a Deviation, never emulate the bytes | CPP7, CPP9   | no       |
| X2  | `new T(v)` / `delete static_cast<T*>(p)`                 | `Box::into_raw` / `Box::from_raw`, `// SAFETY:` names the tag                   | owned enum payload                                        | a null payload (`null` value) must never reach `Box::from_raw`                                  | CPP8         | no       |
| X3  | overloaded constructors                                  | one named constructor per overload (`new`, `from_<type>`), names in RULEBOOK §5 | `Default` and `From<T>` impls                             | overload resolution depends on the compiler and the platform (`long` on LP64); list the cases   | CPP10        | no       |
| X4  | destructor `~T` with a switch over a tag                 | `impl Drop` with the same switch                                                | derived drop                                              | recursion depth on deep values; a leak in the original (missing `delete`) is a Deviation        | CPP13        | no       |
| X5  | copy constructor / `operator=`                           | `impl Clone`; assignment as a method that drops the old payload                 | `#[derive(Clone)]`, plain `a = b.clone()`                 | self-assignment; an original that leaks the old payload on assign                               | CPP14, CPP15 | no       |
| X6  | rule of three without move operations                    | Rust moves                                                                      | same                                                      | every C++ "move" is a deep copy; only allocation counts differ                                  | CPP16        | no       |
| X7  | `operator==` / `!=` with fallthrough or asymmetric cases | explicit chain in C++ case order; `impl PartialEq` delegating to it             | same semantics; never `#[derive(PartialEq)]`              | the relation may be non-symmetric: keep it, mark `QUIRK(port)`                                  | CPP17        | no       |
| X8  | `operator[]` that throws on a bad key or index           | methods returning `Result<&mut T, Error>`                                       | `get_*` methods returning `Result`; no `Index`/`IndexMut` | `Index` cannot return `Err`; a null-pointer vs `int` overload resolves to `int`                 | CPP18        | no       |
| X9  | implicit `operator T()` conversions                      | `operator_<type>() -> Result<T, Error>`                                         | `TryFrom<&T>`                                             | implicit conversions make caller code ambiguous; each operator accepts only its exact type      | CPP19        | no       |
| X10 | `const` by-value or by-reference return of a member      | drop the `const`; return a clone                                                | borrow (`&T`) where the caller cannot mutate              | a borrow also forbids mutation, so the observable result is the same                            | CPP20        | no       |
| X11 | static-only class (`parser::parse`)                      | `pub struct P;` + `impl P` with the same function names                         | same                                                      | tests call the private-in-spirit static functions directly: keep them `pub`                     | CPP29        | no       |
| X12 | unscoped `enum` leaking enumerators                      | `#[repr(i32)] enum` with explicit discriminants                                 | Rust-named enum, same discriminants                       | the enumerators are visible in the namespace; printing one gives the integer                    | CPP6, CPP25  | no       |

## R. Errors and exceptions

| id  | C++                                                       | Stage 1                                                                        | Stage 2         | trap                                                                                           | seed  | verified |
|-----|-----------------------------------------------------------|--------------------------------------------------------------------------------|-----------------|------------------------------------------------------------------------------------------------|-------|----------|
| X13 | `throw std::runtime_error(msg)` and friends               | `Result<T, Error>` and `?`; `Error { kind, what: Vec<u8> }`                    | same            | the `what()` text is observable (tests match it) and may carry user bytes: bytes, not `String` | CPP21 | no       |
| X14 | an exception from the standard library that is not caught | `Err` with the std exception class as `kind` (`InvalidArgument`, `OutOfRange`) | same            | the class and the text are libstdc++ vs libc++ specific: the Reference build decides           | CPP33 | no       |
| X15 | `default:` arm that handles an impossible tag             | `unreachable!()` with an invariant comment                                     | no such variant | prove it unreachable (RULEBOOK §7) or it is a precondition guard                               | CPP14 | no       |
| X16 | `new` that can throw `std::bad_alloc`                     | not modelled                                                                   | not modelled    | Deviation row (as allocation-guard, C table A4)                                                | —     | no       |
| X17 | function-like macro that builds a message and throws      | `fn` returning `Err`                                                           | same            | the macro may be predefined by the user (RULEBOOK §3 row); operand types print as integers     | CPP5  | no       |

## S. Standard library

| id  | C++                                                          | Stage 1                                                              | Stage 2 | trap                                                                                       | seed         | verified |
|-----|--------------------------------------------------------------|----------------------------------------------------------------------|---------|--------------------------------------------------------------------------------------------|--------------|----------|
| X18 | `std::string` operations                                     | `Vec<u8>` / `&[u8]` methods                                          | same    | `.c_str()` in a stream insertion cuts at the first NUL; bytes, not UTF-8                   | CPP22        | no       |
| X19 | `std::vector<T>`                                             | `Vec<T>`                                                             | same    | `==` compares sizes, then elements with `T::operator==`: same as `Vec` once X7 is in place | CPP23        | no       |
| X20 | `std::map<K, V>`                                             | `BTreeMap<K, V>`; `m[k] = v` → `insert`                              | same    | `m[k]` inserts a default on read; order is `char_traits<char>::lt` (unsigned bytes)        | CPP24        | no       |
| X21 | `std::ostringstream` / `stringstream` text building          | push into `Vec<u8>`                                                  | same    | an unscoped enum prints as an integer                                                      | CPP25        | no       |
| X22 | `std::to_string(double)`                                     | the C `%f` formatter (C table I7)                                    | same    | not shortest round-trip: `%f`, six decimals                                                | CPP32        | no       |
| X23 | `std::stoll` / `std::stod` with `&pos`                       | scanners returning value and `pos`, or `Err`                         | same    | leading whitespace and partial parses; exception classes differ (X14)                      | CPP33        | no       |
| X24 | `std::wstring_convert` / `codecvt_utf8<char32_t>`            | hand-written UTF-8 encoder and decoder with the library's error text | same    | deprecated in C++17; behaviour at chunk boundaries and surrogates is library specific      | CPP27, CPP28 | no       |
| X25 | `std::transform` with a lambda, iterator loops with `next()` | `for` loop over the bytes / `iter().enumerate()`                     | same    | `std::tolower` is locale dependent                                                         | CPP30, CPP31 | no       |
| X26 | C-style cast of `EOF` to an unsigned type                    | explicit `as`                                                        | same    | `(char32_t)EOF` is `0xFFFFFFFF`, the stream's EOF                                          | CPP34        | no       |

## H. Header-only layout

| id  | C++                                                         | Stage 1                                               | Stage 2       | trap                                                                              | seed       | verified |
|-----|-------------------------------------------------------------|-------------------------------------------------------|---------------|-----------------------------------------------------------------------------------|------------|----------|
| X27 | a header-only library (`#pragma once`, everything `inline`) | one Part, `rust/src/lib.rs`; `pub` for the public API | same          | missing includes the header relied on (`<locale>`): the Reference build adds them | CPP1, CPP2 | no       |
| X28 | `namespace n { ... }`                                       | the crate root (crate name `n`)                       | same          | nested namespaces become modules only when the public API names them              | CPP3       | no       |
| X29 | `using` aliases of containers                               | `pub type`                                            | named aliases | none                                                                              | CPP4       | no       |

## Not covered (no Pilot has used them yet)

Add a row when the first Pilot does: templates of its own (instantiate and map, or generic with trait bounds),
inheritance and `virtual` functions (trait objects or enum dispatch), `friend`, `mutable`, `static` data members,
smart pointers (`unique_ptr`, `shared_ptr`), `std::move` semantics beyond X6, user-defined literals, RTTI, `goto`,
varargs, global constructors, multiple inheritance, threads.
