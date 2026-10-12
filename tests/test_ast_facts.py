"""ast_facts.py: clang-resolved symbols, calls and facts; the regex fallback; depmap reading calls.tsv."""

from __future__ import annotations

import shutil
from pathlib import Path

import ast_facts
import depmap
import inventory
import pytest

needs_clang = pytest.mark.skipif(shutil.which("clang++") is None, reason="clang++ not installed")

LIB = """\
#include <stdexcept>
namespace ns {
class Box {
public:
    Box(int v) : v_(v) {}
    Box(double d) : v_(static_cast<int>(d)) {}
    int get() const;
    operator bool() const { return v_ != 0; }
private:
    int v_;
};
inline int Box::get() const {
    if (v_ < 0) throw std::runtime_error("negative");
    return v_;
}
inline int helper(const Box &b) {
    int *p = new int(b.get());
    int r = *p;
    delete p;
    return r;
}
inline int uses_conversion(const Box &b) { return b ? 1 : 0; }
}
"""


def make_lib(tmp: Path) -> Path:
    root = tmp / "lib"
    root.mkdir()
    (root / "box.hpp").write_text(LIB)
    return root


@needs_clang
def test_overloads_calls_and_facts(tmp_path: Path) -> None:
    root = make_lib(tmp_path)
    out = tmp_path / "out"
    assert ast_facts.main([str(root), "--file", "box.hpp", "--out-dir", str(out)]) == 0
    symbols = [ln.split("\t") for ln in (out / "ast-symbols.tsv").read_text().splitlines()[3:]]
    names = [s[0] for s in symbols if s[1] == "function"]
    assert names.count("Box::Box") == 2  # both overloads, not one row
    assert {"Box::get", "Box::operator bool", "helper", "uses_conversion"} <= set(names)
    calls = {tuple(ln.split("\t")[:2]) for ln in (out / "calls.tsv").read_text().splitlines()[1:]}
    assert ("helper@box.hpp", "Box::get@box.hpp") in calls
    assert ("uses_conversion@box.hpp", "Box::operator bool@box.hpp") in calls  # implicit conversion is a call
    facts = {
        tuple(ln.split("\t")[:4:2]) + (ln.split("\t")[3],) for ln in (out / "facts.tsv").read_text().splitlines()[1:]
    }
    assert ("Box::get@box.hpp", "throw", "std::runtime_error") in facts
    assert any(f[:2] == ("helper@box.hpp", "new") for f in facts)
    assert any(f[:2] == ("helper@box.hpp", "delete") for f in facts)


@needs_clang
def test_inventory_uses_ast_engine_and_depmap_reads_its_calls(tmp_path: Path) -> None:
    root = make_lib(tmp_path)
    out = tmp_path / "migration"
    out.mkdir()
    syms, engine = inventory.inventory(root, "cpp", None, (), out)
    assert engine == "clang-ast+regex"
    assert [s.name for s in syms].count("Box::Box") == 2
    inv = out / "inventory.tsv"
    inv.write_text(
        "symbol\tkind\tfile\tlines\ttags\n"
        + "".join(f"{s.name}\t{s.kind}\t{s.file}\t{s.start}-{s.end}\t\n" for s in syms)
    )
    edges, _, _ = depmap.build(root, depmap.read_inventory(inv), out)
    assert ("helper@box.hpp", "Box::get@box.hpp", "call") in edges


def test_missing_compiler_falls_back_to_regex(tmp_path: Path) -> None:
    root = make_lib(tmp_path)
    out = tmp_path / "migration"
    out.mkdir()
    syms, engine = inventory.inventory(root, "cpp", None, (), out, cxx="no-such-compiler-xyz")
    assert engine == "regex"
    assert syms  # the regex pass still found the declarations
    assert not (out / "calls.tsv").exists()


def test_depmap_prefers_clang_edges_for_parsed_callers(tmp_path: Path) -> None:
    (tmp_path / "a.hpp").write_text("int f() { return g(); }\nint g() { return 1; }\n")
    syms: list[depmap.Sym] = [("f", "function", "a.hpp", 1, 1, ""), ("g", "function", "a.hpp", 2, 2, "")]
    ast = tmp_path / "ast"
    ast.mkdir()
    (ast / "ast-symbols.tsv").write_text(
        "symbol\tkind\tfile\tlines\nf\tfunction\ta.hpp\t1-1\ng\tfunction\ta.hpp\t2-2\n"
    )
    (ast / "calls.tsv").write_text("caller\tcallee\tcount\n")  # clang saw no call: the name-based f->g is dropped
    edges, _, _ = depmap.build(tmp_path, syms, ast)
    assert not [e for e in edges if e[2] == "call"]
    edges_regex, _, _ = depmap.build(tmp_path, syms, None)
    assert ("f@a.hpp", "g@a.hpp", "call") in edges_regex
