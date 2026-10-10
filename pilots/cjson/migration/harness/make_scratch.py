#!/usr/bin/env python3
"""Generate the SCRATCH Rust crates the Judge is validated on (ticket 02). Not the accepted target `rust/`.

The Judge links the upstream tests to a Rust crate, but the crate does not exist until Stage 1 (ticket 04).
To prove the Judge works (links, passes, can fail) this script transpiles the pinned C with c2rust once per
feature set and writes one crate per set under build/scratch/<set>/ (git-ignored):
    default  -> no macros
    locales  -> -DENABLE_LOCALES
Each scratch crate declares the cargo features `locales` (no-op: the macro is baked in by c2rust, which
works after the preprocessor, decision 24) and `test-internals` (makes the static items the upstream tests
need `pub` + `#[no_mangle]`, RULEBOOK §3/§10).

Usage: python3 -I make_scratch.py [--force]
Prints every c2rust workaround and hand tweak applied, with counts (copied into RUN-NOTES.md).
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SRC = HERE.parent.parent / "src"
OUT = HERE / "build" / "scratch"
C2RUST = Path.home() / ".cargo" / "bin" / "c2rust"
SETS: dict[str, list[str]] = {"default": [], "locales": ["-DENABLE_LOCALES"]}

# static functions of cJSON.c that the upstream tests call (grep over tests/*.c, see RUN-NOTES.md)
INTERNAL_FNS = [
    "parse_number", "parse_hex4", "parse_string", "parse_value", "parse_array", "parse_object",
    "print_number", "print_string_ptr", "print_value", "print_array", "print_object", "ensure",
    "skip_utf8_bom", "add_item_to_array", "cJSON_strdup", "compare_double", "print_string", "print",
    "buffer_skip_whitespace", "update_offset", "get_object_item", "get_array_item", "suffix_object",
    "create_reference", "cJSON_New_Item", "add_item_to_object", "replace_item_in_object",
]
INTERNAL_STATICS = ["global_hooks", "global_error"]

CARGO_TOML = """[package]
name = "cjson_scratch_{name}"
version = "0.0.0"
edition = "2021"
publish = false
description = "SCRATCH c2rust output for the cJSON Judge validation (feature set: {label}); not the accepted crate"

[lib]
name = "cjson_scratch_{name}"
path = "lib.rs"
crate-type = ["staticlib", "rlib"]

[features]
default = []
# ENABLE_LOCALES is baked in by c2rust for this crate; the feature exists so the Judge can pass the same name.
locales = []
# makes the static items the upstream tests need `pub` + `#[no_mangle]`
test-internals = []

[workspace]
"""

LIB_RS = """#![allow(dead_code, non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(unused_assignments, unused_mut, unused_variables, unused_unsafe, static_mut_refs)]
#![allow(clippy::all)]

