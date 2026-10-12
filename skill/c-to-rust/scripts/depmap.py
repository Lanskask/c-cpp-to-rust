#!/usr/bin/env python3
"""Build include/call/inheritance edges, strongly connected components and a leaf-first Part order.

Input: inventory.tsv (from inventory.py) and the source root. Outputs in --out-dir:
  edges.tsv   src, dst, kind (include|call|uses|inherit); `include` is file->file, the rest symbol->symbol
              as `name@file`
  cycles.txt  one line per cycle of Parts (SCC with more than one Part); empty file if none
  order.txt   leaf-first: dependencies before dependents; a line with several Parts is a cycle to translate together
  dead.txt    candidate dead code: functions not reachable from tests or from any non-static library function
              that tests reach; examples and unused helpers show up here
A Part is a group of files sharing directory and stem (jsmn.h + jsmn.c = Part `jsmn`).
Roots for dead-code analysis are symbols in test files (path has a test/tests component or a name starting
with `test`) and in example/sample/demo directories. Do not --exclude test/ or samples/ in inventory.py: without
their symbols a header-only library has no caller of its public API and every function is reported dead.
Call edges are name-based (a call to a name defined once in the project); ambiguous names are
resolved to the same file first and skipped otherwise, so the graph under-approximates for overloads.
Exit code 0 on success, 1 on failure.
"""

from __future__ import annotations

import argparse
import posixpath
import re
import sys
from collections import defaultdict
from pathlib import Path

INCLUDE = re.compile(r'^\s*#\s*include\s+"([^"]+)"', re.M)
INHERIT = re.compile(r"\b(?:class|struct)\s+(\w+)\s*(?:final\s*)?:\s*([^{;]+)\{")
IDENT = re.compile(r"[A-Za-z_]\w*")
CALL = re.compile(r"\b([A-Za-z_]\w*)\s*\(")
TEST_DIRS = {"test", "tests", "unittest", "unittests"}
EXAMPLE_DIRS = {"example", "examples", "sample", "samples", "demo"}
# same list as inventory.py: vendored or non-library directories never take part in the graph
VENDORED_DIRS = {"fuzzing", "fuzz", "third_party", "thirdparty", "vendor", "external", "unity", ".git", "build"}

Sym = tuple[str, str, str, int, int, str]  # name, kind, file, start, end, tags


def read_inventory(path: Path) -> list[Sym]:
    rows: list[Sym] = []
    for ln in path.read_text().splitlines():
        if not ln or ln.startswith("#") or ln.startswith("symbol\t"):
            continue
        name, kind, file, lines, *rest = ln.split("\t")
        a, b = lines.split("-")
        rows.append((name, kind, file, int(a), int(b), rest[0] if rest else ""))
    return rows


def is_test_file(rel: str) -> bool:
    p = Path(rel)
    return bool(TEST_DIRS & set(p.parts[:-1])) or p.stem.lower().startswith("test")


def is_root_file(rel: str) -> bool:
    """Tests and examples are the callers of a library's public API: roots of the dead-code walk."""
    return is_test_file(rel) or bool(EXAMPLE_DIRS & set(Path(rel).parts[:-1]))


def part_of(rel: str) -> str:
    return str(Path(rel).with_suffix(""))


def strip_comments(text: str) -> str:
    text = re.sub(r"/\*.*?\*/", lambda m: re.sub(r"[^\n]", " ", m.group(0)), text, flags=re.S)
    return re.sub(r"//[^\n]*", "", text)


def include_edges(root: Path, files: set[str]) -> list[tuple[str, str, str]]:
    by_name: dict[str, list[str]] = defaultdict(list)
    for f in files:
        by_name[Path(f).name].append(f)
    edges = []
    for f in sorted(files):
        text = strip_comments((root / f).read_text(errors="replace"))
        for inc in INCLUDE.findall(text):
            norm = posixpath.normpath(posixpath.join(posixpath.dirname(f), inc))  # relative to the includer
            if norm not in files:
                same_name = by_name.get(Path(inc).name, [])
                norm = same_name[0] if len(same_name) == 1 else ""
            if norm and norm != f:
                edges.append((f, norm, "include"))
    return edges


def symbol_edges(root: Path, syms: list[Sym]) -> list[tuple[str, str, str]]:
    funcs: dict[str, list[Sym]] = defaultdict(list)
    types: dict[str, list[Sym]] = defaultdict(list)
    for s in syms:
        if s[1] == "function":
            funcs[s[0].rsplit("::", 1)[-1]].append(s)  # call site: `get_string`; symbol: `json::get_string`
        elif s[1] in ("record", "typedef", "enum"):
            types[s[0]].append(s)
    cache: dict[str, list[str]] = {}

    def lines_of(file: str) -> list[str]:
        if file not in cache:
            cache[file] = strip_comments((root / file).read_text(errors="replace")).split("\n")
        return cache[file]

    def ref(s: Sym) -> str:
        return f"{s[0]}@{s[2]}"

    def resolve(cands: list[Sym], here: str) -> Sym | None:
        if len(cands) == 1:
            return cands[0]
        same = [c for c in cands if c[2] == here]
        return same[0] if len(same) == 1 else None

    edges: list[tuple[str, str, str]] = []
    for s in syms:
        if s[1] != "function":
            continue
        body = "\n".join(lines_of(s[2])[s[3] - 1 : s[4]])
        for name in sorted(set(CALL.findall(body)) - {s[0].rsplit("::", 1)[-1]}):
            t = resolve(funcs.get(name, []), s[2])
            if t:
                edges.append((ref(s), ref(t), "call"))
        for name in sorted(set(IDENT.findall(body))):
            t = resolve(types.get(name, []), s[2]) if name in types else None
            if t:
                edges.append((ref(s), ref(t), "uses"))
    for f in sorted({s[2] for s in syms}):
        for derived, bases in INHERIT.findall("\n".join(lines_of(f))):
            for b in re.findall(r"\b(\w+)\s*(?:,|$)", re.sub(r"\b(public|protected|private|virtual)\b", "", bases)):
                if b in types and derived in types:
                    d = resolve(types[derived], f)
                    t = resolve(types[b], f)
                    if d and t:
                        edges.append((ref(d), ref(t), "inherit"))
    return edges


