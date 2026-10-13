#!/usr/bin/env python3
"""Extract C++ symbols, resolved call edges and per-function facts from clang's JSON AST.

The regex inventory cannot tell overloads apart, misses static members it cannot parse, and finds calls by name.
This script asks clang instead (`-fsyntax-only -Xclang -ast-dump=json`), with no compile_commands.json needed.

Input: a source root and the files to look at (`--file REL`, repeatable; headers and sources, not tests).
Outputs in --out-dir:
  ast-symbols.tsv  name, kind (function|record|enum|typedef|variable), file, lines; first lines `# engine: ...` and
                   `# in-root-errors: N`. Names are class-qualified without namespaces (`json::json`,
                   `json::operator bool`), the same form `inventory.py` writes.
  calls.tsv        caller, callee, count; both as `name@file` (the form depmap.py uses). Resolved by clang: an
                   overload set gives one edge per overload actually called, a member call is not confused with a
                   free function of the same name.
  facts.tsv        symbol, lines, fact, detail, count: throw (exception type), new (type), delete (type),
                   conversion (user-defined conversion function used). The `{facts}` of a Stage 1 prompt.

How a translation unit is built: all headers are included by one generated file, every source file is its own unit;
`-I` the root and the directories of the files. A full dump of a unit that includes the standard library is hundreds
of MB, so the dump is filtered by namespace name (`-ast-dump-filter`) when the files declare namespaces, and capped
at 150 MB otherwise (the unit is then skipped and the caller falls back to the regex inventory).
Errors inside the root are counted and printed; errors in system headers (libc++ missing a facility the project
needs, as for tinyjson with Apple clang) do not stop the extraction: clang still dumps the declarations.

Exit code 0 on success, 1 on bad arguments, 2 when clang is missing or yielded no symbols (fall back to regex).
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any

HEADER_EXT = {".h", ".hpp", ".hh", ".hxx"}
SOURCE_EXT = {".cpp", ".cc", ".cxx", ".c++"}
FUNC_KINDS = {"FunctionDecl", "CXXMethodDecl", "CXXConstructorDecl", "CXXDestructorDecl", "CXXConversionDecl"}
DUMP_CAP = 150 * 1024 * 1024
NAMESPACE = re.compile(r"\bnamespace\s+([A-Za-z_]\w*)")
ERROR_LINE = re.compile(r"^(.+?):\d+:\d+: (?:fatal )?error:", re.M)

Key = tuple[str, str, int, int]  # name, file, start, end


@dataclass
class Sym:
    name: str
    kind: str
    file: str
    start: int
    end: int

    @property
    def key(self) -> Key:
        return (self.name, self.file, self.start, self.end)

    @property
    def ref(self) -> str:
        return f"{self.name}@{self.file}"


@dataclass
class Unit:
    """What one clang run yielded."""

    symbols: list[Sym]
    calls: Counter[tuple[Key, Key]]
    facts: Counter[tuple[Key, str, str]]


class Collector:
    """Walks one JSON AST in document order. clang prints `file` and `line` of a location only when they differ
    from the previous location it printed, so every node of every subtree must be visited, wanted or not."""

    def __init__(self, root: Path, wanted: set[str]) -> None:
        self.root = root.resolve()
        self.wanted = wanted
        self.file = ""
        self.line = 0
        self.rel_cache: dict[str, str | None] = {}
        self.syms: list[Sym] = []
        self.sym_by_id: dict[str, int] = {}
        self.prev: dict[str, str] = {}
        self.records: dict[str, str] = {}
        self.pending: list[tuple[int, str]] = []
        self.pending_facts: Counter[tuple[int, str, str]] = Counter()

    def update(self, loc: object) -> tuple[str, int]:
        if isinstance(loc, dict):
            if "spellingLoc" in loc:
                self.update(loc["spellingLoc"])
            if "expansionLoc" in loc:
                self.update(loc["expansionLoc"])
            else:
                if "file" in loc:
                    self.file = str(loc["file"])
                if "line" in loc:
                    self.line = int(loc["line"])
        return self.file, self.line

    def rel(self, file: str) -> str | None:
        if file not in self.rel_cache:
            try:
                self.rel_cache[file] = str(Path(file).resolve().relative_to(self.root))
            except ValueError:
                self.rel_cache[file] = None
        return self.rel_cache[file]

    def add(self, name: str, kind: str, file: str, start: int, end: int) -> int | None:
        rel = self.rel(file)
        if rel is None or rel not in self.wanted:
            return None
        self.syms.append(Sym(name, kind, rel, start, end))
        return len(self.syms) - 1

    def walk(self, node: object, stack: tuple[str, ...], func: int | None) -> None:
        if not isinstance(node, dict):
            return
        self.update(node.get("loc"))
        raw_range = node.get("range")
        rng: dict[str, object] = raw_range if isinstance(raw_range, dict) else {}
        bfile, bline = self.update(rng.get("begin"))
        _, eline = self.update(rng.get("end"))
        kind = str(node.get("kind", ""))
        inner = node.get("inner") or []
        nid = str(node.get("id", ""))
        name = str(node.get("name", ""))
        implicit = bool(node.get("isImplicit"))
        child_stack, child_func = stack, func

        if kind == "CXXRecordDecl" and name and node.get("completeDefinition") and not implicit:
            qual = "::".join((*stack, name))
            self.records[nid] = qual
            if func is None:
                self.add(qual, "record", bfile, bline, eline)
            child_stack = (*stack, name)
        elif kind in FUNC_KINDS and name:
            if "previousDecl" in node:
                self.prev[nid] = str(node["previousDecl"])
            owner = self.records.get(str(node.get("parentDeclContextId", "")))
            qual = f"{owner}::{name}" if owner else "::".join((*stack, name))
            has_body = any(isinstance(c, dict) and c.get("kind") == "CompoundStmt" for c in inner)
            if has_body and not implicit:
                idx = self.add(qual, "function", bfile, bline, eline)
                if idx is not None:
                    self.sym_by_id[nid] = idx
                    child_func = idx
        elif kind == "EnumDecl" and name and func is None and node.get("completeDefinition", True):
            self.add("::".join((*stack, name)), "enum", bfile, bline, eline)
        elif kind in ("TypedefDecl", "TypeAliasDecl") and name and func is None and not implicit:
            self.add("::".join((*stack, name)), "typedef", bfile, bline, eline)
        elif kind == "VarDecl" and name and func is None and not stack and not implicit:
            self.add(name, "variable", bfile, bline, eline)
        elif func is not None:
            self.expression(node, kind, inner, func)

        for c in inner:
            self.walk(c, child_stack, child_func)

    def expression(self, node: dict[str, Any], kind: str, inner: list[Any], func: int) -> None:
        if kind == "CXXThrowExpr":
            thrown = (
                inner[0].get("type", {}).get("qualType", "?") if inner and isinstance(inner[0], dict) else "rethrow"
            )
            self.pending_facts[(func, "throw", str(thrown))] += 1
        elif kind == "CXXNewExpr":
            self.pending_facts[(func, "new", str(node.get("type", {}).get("qualType", "?")))] += 1
        elif kind == "CXXDeleteExpr":
            target = inner[0].get("type", {}).get("qualType", "?") if inner and isinstance(inner[0], dict) else "?"
            self.pending_facts[(func, "delete", str(target))] += 1
        elif kind == "DeclRefExpr":
            ref = node.get("referencedDecl")
            if isinstance(ref, dict) and ref.get("kind") in FUNC_KINDS:
                self.pending.append((func, str(ref.get("id", ""))))
        elif kind == "MemberExpr" and "referencedMemberDecl" in node:
            self.pending.append((func, str(node["referencedMemberDecl"])))
        elif kind == "ImplicitCastExpr" and isinstance(node.get("conversionFunc"), dict):
            conv = node["conversionFunc"]
            self.pending.append((func, str(conv.get("id", ""))))
            self.pending_facts[
                (func, "conversion", f"{conv.get('name', '?')} ({conv.get('type', {}).get('qualType', '?')})")
            ] += 1

    def resolve(self, decl_id: str) -> int | None:
        """A reference names the declaration clang saw at that point; follow redeclarations to the definition."""
        seen = 0
        while decl_id and seen < 32:
            if decl_id in self.sym_by_id:
                return self.sym_by_id[decl_id]
            decl_id = self.prev.get(decl_id, "")
            seen += 1
        return None

    def finish(self) -> Unit:
        # a definition is reached from its in-class prototype too: map the prototype chain onto the definition
        for sid, idx in list(self.sym_by_id.items()):
            p = self.prev.get(sid, "")
            hops = 0
            while p and hops < 32:
                self.sym_by_id.setdefault(p, idx)
                p = self.prev.get(p, "")
                hops += 1
        calls: Counter[tuple[Key, Key]] = Counter()
        for caller, decl_id in self.pending:
            callee = self.resolve(decl_id)
            if callee is not None and callee != caller:
                calls[(self.syms[caller].key, self.syms[callee].key)] += 1
        facts: Counter[tuple[Key, str, str]] = Counter()
        for (idx, fact, detail), n in self.pending_facts.items():
            facts[(self.syms[idx].key, fact, detail)] += n
        return Unit(self.syms, calls, facts)


def iter_documents(text: str) -> list[object]:
    """`-ast-dump-filter` prints one JSON document per match, back to back."""
    dec = json.JSONDecoder()
    out: list[object] = []
    i = 0
    while i < len(text):
        while i < len(text) and text[i].isspace():
            i += 1
        if i >= len(text):
            break
        doc, i = dec.raw_decode(text, i)
        out.append(doc)
    return out


def run_clang(cxx: str, flags: list[str], tu: Path, dump_filter: str | None) -> tuple[str, str] | None:
    cmd = [cxx, *flags, "-fsyntax-only", "-w", "-ferror-limit=0", "-Xclang", "-ast-dump=json"]
    if dump_filter:
        cmd += ["-Xclang", "-ast-dump-filter", "-Xclang", dump_filter]
    cmd.append(str(tu))
    try:
        proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    except OSError:
        return None
    assert proc.stdout is not None and proc.stderr is not None
    chunks: list[str] = []
    size = 0
    while True:
        part = proc.stdout.read(1 << 20)
        if not part:
            break
        size += len(part)
        if size > DUMP_CAP:
            proc.kill()
            print(
                f"ast_facts: {tu.name}: dump over {DUMP_CAP >> 20} MB, unit skipped (no namespace filter)",
                file=sys.stderr,
            )
            return None
        chunks.append(part)
    err = proc.stderr.read()
    proc.wait()
    return "".join(chunks), err


def collect_unit(
    root: Path, wanted: set[str], cxx: str, flags: list[str], tu: Path, namespaces: list[str]
) -> tuple[Unit, int]:
    """All dumps of one translation unit, merged; returns the unit and the number of in-root errors."""
    merged = Unit([], Counter(), Counter())
    seen: set[Key] = set()
    errors = 0
    for flt in namespaces or [None]:  # type: ignore[list-item]
        got = run_clang(cxx, flags, tu, flt)
        if got is None:
            continue
        out, err = got
        errors += sum(1 for m in ERROR_LINE.finditer(err) if _inside(root, m.group(1)))
        col = Collector(root, wanted)
        for doc in iter_documents(out):
            col.walk(doc, (), None)
        unit = col.finish()
        for s in unit.symbols:
            if s.key not in seen:
                seen.add(s.key)
                merged.symbols.append(s)
        merged.calls.update(unit.calls)
        merged.facts.update(unit.facts)
    return merged, errors


def _inside(root: Path, path: str) -> bool:
    try:
        Path(path).resolve().relative_to(root.resolve())
        return True
    except (ValueError, OSError):
        return False


def extract(root: Path, files: list[str], cxx: str, extra_flags: list[str]) -> tuple[Unit, int]:
    root = root.resolve()
    wanted = set(files)
    headers = sorted(f for f in files if Path(f).suffix in HEADER_EXT)
    sources = sorted(f for f in files if Path(f).suffix in SOURCE_EXT)
    namespaces: list[str] = []
    for f in files:
        for ns in NAMESPACE.findall((root / f).read_text(errors="replace")):
            if ns not in namespaces and ns != "std":
                namespaces.append(ns)
    dirs = sorted({str((root / f).parent) for f in files} | {str(root)})
    flags = [*extra_flags]
    if not any(a.startswith("-std=") for a in flags):
        flags.append("-std=c++17")
    flags += [f"-I{d}" for d in dirs]
    units: list[Path] = []
    tmp = Path(tempfile.mkdtemp(prefix="ast_facts_"))
    try:
        if headers:
            tu = tmp / "headers.cpp"
            tu.write_text("".join(f'#include "{(root / h)}"\n' for h in headers))
            units.append(tu)
        units += [root / s for s in sources]
        total = Unit([], Counter(), Counter())
        errors = 0
        seen: set[Key] = set()
        for tu in units:
            unit, errs = collect_unit(root, wanted, cxx, flags, tu, namespaces)
            errors += errs
            for s in unit.symbols:
                if s.key not in seen:
                    seen.add(s.key)
                    total.symbols.append(s)
            total.calls.update(unit.calls)
            total.facts.update(unit.facts)
        return total, errors
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def write_outputs(out: Path, unit: Unit, engine: str, errors: int) -> None:
    out.mkdir(parents=True, exist_ok=True)
    by_key = {s.key: s for s in unit.symbols}
    rows = sorted(unit.symbols, key=lambda s: (s.file, s.start, s.end, s.name))
    (out / "ast-symbols.tsv").write_text(
        f"# engine: {engine}\n# in-root-errors: {errors}\nsymbol\tkind\tfile\tlines\n"
        + "".join(f"{s.name}\t{s.kind}\t{s.file}\t{s.start}-{s.end}\n" for s in rows)
    )
    (out / "calls.tsv").write_text(
        "caller\tcallee\tcount\n"
        + "".join(
            f"{by_key[a].ref}\t{by_key[b].ref}\t{n}\n"
            for (a, b), n in sorted(unit.calls.items(), key=lambda kv: (by_key[kv[0][0]].ref, by_key[kv[0][1]].ref))
        )
    )
    (out / "facts.tsv").write_text(
        "symbol\tlines\tfact\tdetail\tcount\n"
        + "".join(
            f"{by_key[k].ref}\t{k[2]}-{k[3]}\t{fact}\t{detail}\t{n}\n"
            for (k, fact, detail), n in sorted(
                unit.facts.items(), key=lambda kv: (kv[0][0][1], kv[0][0][2], kv[0][1], kv[0][2])
            )
        )
    )


def clang_version(cxx: str) -> str:
    try:
        r = subprocess.run([cxx, "--version"], capture_output=True, text=True, check=False)
    except OSError:
        return "unknown"
    return (r.stdout.splitlines() or ["unknown"])[0].strip()


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("source_root")
    p.add_argument("--file", action="append", default=[], metavar="REL", help="source-root-relative file (repeatable)")
    p.add_argument("--out-dir", required=True)
    p.add_argument("--cxx", default=os.environ.get("CXX", "clang++"), help="C++ compiler with `-Xclang -ast-dump=json`")
    p.add_argument("--cxx-flag", action="append", default=[], metavar="FLAG", help="extra compiler flag (repeatable)")
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    root = Path(args.source_root)
    if not root.is_dir() or not args.file:
        print("ast_facts: need a source root and at least one --file", file=sys.stderr)
        return 1
    if shutil.which(args.cxx) is None:
        print(f"ast_facts: {args.cxx} not found", file=sys.stderr)
        return 2
    unit, errors = extract(root, list(args.file), args.cxx, list(args.cxx_flag))
    if not unit.symbols:
        print("ast_facts: clang yielded no symbols", file=sys.stderr)
        return 2
    write_outputs(Path(args.out_dir), unit, f"clang-ast ({clang_version(args.cxx)})", errors)
    print(
        f"ast_facts: {len(unit.symbols)} symbols, {sum(unit.calls.values())} call sites, "
        f"{len(unit.facts)} fact rows, {errors} in-root error(s) -> {args.out_dir}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
