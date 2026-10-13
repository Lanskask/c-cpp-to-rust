"""refbuild.py: candidates are tried in order, the report names the first passing one, src/ stays untouched."""

from __future__ import annotations

import shutil
from pathlib import Path

import pytest
import refbuild

needs_cxx = pytest.mark.skipif(shutil.which("c++") is None, reason="no c++ compiler")


def ws_with(tmp: Path, test: str) -> Path:
    ws = tmp / "pilots" / "p"
    (ws / "src" / "res").mkdir(parents=True)
    (ws / "src" / "t.cpp").write_text(test)
    (ws / "src" / "res" / "in.txt").write_text("x")
    return ws


@needs_cxx
def test_first_passing_candidate_is_reported_and_src_is_untouched(tmp_path: Path) -> None:
    ws = ws_with(tmp_path, '#include <fstream>\nint main(){ std::ifstream f("res/in.txt"); return f ? 0 : 1; }\n')
    args = [str(ws), "--test", "t.cpp", "--resource", "res", "--cc", "c++", "--std", "c++17", "--opt=-O0"]
    assert refbuild.main(args) == 0
    text = (ws / "migration" / "harness" / "refbuild.md").read_text()
    assert "## Chosen" in text and "`-std=c++17 -O0`" in text
    assert sorted(p.name for p in (ws / "src").iterdir()) == ["res", "t.cpp"]


@needs_cxx
def test_failing_test_exits_1_and_says_so(tmp_path: Path) -> None:
    ws = ws_with(tmp_path, "int main(){ return 3; }\n")
    assert refbuild.main([str(ws), "--test", "t.cpp", "--cc", "c++", "--std", "c++17", "--opt=-O0"]) == 1
    assert "No candidate passed" in (ws / "migration" / "harness" / "refbuild.md").read_text()