pub mod src {
    pub mod cJSON;
    pub mod cJSON_Utils;
}
"""


def log(msg: str) -> None:
    print(msg)


def transpile(name: str, defines: list[str]) -> Path:
    work = OUT / f"{name}-c2rust"
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir(parents=True)
    entries = [
        {
            "directory": str(work),
            "file": str(SRC / f),
            "arguments": ["clang", "-c", "-I", str(SRC), *defines, str(SRC / f), "-o", f"{Path(f).stem}.o"],
        }
        for f in ("cJSON.c", "cJSON_Utils.c")
    ]
    (work / "compile_commands.json").write_text(json.dumps(entries, indent=2))  # exact name required
    crate = work / "cjson_c2rust"  # output dir name becomes the package name; never `crate`
    r = subprocess.run(
        [str(C2RUST), "transpile", "-e", "-o", str(crate), str(work / "compile_commands.json")],
        capture_output=True, text=True,
    )
    (work / "transpile.log").write_text(f"exit {r.returncode}\n--- stdout\n{r.stdout}\n--- stderr\n{r.stderr}")
    if r.returncode:
        print(r.stderr[-2000:], file=sys.stderr)
        raise SystemExit(f"c2rust failed for {name}")
    return crate


def patch(name: str, label: str, crate: Path) -> Path:
    dest = OUT / name
    shutil.rmtree(dest, ignore_errors=True)
    (dest / "src").mkdir(parents=True)
    (dest / "Cargo.toml").write_text(CARGO_TOML.format(name=name, label=label))
    (dest / "lib.rs").write_text(LIB_RS)
    log(f"[{label}] tweak T1: Cargo.toml and lib.rs rewritten (c2rust's lib.rs has `#![feature(linkage)]`,"
        " `#![feature(raw_ref_op)]` and `extern crate f128`: nightly-only, not needed; builds on stable)")
    for f in ("cJSON.rs", "cJSON_Utils.rs"):
        text = (crate / "src" / f).read_text()
        before = text.count("f128")
        text = re.sub(r"^use ::f128;\n", "", text, flags=re.M)
        # macOS math.h `isnan`/`isinf` expand to `__inline_isnanl(long double)`; on aarch64 macOS long double is
        # f64, so the f128 crate is replaced by f64 (same value, same semantics)
        text, n_ty = re.subn(r"::f128::f128::new\(([^()]*)\)", r"\1", text)
        text, n_ty2 = re.subn(r"::f128::f128\b", "f64", text)
        text, n_link = re.subn(r'^#\[linkage = "external"\]\n', "", text, flags=re.M)
        log(f"[{label}] tweak T2 ({f}): removed `use ::f128;`; f128::new(x) -> x: {n_ty}, `::f128::f128` -> f64:"
            f" {n_ty2}; f128 occurrences {before} -> {text.count('f128')}")
        log(f"[{label}] tweak T5 ({f}): removed {n_link} `#[linkage = \"external\"]` attributes (nightly-only)")
        # macOS ctype.h/math.h `static inline` helpers (tolower, __inline_isnanl, ...) come out as exported
        # `#[no_mangle]` functions: they would shadow libc's tolower and collide between the two modules
        text, n_nm = re.subn(
            r'^#\[no_mangle\]\n(?=(?:#\[[^\n]*\]\n)*(?:pub )?unsafe extern "C" fn (?:tolower|__inline_\w+)\()',
            "", text, flags=re.M,
        )
        log(f"[{label}] tweak T6 ({f}): removed `#[no_mangle]` from {n_nm} inline libc helpers (tolower, __inline_*)")
        if f == "cJSON.rs":
            n_fn = 0
            for fn in INTERNAL_FNS:
                text, k = re.subn(
                    rf'^unsafe extern "C" fn {fn}\(',
                    f'#[cfg_attr(feature = "test-internals", no_mangle)]\npub unsafe extern "C" fn {fn}(',
                    text, flags=re.M,
                )
                n_fn += k
            n_st = 0
            for st in INTERNAL_STATICS:
                text, k = re.subn(
                    rf"^(?:pub )?static mut {st}\b",
                    f'#[cfg_attr(feature = "test-internals", no_mangle)]\npub static mut {st}',
                    text, flags=re.M,
                )
                n_st += k
            log(f"[{label}] tweak T3 (cJSON.rs): test-internals: {n_fn} static fns and {n_st} statics made"
                " `pub` + `#[cfg_attr(feature = \"test-internals\", no_mangle)]`")
        (dest / "src" / f).write_text(text)
    return dest


def main() -> int:
    if not C2RUST.exists() or not SRC.is_dir():
        print("make_scratch: need c2rust and pilots/cjson/src", file=sys.stderr)
        return 1
    OUT.mkdir(parents=True, exist_ok=True)
    for name, defines in SETS.items():
        label = " ".join(defines) or "no macros"
        crate = transpile(name, defines)
        n = sum(len(p.read_text().splitlines()) for p in (crate / "src").glob("*.rs"))
        log(f"[{label}] c2rust 0.22.1 transpiled cJSON.c + cJSON_Utils.c: {n} Rust lines")
        patch(name, label, crate)
    return 0


if __name__ == "__main__":
    sys.exit(main())
