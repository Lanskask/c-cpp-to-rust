#!/usr/bin/env python3
"""Assign each Part a tier (mechanical, judgment, unknown) and write the parts.tsv skeleton.

Inputs: inventory.tsv, order.txt (from depmap.py; same directory as edges.tsv). Part = files sharing
directory and stem. Test and example files are not Parts (tests are translated separately).
Tiers, from the risky-construct tags of the Part's symbols:
  unknown    setjmp (non-local control flow: ask the user or stub)
  judgment   virtual template goto union varargs macro-fn static-global errno malloc fnptr bitfield float-format
             strtod qsort asm signal thread, or any config-macro
             (needs a cargo feature, decision 24)
  mechanical everything else (POD, enums, constants, pure helpers)
`target_path`: a single Part is `rust/src/lib.rs`; otherwise `rust/src/<stem>.rs`, and Parts that share a
stem get a module name derived from the path (`a/util` -> `a_util.rs`). `--crate-root-part NAME` makes the
named Part `lib.rs` in a multi-Part crate (the orchestrator must then declare the other modules).
`deps` come from edges.tsv. Rows are written in leaf-first order, state=todo, attempts=0.
Exit code 0 on success, 1 on failure.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

HEADER = "part_id\tkind\tsource_paths\ttarget_path\ttier\tdeps\tstate\tattempts\tmodel\tlast_error\tcommit\tupdated"
JUDGMENT = {
    "virtual", "template", "goto", "union", "varargs", "macro-fn", "static-global", "errno", "malloc",
    "fnptr", "bitfield", "float-format", "strtod", "qsort", "asm", "signal", "thread",
}  # fmt: skip
# `ctype` and `long` stay mechanical: their idiom rows (L4-L6, T1) are plain mappings
TEST_DIRS = {"test", "tests", "unittest", "unittests"}
EXAMPLE_DIRS = {"example", "examples", "sample", "samples", "demo", "bench", "benchmark"}


def is_excluded(rel: str) -> bool:
    p = Path(rel)
    return bool((TEST_DIRS | EXAMPLE_DIRS) & set(p.parts[:-1])) or p.stem.lower().startswith("test")


def part_of(rel: str) -> str:
    return str(Path(rel).with_suffix(""))


def tier_of(tags: set[str]) -> str:
    if "setjmp" in tags:
        return "unknown"
    if tags & JUDGMENT or any(t.startswith("config-macro:") for t in tags):
        return "judgment"
    return "mechanical"


def target_paths(parts: list[str], crate_root: str | None) -> dict[str, str]:
    """Map each Part id to its Rust file (G12). Raises ValueError if crate_root names no Part."""
    if crate_root is not None and crate_root not in parts:
        raise ValueError(f"--crate-root-part {crate_root!r} is not a Part")
    if len(parts) == 1:
        return {parts[0]: "rust/src/lib.rs"}
    stems = [Path(p).name for p in parts]
    out: dict[str, str] = {}
    for p, stem in zip(parts, stems, strict=True):
        if p == crate_root:
            out[p] = "rust/src/lib.rs"
        else:
            name = re.sub(r"\W", "_", p) if stems.count(stem) > 1 else stem
            out[p] = f"rust/src/{name}.rs"
    return out


def main_logic(
    inv: Path, order: Path, edges: Path | None, include_tests: bool, crate_root: str | None = None
) -> list[list[str]]:
    tags: dict[str, set[str]] = {}
    files: dict[str, set[str]] = {}
    for ln in inv.read_text().splitlines():
        if not ln or ln.startswith(("#", "symbol\t")):
            continue
        cells = ln.split("\t")
        file = cells[2]
        if not include_tests and is_excluded(file):
            continue
        p = part_of(file)
        files.setdefault(p, set()).add(file)
        tags.setdefault(p, set()).update(t for t in (cells[4].split(",") if len(cells) > 4 else []) if t)
    deps: dict[str, set[str]] = {p: set() for p in files}
    if edges and edges.is_file():
        for ln in edges.read_text().splitlines()[1:]:
            a, b, kind = ln.split("\t")
            pa, pb = (
                (part_of(a), part_of(b)) if kind == "include" else (part_of(a.split("@")[1]), part_of(b.split("@")[1]))
            )
            if pa in deps and pb in deps and pa != pb:
                deps[pa].add(pb)
    ordered: list[str] = []
    for ln in order.read_text().splitlines():
        ordered += [p for p in ln.split() if p in files]
    ordered += sorted(set(files) - set(ordered))
    targets = target_paths(ordered, crate_root)
    rows = []
    for p in ordered:
        src = ",".join(sorted(files[p]))
        target = targets[p]
        kind = "file"
        rows.append([p, kind, src, target, tier_of(tags[p]), ",".join(sorted(deps[p])), "todo", "0", "", "", "", ""])
    return rows


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("inventory")
    p.add_argument("order")
    p.add_argument("--edges", help="edges.tsv (default: next to order.txt)")
    p.add_argument("--include-tests", action="store_true", help="also make Parts of test/example files")
    p.add_argument(
        "--crate-root-part",
        metavar="NAME",
        help="Part id that becomes rust/src/lib.rs in a multi-Part crate (default: none; "
        "the orchestrator must declare the other modules, e.g. `mod x;` in lib.rs)",
    )
    p.add_argument("--out", default="migration/parts.tsv")
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    inv, order = Path(args.inventory), Path(args.order)
    if not inv.is_file() or not order.is_file():
        print("tier: inventory or order file missing", file=sys.stderr)
        return 1
    edges = Path(args.edges) if args.edges else order.parent / "edges.tsv"
    try:
        rows = main_logic(inv, order, edges, args.include_tests, args.crate_root_part)
    except ValueError as e:
        print(f"tier: {e}", file=sys.stderr)
        return 1
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(HEADER + "\n" + "".join("\t".join(r) + "\n" for r in rows))
    counts = {t: sum(r[4] == t for r in rows) for t in ("mechanical", "judgment", "unknown")}
    print(f"tier: {len(rows)} part(s) {counts} -> {out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
