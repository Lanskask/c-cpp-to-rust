"""inventory -> depmap -> tier -> tm on a small C fixture with a header/impl pair, a cycle and risky constructs."""

from __future__ import annotations

import subprocess
from pathlib import Path

import depmap
import inventory
import tier
import tm

FIXTURE = {
    "lib/util.h": "#ifndef UTIL_H\n#define UTIL_H\n#define SQR(x) ((x)*(x))\nint util_add(int a, int b);\n#endif\n",
    "lib/util.c": '#include "util.h"\nint util_add(int a, int b) {\n  return a + b;\n}\n',
    "lib/buf.h": "typedef struct buf { char *p; int n; } buf_t;\nvoid buf_free(buf_t *b);\n",
    "lib/buf.c": '#include <stdlib.h>\n#include "buf.h"\n#include "util.h"\n'
    "void buf_free(buf_t *b) { free(b->p); b->n = util_add(0, 0); }\n"
    "static int counter = 0;\n"
    "int next_id(void) {\n#ifdef USE_FAST\n  return ++counter;\n#else\n  goto out;\nout:\n"
    "  return counter++;\n#endif\n}\n",
    "lib/a.c": "int b_fn(void);\nint a_fn(void) { return b_fn(); }\n",
    "lib/b.c": "int a_fn(void);\nint b_fn(void) { return a_fn(); }\n",
    "test/test_util.c": '#include "../lib/util.h"\nint main(void) { return util_add(1, 2) - 3; }\n',
}


def make(tmp: Path) -> Path:
    for rel, text in FIXTURE.items():
        p = tmp / "src" / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text)
    return tmp / "src"


def test_inventory_symbols_and_tags(tmp_path: Path) -> None:
    root = make(tmp_path)
    syms, engine = inventory.inventory(root, "c", None)
    assert engine == "regex"
    by = {(s.name, s.file): s for s in syms}
    assert by[("util_add", "lib/util.c")].start == 2
    assert by[("SQR", "lib/util.h")].tags.count("macro-fn") == 1
    assert ("buf_t", "lib/buf.h") in by and ("buf", "lib/buf.h") in by
    nid = by[("next_id", "lib/buf.c")]
    assert {"goto", "config-macro:USE_FAST"} <= set(nid.tags)
    assert "static-global" in by[("counter", "lib/buf.c")].tags
    assert "malloc" in by[("buf_free", "lib/buf.c")].tags  # free()
    assert not any(t.startswith("config-macro:UTIL_H") for s in syms for t in s.tags)  # header guard ignored


def run_all(tmp_path: Path) -> Path:
    root = make(tmp_path)
    mig = tmp_path / "migration"
    assert inventory.main([str(root), "--lang", "c", "--out", str(mig / "inventory.tsv")]) == 0
    assert depmap.main([str(mig / "inventory.tsv"), "--source-root", str(root), "--out-dir", str(mig)]) == 0
    assert tier.main([str(mig / "inventory.tsv"), str(mig / "order.txt"), "--out", str(mig / "parts.tsv")]) == 0
    return mig


def test_depmap_order_cycles_dead(tmp_path: Path) -> None:
    mig = run_all(tmp_path)
    order = (mig / "order.txt").read_text().splitlines()
    assert order.index("lib/util") < order.index("lib/buf")  # dependency first
    assert "lib/a lib/b" in order  # cycle stays together
    assert (mig / "cycles.txt").read_text().strip() == "lib/a lib/b"
    dead = (mig / "dead.txt").read_text().split()
    assert "buf_free@lib/buf.c" in dead and "util_add@lib/util.c" not in dead  # tests reach util_add only


def test_tier_and_deps(tmp_path: Path) -> None:
    mig = run_all(tmp_path)
    rows = {r["part_id"]: r for r in tm.read_parts(mig / "parts.tsv")}
    assert set(rows) == {"lib/util", "lib/buf", "lib/a", "lib/b"}  # test file is not a Part
    assert rows["lib/buf"]["tier"] == "judgment" and rows["lib/buf"]["deps"] == "lib/util"
    assert rows["lib/a"]["deps"] == "lib/b"
    assert rows["lib/util"]["tier"] == "judgment"  # SQR is a function-like macro
    assert all(r["state"] == "todo" for r in rows.values())
    assert rows["lib/util"]["target_path"] == "rust/src/util.rs"  # distinct stems keep <stem>.rs


def test_tm_build_and_search(tmp_path: Path) -> None:
    ws = tmp_path / "ws"
    (ws / "migration").mkdir(parents=True)
    subprocess.run(["git", "-C", str(ws), "init", "-q"], check=True)
    (ws / "a.c").write_text("int f(void) { return 1; }\n")
    (ws / "a.rs").write_text("pub fn f() -> i32 { 1 }\n")
    (ws / "migration" / "parts.tsv").write_text(
        tier.HEADER + "\n" + "a\tfile\ta.c\ta.rs\tmechanical\t\tdone\t1\t\t\tabc1234\t\n"
        "b\tfile\tb.c\tb.rs\tjudgment\t\tfailed\t2\t\tE0308 mismatched types in jsmn_parse\t\t\n"
    )
    (ws / "migration" / "RUN-NOTES.md").write_text(
        "# notes\n## Pointer arithmetic\nuse offset() not add() for negative\n"
    )
    db = ws / "migration" / "tm.sqlite"
    assert tm.main(["--db", str(db), "build"]) == 0
    kinds = {k for k, _, _ in tm.search(db, "mismatched types E0308")}
    assert "error" in kinds
    assert tm.search(db, "negative offset")[0][0] == "note"
    assert tm.search(db, "!!!") == []


def test_inventory_skips_vendored_dirs_and_exclude_globs(tmp_path: Path) -> None:
    import inventory

    src = tmp_path / "src"
    for rel in ("lib.c", "fuzzing/fuzz.c", "tests/unity/unity.c", "gen/skip_me.c", "keep/ok.c"):
        f = src / rel
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text("int f(void) { return 0; }\n")
    names = {str(p.relative_to(src)) for p in inventory.source_files(src, "c", ("gen/*",))}
    assert names == {"lib.c", "keep/ok.c"}


def test_function_returning_an_enum_is_a_function_not_an_enum(tmp_path: Path) -> None:
    import inventory

    code = (
        "enum op { ADD, REMOVE };\n"
        "static enum op decode(const char *s)\n{\n    if (s) { return ADD; }\n    return REMOVE;\n}\n"
        "int after(void)\n{\n    return 1;\n}\n"
    )
    syms = {(s.name, s.kind) for s in inventory.scan_regex(code, "a.c")}
    assert ("decode", "function") in syms and ("after", "function") in syms and ("op", "enum") in syms
    assert not any(n in ("if", "return") for n, _ in syms)
