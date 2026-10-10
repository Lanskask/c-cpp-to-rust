"""harness.py: same output passes, different output or exit code is reported."""

from __future__ import annotations

import sys
from pathlib import Path

import harness


def driver(tmp_path: Path, name: str, body: str) -> list[str]:
    p = tmp_path / name
    p.write_text("import sys\ndata = open(sys.argv[-1]).read()\n" + body)
    return [sys.executable, str(p)]


def test_match_and_diff(tmp_path: Path) -> None:
    inputs = tmp_path / "in"
    inputs.mkdir()
    (inputs / "a.txt").write_text("hello")
    (inputs / "b.txt").write_text("boom")
    orig = driver(tmp_path, "o.py", "print(data.upper())\n")
    rust = driver(tmp_path, "r.py", "print(data.upper() if data != 'boom' else 'BANG')\nsys.exit(3 if data == 'boom' else 0)\n")
    rows = harness.compare(orig, rust, sorted(inputs.iterdir()))
    assert rows[0][3] == "same"
    assert rows[1][3] == "DIFF" and "exit 0 vs 3" in rows[1][4]


def test_main_exit_codes(tmp_path: Path) -> None:
    inputs = tmp_path / "in"
    inputs.mkdir()
    (inputs / "a.txt").write_text("x")
    cmd = f"{sys.executable} -c \"import sys; print(open(sys.argv[-1]).read())\""
    out = tmp_path / "r.tsv"
    assert harness.main(["--orig", cmd, "--rust", cmd, "--inputs", str(inputs), "--out", str(out)]) == 0
    assert "same" in out.read_text()
    assert harness.main(["--orig", cmd, "--rust", cmd, "--inputs", str(tmp_path / "none")]) == 1
