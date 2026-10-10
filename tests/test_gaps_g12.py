"""G12: tier.py target_path rules (single Part -> lib.rs, stem collisions, --crate-root-part)."""

from __future__ import annotations

from pathlib import Path

import tier
import tm


def run(tmp: Path, files: list[str], *extra: str) -> tuple[int, dict[str, str]]:
    inv = tmp / "inventory.tsv"
    inv.write_text("symbol\tkind\tfile\tlines\ttags\n")
    with inv.open("a") as f:
        for i, rel in enumerate(files):
            f.write(f"f{i}\tfunction\t{rel}\t1-2\t\n")
    order = tmp / "order.txt"
    order.write_text("\n".join(str(Path(r).with_suffix("")) for r in files) + "\n")
    out = tmp / "parts.tsv"
    rc = tier.main([str(inv), str(order), "--out", str(out), *extra])
    rows = tm.read_parts(out) if rc == 0 else []
    return rc, {r["part_id"]: r["target_path"] for r in rows}


def test_single_part_is_lib_rs(tmp_path: Path) -> None:
    rc, t = run(tmp_path, ["jsmn.h"])
    assert rc == 0 and t == {"jsmn": "rust/src/lib.rs"}


def test_distinct_stems(tmp_path: Path) -> None:
    rc, t = run(tmp_path, ["cJSON.c", "cJSON_Utils.c"])
    assert rc == 0
    assert t == {"cJSON": "rust/src/cJSON.rs", "cJSON_Utils": "rust/src/cJSON_Utils.rs"}


def test_colliding_stems_get_path_names(tmp_path: Path) -> None:
    rc, t = run(tmp_path, ["a/util.c", "b/util.c", "main.c"])
    assert rc == 0
    assert t == {"a/util": "rust/src/a_util.rs", "b/util": "rust/src/b_util.rs", "main": "rust/src/main.rs"}
    assert len(set(t.values())) == 3


def test_crate_root_part(tmp_path: Path) -> None:
    rc, t = run(tmp_path, ["a.c", "b.c"], "--crate-root-part", "a")
    assert rc == 0 and t == {"a": "rust/src/lib.rs", "b": "rust/src/b.rs"}
    rc, _ = run(tmp_path, ["a.c", "b.c"], "--crate-root-part", "zzz")
    assert rc == 1


def test_new_risky_tags_make_a_part_judgment_and_ctype_long_do_not() -> None:
    import tier

    assert {"fnptr", "bitfield", "float-format", "strtod", "qsort", "asm", "signal", "thread"} <= tier.JUDGMENT
    assert not ({"ctype", "long"} & tier.JUDGMENT)
