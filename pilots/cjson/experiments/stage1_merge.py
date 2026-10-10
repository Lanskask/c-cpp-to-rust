#!/usr/bin/env python3
"""Stage 1 (ticket 04): merge the two c2rust outputs (feature sets "" and `locales`) into the one crate `rust/`.

Question: how do the c2rust outputs of cJSON for `""` and `-DENABLE_LOCALES` differ, and can they be merged into one
source with a real `locales` cargo feature without touching any function body?

Answer: yes. After removing c2rust's `current_block = <random 64-bit id>` numbers (renumbered per run), the two
`cJSON.rs` files differ in exactly three places (`localeconv` declaration, `struct lconv`, body of
`get_decimal_point`) and the two `cJSON_Utils.rs` files do not differ. The merge keeps the `""` output and wraps those
three places in `#[cfg(feature = "locales")]` / `#[cfg(not(feature = "locales"))]`. It asserts this and refuses to
write anything otherwise.

Input: the tweaked scratch crates made by `migration/harness/make_scratch.py` (tweaks T1-T6 of RUN-NOTES.md).
Output: `rust/Cargo.toml`, `rust/src/lib.rs`, `rust/src/cJSON.rs`, `rust/src/cJSON_Utils.rs`.

Usage: python3 -I stage1_merge.py
"""

from __future__ import annotations

import difflib
import re
import sys
from pathlib import Path

PILOT = Path(__file__).resolve().parent.parent
SCRATCH = PILOT / "migration" / "harness" / "work" / "scratch"
OUT = PILOT / "rust"
ID = re.compile(r"\b\d{15,}\b")

CARGO_TOML = """[package]
name = "cjson"
version = "0.0.0"
edition = "2021"
publish = false
description = "cJSON 1.7.19 (6d9f244), Stage 1 faithful port: c2rust output, C ABI, one cargo feature per C macro"

[lib]
name = "cjson"
path = "src/lib.rs"
crate-type = ["staticlib", "rlib"]

[features]
default = []
# ENABLE_LOCALES: decimal point from localeconv() (cJSON.c:48-50,281-286); RULEBOOK section 3
locales = []
# Stage 1 only: makes the static items the upstream tests need `pub` + `#[no_mangle]` (RULEBOOK sections 3, 10)
test-internals = []

[workspace]
"""

LIB_RS = """#![allow(dead_code, non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(unused_assignments, unused_mut, unused_variables, unused_unsafe, static_mut_refs)]
#![allow(clippy::all)]

pub mod cJSON;
pub mod cJSON_Utils;
"""

CFG_ON = '#[cfg(feature = "locales")]'
CFG_OFF = '#[cfg(not(feature = "locales"))]'


def norm(lines: list[str]) -> list[str]:
    return [ID.sub("ID", ln) for ln in lines]


def merge_cjson(default: list[str], locales: list[str]) -> list[str]:
    sm = difflib.SequenceMatcher(None, norm(default), norm(locales), autojunk=False)
    ops = [op for op in sm.get_opcodes() if op[0] != "equal"]
    kinds = [(tag, "".join(locales[j1:j2])[:60]) for tag, _, _, j1, j2 in ops]
    if len(ops) != 3 or [o[0] for o in ops] != ["insert", "insert", "replace"]:
        raise SystemExit(f"stage1_merge: unexpected differences between the c2rust outputs: {kinds}")
    (_, a1, a2, b1, b2), (_, c1, c2, d1, d2), (_, e1, e2, f1, f2) = ops
    decl, strukt = locales[b1:b2], locales[d1:d2]
    body_c = default[e1:e2]
    body_l = locales[f1:f2]
    ok = (
        len(decl) == 1 and decl[0].strip() == "fn localeconv() -> *mut lconv;"
        and [x.strip() for x in strukt[:3]] == ["#[derive(Copy, Clone)]", "#[repr(C)]", "pub struct lconv {"]
        and strukt[-1] == "}\n"
        and len(body_c) == 1 and "'.'" in body_c[0] and "localeconv()" in body_l[0]
    )
    if not ok:
        raise SystemExit("stage1_merge: the three hunks are not the expected locales hunks")
    out: list[str] = []
    i = 0
    while i < len(default):
        if i == a1:
            out.append(f"    {CFG_ON}\n")
            out += decl
        if i == c1:
            out.append(CFG_ON + "\n")
            out += strukt
        if i == e1:
            out.append(f"    {CFG_ON}\n    {{\n")
            out += ["    " + ln for ln in body_l]
            out.append("    }\n")
            out.append(f"    {CFG_OFF}\n    {{\n")
            out += ["    " + ln for ln in body_c]
            out.append("    }\n")
            i = e2
            continue
        out.append(default[i])
        i += 1
    return out


def main() -> int:
    parts = {}
    for f in ("cJSON.rs", "cJSON_Utils.rs"):
        d = (SCRATCH / "default" / "src" / f).read_text().splitlines(keepends=True)
        l = (SCRATCH / "locales" / "src" / f).read_text().splitlines(keepends=True)
        parts[f] = (d, l)
    du, lu = parts["cJSON_Utils.rs"]
    if norm(du) != norm(lu):
        raise SystemExit("stage1_merge: cJSON_Utils.rs differs between the feature sets")
    merged = {"cJSON.rs": merge_cjson(*parts["cJSON.rs"]), "cJSON_Utils.rs": du}
    (OUT / "src").mkdir(parents=True, exist_ok=True)
    (OUT / "Cargo.toml").write_text(CARGO_TOML)
    (OUT / "src" / "lib.rs").write_text(LIB_RS)
    for f, lines in merged.items():
        (OUT / "src" / f).write_text("".join(lines))
        print(f"wrote rust/src/{f}: {len(lines)} lines")
    return 0


if __name__ == "__main__":
    sys.exit(main())
