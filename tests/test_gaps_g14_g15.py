"""Gaps G14 (clang engine keeps macros and config-macro tags), G15 (risky-construct tags), G16 (# cmd: header)."""

from __future__ import annotations

import json
import shutil
from pathlib import Path

import depmap
import inventory
import pytest
import tier


def tags_of(code: str, name: str = "f", lang: str = "c", ext: str = "c") -> set[str]:
    return set(_scan(code, lang, ext)[name])


def _scan(code: str, lang: str, ext: str) -> dict[str, list[str]]:
    syms = inventory.scan_regex(code, f"x.{ext}", lang)
    return {s.name: s.tags for s in syms}


# --- G15: one fixture per tag, plus a negative case where a false positive is likely


@pytest.mark.parametrize(
    ("tag", "code"),
    [
        ("fnptr", "typedef int (*cb_t)(int);\nint f(cb_t g) { int (*h)(int) = g; return h(1); }\n"),
        ("fnptr", "struct ops { int (*run)(int); };\nint f(struct ops *o) { return o->run(2); }\n"),
        ("bitfield", "struct f { unsigned a : 3; unsigned : 5; int b : 2; };\n"),
        ("float-format", '#include <stdio.h>\nvoid f(double d) { printf("%d %.3f\\n", 1, d); }\n'),
        ("float-format", 'void f(double d) { fprintf(stderr, "%-8.2e|%10g|%a", d, d, d); }\n'),
        ("strtod", "double f(const char *s) { return strtod(s, 0) + atof(s); }\n"),
        ("qsort", "void f(int *a, int n) { qsort(a, n, sizeof *a, cmp); }\n"),
        ("ctype", "int f(int c) { return isspace(c) || isdigit(c) || isalpha(c); }\n"),
        ("long", "unsigned long f(long x) { return (unsigned long)x; }\n"),
        ("asm", 'void f(void) { __asm__ volatile("nop"); }\n'),
        ("signal", "void f(void) { signal(2, h); raise(2); }\n"),
        ("signal", "void f(void) { struct sigaction sa; sigaction(2, &sa, 0); }\n"),
        ("thread", "int f(void) { pthread_t t; return pthread_create(&t, 0, g, 0); }\n"),
        ("thread", "int f(void) { thrd_t t; return thrd_create(&t, g, 0); }\n"),
        ("thread", "_Thread_local int tls;\nint f(void) { return tls; }\n"),
    ],
)
def test_new_tag_present(tag: str, code: str) -> None:
    by = _scan(code, "c", "c")
    assert any(tag in t for t in by.values()), (tag, by)


def test_no_false_positives_on_plain_code() -> None:
    code = (
        'void f(int x, double d) {\n  printf("%d %s %%f\\n", x, "a");\n  int y = x ? 1 : 2;\n'
        "  switch (x) {\n  case 1: break;\n  default: break;\n  }\n  g(d);\n}\n"
        "int h(int *p) { return *p > 3 ? (*p) * (2) : 0; }\n"
    )
    by = _scan(code, "c", "c")
    for tags in by.values():
        assert not {"float-format", "bitfield", "fnptr", "long", "ctype"} & set(tags), tags


def test_new_delete_only_for_cpp() -> None:
    code = "void f(int new, int delete) { new = delete; }\n"
    assert "malloc" not in _scan(code, "c", "c")["f"]
    cpp = "void f() { int *p = new int[3]; delete[] p; }\n"
    assert "malloc" in _scan(cpp, "cpp", "cpp")["f"]
    assert "malloc" not in _scan(cpp, "c", "c")["f"]


def test_cpp_member_call_is_not_fnptr() -> None:
    assert "fnptr" not in _scan("void f(S &s) { s.go(); s.p->go(); }\n", "cpp", "cpp")["f"]


def test_existing_tags_unchanged() -> None:
    code = "static int n;\nint f(void) { char *p = malloc(3); free(p); errno = 1; goto e; e: return n; }\n"
    by = _scan(code, "c", "c")
    assert by["f"] == ["goto", "errno", "malloc"]
    assert by["n"] == ["static-global"]


# --- G16: command line in the header; consumers tolerate it


def test_cmd_header_and_consumers(tmp_path: Path) -> None:
    root = tmp_path / "src"
    root.mkdir()
    (root / "a.c").write_text("int a(void) { return 1; }\n")
    out = tmp_path / "m" / "inventory.tsv"
    assert inventory.main([str(root), "--lang", "c", "--out", str(out)]) == 0
    head = out.read_text().splitlines()[:3]
    assert head[0] == "# engine: regex"
    assert head[1].startswith("# cmd: inventory.py ") and "--lang c" in head[1] and str(root) in head[1]
    assert head[2].startswith("symbol\t")
    assert depmap.main([str(out), "--source-root", str(root), "--out-dir", str(out.parent)]) == 0
    assert tier.main([str(out), str(out.parent / "order.txt"), "--out", str(out.parent / "parts.tsv")]) == 0


# --- G14: clang engine keeps config-macro tags and macros


@pytest.mark.skipif(shutil.which("clang") is None, reason="clang not installed")
def test_clang_engine_merges_regex_tags(tmp_path: Path) -> None:
    root = (tmp_path / "src").resolve()
    root.mkdir()
    src = root / "a.c"
    src.write_text(
        "#define SQR(x) ((x)*(x))\n"
        "#define LIMIT 4\n"
        "int on(void) {\n"
        "#ifdef FAST\n  return SQR(2);\n#else\n  return LIMIT;\n#endif\n}\n"
        "#ifdef NEVER\nint hidden(void) { return 9; }\n#endif\n"
    )
    db = tmp_path / "compile_commands.json"
    db.write_text(json.dumps([{"directory": str(root), "file": "a.c", "arguments": ["clang", "-c", str(src)]}]))
    syms, engine = inventory.inventory(root, "c", db)
    assert engine.startswith("clang-ast")
    by = {(s.name, s.kind): s for s in syms}
    assert "config-macro:FAST" in by[("on", "function")].tags
    assert "macro-fn" in by[("SQR", "macro")].tags
    assert ("LIMIT", "macro") in by
    assert "config-macro:NEVER" in by[("hidden", "function")].tags  # disabled code, invisible to clang