def tarjan(graph: dict[str, set[str]]) -> list[list[str]]:
    """Strongly connected components, returned dependencies-first (reverse topological of the condensation)."""
    index: dict[str, int] = {}
    low: dict[str, int] = {}
    on: set[str] = set()
    stack: list[str] = []
    out: list[list[str]] = []
    counter = 0

    def visit(v: str) -> None:
        nonlocal counter
        index[v] = low[v] = counter
        counter += 1
        stack.append(v)
        on.add(v)
        for w in sorted(graph.get(v, ())):
            if w not in index:
                visit(w)
                low[v] = min(low[v], low[w])
            elif w in on:
                low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            comp = []
            while True:
                w = stack.pop()
                on.discard(w)
                comp.append(w)
                if w == v:
                    break
            out.append(sorted(comp))

    sys.setrecursionlimit(10000)
    for v in sorted(graph):
        if v not in index:
            visit(v)
    return out


def implicit_callee(name: str) -> bool:
    """Operators, constructors and destructors are invoked by syntax, never by a name-based call: not dead evidence."""
    if re.search(r"(^|::)(operator\b|operator(?=[^\w\s])|~)", name):  # `A::operator std::string` holds a `::`
        return True
    *scope, last = name.split("::")
    return bool(scope) and scope[-1] == last


def dead_candidates(syms: list[Sym], edges: list[tuple[str, str, str]]) -> list[str]:
    ref = lambda s: f"{s[0]}@{s[2]}"  # noqa: E731
    adj: dict[str, set[str]] = defaultdict(set)
    for a, b, k in edges:
        if k != "include":
            adj[a].add(b)
    seen: set[str] = set()
    todo = [ref(s) for s in syms if is_root_file(s[2])]
    while todo:
        v = todo.pop()
        if v in seen:
            continue
        seen.add(v)
        todo.extend(adj.get(v, ()))
    return sorted(
        ref(s)
        for s in syms
        if s[1] == "function" and ref(s) not in seen and not is_root_file(s[2]) and not implicit_callee(s[0])
    )


def build(root: Path, syms: list[Sym]) -> tuple[list[tuple[str, str, str]], list[list[str]], list[str]]:
    files = {s[2] for s in syms} | {
        str(p.relative_to(root))
        for p in root.rglob("*")
        if p.suffix in {".h", ".hpp", ".c", ".cc", ".cpp"}
        and p.is_file()
        and not (VENDORED_DIRS & set(p.relative_to(root).parts))
    }
    edges = include_edges(root, files) + symbol_edges(root, syms)
    parts = {part_of(f) for f in {s[2] for s in syms}}
    graph: dict[str, set[str]] = {p: set() for p in parts}
    for a, b, k in edges:
        pa, pb = (part_of(a), part_of(b)) if k == "include" else (part_of(a.split("@")[1]), part_of(b.split("@")[1]))
        if pa != pb and pa in graph and pb in graph:
            graph[pa].add(pb)
    return edges, tarjan(graph), dead_candidates(syms, edges)


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("inventory")
    p.add_argument("--source-root", required=True)
    p.add_argument("--out-dir", default="migration")
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    inv, root, out = Path(args.inventory), Path(args.source_root), Path(args.out_dir)
    if not inv.is_file() or not root.is_dir():
        print("depmap: need an inventory file and a source root", file=sys.stderr)
        return 1
    syms = read_inventory(inv)
    edges, sccs, dead = build(root, syms)
    out.mkdir(parents=True, exist_ok=True)
    (out / "edges.tsv").write_text("src\tdst\tkind\n" + "".join(f"{a}\t{b}\t{k}\n" for a, b, k in sorted(set(edges))))
    cycles = [c for c in sccs if len(c) > 1]
    (out / "cycles.txt").write_text("".join(" ".join(c) + "\n" for c in cycles))
    (out / "order.txt").write_text("".join(" ".join(c) + "\n" for c in sccs))
    (out / "dead.txt").write_text("".join(d + "\n" for d in dead))
    print(
        f"depmap: {len(set(edges))} edges, {len(sccs)} part group(s), {len(cycles)} cycle(s), "
        f"{len(dead)} dead-code candidate(s) -> {out}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
