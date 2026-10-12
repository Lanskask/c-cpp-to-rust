# Rulebook template: C++ branch

Use with `rulebook-template.md` for a Pilot run with `--lang cpp`. Copy the base template, then **replace** the
sections below; every other section stays as written. Idiom table: `idiom-table-c.md` for the C subset plus
`idiom-table-cpp.md`. Worked example: `pilots/tinyjson/migration/RULEBOOK.md` (status `drafted` at the time of
writing; its §2.1 seeded the C++ table).

The base template assumes a C library. These are the places where that is wrong for C++:

| Base section         | Why it does not fit C++                                        | Replace with                                    |
|----------------------|----------------------------------------------------------------|-------------------------------------------------|
| §0 Scope and posture | demands a C ABI and `#[repr(C)]` on every public symbol        | the C++ posture below                           |
| §2 Idiom overrides   | only the C table exists                                        | rows of `idiom-table-cpp.md` as `X<n>`          |
| §3 Configuration     | assumes `config-macro:` tags; C++ headers have user macros too | a row per user-overridable macro, found by hand |
| §5 Naming and paths  | no scheme for overloads, operators, conversions                | the naming table below                          |
| §7 Error model       | C error codes and `NULL`                                       | the exception model below                       |
| §8 Deviations        | no default rule                                                | the default rule (also in the base template)    |
| §10 Judge            | "original tests linked to the staticlib"                       | the reference build and the C++ Judge           |

## 0. Scope and posture (C++)

- **No C ABI in either Stage** (decision 5): the original tests cannot link to a Rust crate. Stage 1 exports a Rust API
  that keeps the C++ names and structure; the Judge reaches it through the adapter (`judge.md` §5).
- Stage 1 (Faithful port): same structure and names, `unsafe` allowed, engine LLM (c2rust does not translate C++).
- Stage 2 (Idiomatic pass): the API in §6, zero `unsafe`. Same behaviour, not same API.
- Reference behaviour: the original built with the **Reference build** below, nothing else.

## Reference build (new slot, filled in ticket 01)

| Item                 | Value                                                                                   |
|----------------------|-----------------------------------------------------------------------------------------|
| Compiler and version | [the compiler that builds the original header and tests, from `<cxx> --version`]        |
| Standard library     | [libstdc++ or libc++, and its version]                                                  |
| Standard and flags   | [`-std=c++NN`, `-include ...`, warnings; every flag the original needs to compile]      |
| Optimisation level   | [`-O0` or `-O2`: the level of the original **driver** and of the translated-test build] |
| Patches to upstream  | [file, line, reason, one row each; none may change an expected value]                   |
| Why this build       | [measured: which default compiler fails and with what message]                          |

The level matters: an uninitialised read or a signed overflow can differ between `-O0` and `-O2`, so a Deviation
about it is meaningless without the level. Every Judge piece builds the original with this row, never with `c++`.

## 5. Naming (C++)

| C++ construct            | Stage 1 name                                  | Stage 2 name                      |
|--------------------------|-----------------------------------------------|-----------------------------------|
| constructor overload set | `new`, `from_<param type>` (one per overload) | `Default` and `From<T>` impls     |
| `operator==`, `!=`       | `eq` (and `PartialEq`), `ne`                  | `PartialEq`                       |
| `operator[]`             | `index_<arg type>`                            | `get_*` returning `Result`        |
| `operator T()`           | `operator_<type>`                             | `TryFrom<&Self> for T`            |
| namespace `n`            | crate root, crate name `n`                    | same                              |
| overloaded free function | `<name>_<param types>`                        | trait or generic when it is clean |

Every row of a Pilot's overload set is listed in §6 or §5 with its Rust name. `mapping.tsv` (when it exists) uses
these names.

## 7. Error model (C++)

- A `throw` becomes `Err(Error { kind, what })`; `kind` keeps the exception class (`RuntimeError`, `InvalidArgument`,
  `OutOfRange`, ...), `what` is the exact `what()` bytes.
- Classify every `throw` site: **precondition-guard** (kept, same message), **unreachable** (a tag or state no
  constructor builds: `unreachable!()` with an invariant comment, proved or reasoned in this section) or
  **allocation-guard** (`bad_alloc`: a Deviation row).
- An exception that the library does not catch but the standard library throws (`std::stoll`) is a row too: class and
  text come from the Reference build's standard library.
- State after an exception (strong guarantee or not) is stated per function that mutates `*this` before it can throw.

## 8. Deviations: default rule

**The Rust result matches the C++ origin as closely as possible.** A Deviation proposal must first state what the
Reference build does (observed, with the optimisation level), and the default decision is *kept bug*: the Rust code
reproduces the original, marked `BUG(port)` or `QUIRK(port)`. A fix is proposed only when matching is impossible
(undefined behaviour with no single result, a platform dependence) or the user asks for it.

## 10. Judge (C++)

- Stage 1: cargo check/test, the Differential harness against the original C++ driver built with the Reference build,
  translated upstream tests (`judge.md` §1, C++ column). There are no original tests linked to the crate.
- Stage 2: as the base template.
- Feature matrix: one row per user macro of §3 that the upstream tests set.
