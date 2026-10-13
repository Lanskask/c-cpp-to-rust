"""install_global.py: the installed copy carries no repository-relative path and keeps working scripts."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def test_installed_copy_has_global_paths_only(tmp_path: Path) -> None:
    dest = tmp_path / "c-cpp-to-rust"
    r = subprocess.run(
        [sys.executable, "-I", str(ROOT / "skill" / "install_global.py"), "--dest", str(dest)],
        capture_output=True,
        text=True,
    )
    assert r.returncode == 0, r.stderr
    skill = (dest / "SKILL.md").read_text()
    assert skill.startswith("---\nname: c-cpp-to-rust\n")
    leftovers = [
        str(f.relative_to(dest))
        for f in dest.rglob("*")
        if f.is_file() and f.suffix in {".md", ".py", ".json", ".Makefile"} and "skill/c-cpp-to-rust" in f.read_text()
    ]
    assert leftovers == []
    assert "guard.py" in (dest / "assets" / "settings.json").read_text()
    assert "$HOME/.claude/skills/c-cpp-to-rust/scripts/guard.py" in (dest / "assets" / "settings.json").read_text()
    r = subprocess.run(
        [sys.executable, "-I", str(dest / "scripts" / "guard.py"), "check", "pilots/x/src/a.c"],
        capture_output=True,
        text=True,
        cwd=tmp_path,
    )
    assert r.returncode == 0 and r.stdout.split()[0] in {"ALLOW", "REFUSE"}
