"""harness.py G8 (raw bytes, timeouts are DIFF) and G9 (--repeat: unstable inputs are not compared)."""

from __future__ import annotations

import sys
from pathlib import Path

import harness


def script(tmp_path: Path, name: str, body: str) -> list[str]:
    p = tmp_path / name
    p.write_text("import sys\n" + body)
    return [sys.executable, str(p)]


def one_input(tmp_path: Path) -> list[Path]:
    d = tmp_path / "in"
    d.mkdir(exist_ok=True)
    (d / "a.txt").write_text("x")
    return sorted(d.iterdir())


def test_unstable_original_is_reported_not_compared(tmp_path: Path) -> None:
    counter = tmp_path / "count"
    counter.write_text("0")
    flaky = script(
        tmp_path,
        "flaky.py",
        f"p = {str(counter)!r}\nn = int(open(p).read()) + 1\nopen(p, 'w').write(str(n))\nprint(n)\n",
    )
    steady = script(tmp_path, "steady.py", "print('same')\n")
    rows = harness.compare(flaky, steady, one_input(tmp_path), repeat=2)
    assert rows[0][3] == "UNSTABLE" and "2 distinct" in rows[0][4]
    out = tmp_path / "r.tsv"
    cmd_o = " ".join([sys.executable, str(tmp_path / "flaky.py")])
    cmd_r = " ".join([sys.executable, str(tmp_path / "steady.py")])
    assert harness.main(["--orig", cmd_o, "--rust", cmd_r, "--inputs", str(tmp_path / "in"), "--out", str(out),
                         "--repeat", "2"]) == 0  # fmt: skip


def test_repeat_one_keeps_old_behaviour(tmp_path: Path) -> None:
    steady = script(tmp_path, "s.py", "print('a')\n")
    other = script(tmp_path, "o.py", "print('b')\n")
    assert harness.compare(steady, steady, one_input(tmp_path))[0][3] == "same"
    assert harness.compare(steady, other, one_input(tmp_path))[0][3] == "DIFF"


def test_different_invalid_utf8_bytes_are_a_diff(tmp_path: Path) -> None:
    a = script(tmp_path, "a.py", "sys.stdout.buffer.write(b'\\xff')\n")
    b = script(tmp_path, "b.py", "sys.stdout.buffer.write(b'\\xfe')\n")
    assert harness.compare(a, b, one_input(tmp_path))[0][3] == "DIFF"


def test_timeout_on_both_sides_is_a_diff(tmp_path: Path, monkeypatch) -> None:  # noqa: ANN001
    monkeypatch.setattr(harness, "TIMEOUT_S", 0.3)
    hang = script(tmp_path, "h.py", "import time\ntime.sleep(5)\n")
    assert harness.compare(hang, hang, one_input(tmp_path))[0][3] == "DIFF"
