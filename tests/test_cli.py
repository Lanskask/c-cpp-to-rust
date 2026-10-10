"""Every script answers --help with exit 0; stubs exit 1 until implemented."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

SCRIPTS = Path(__file__).resolve().parent.parent / "skill" / "c-to-rust" / "scripts"
ALL = sorted(p.name for p in SCRIPTS.glob("*.py"))
STUB_ARGS = {
    "depmap.py": ["inv.tsv"],
    "inventory.py": [".", "--lang", "c"],
    "tier.py": ["inv.tsv", "order.txt"],
    "tm.py": ["build"],
}
STUBS = sorted(STUB_ARGS)


def run(script: str, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run([sys.executable, "-I", str(SCRIPTS / script), *args], capture_output=True, text=True)


@pytest.mark.parametrize("script", ALL)
def test_help(script: str) -> None:
    r = run(script, "--help")
    assert r.returncode == 0, r.stderr
    assert "usage:" in r.stdout


def test_no_stdlib_shadowing() -> None:
    names = {p.stem for p in SCRIPTS.glob("*.py")}
    assert not names & set(sys.stdlib_module_names)


@pytest.mark.parametrize("script", STUBS)
def test_stub_exits_1(script: str) -> None:
    r = run(script, *STUB_ARGS[script])
    assert r.returncode == 1
    assert "not implemented" in r.stderr
