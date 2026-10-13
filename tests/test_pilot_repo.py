"""pilot_repo.py: split a tracked Pilot into its own repository with src/ as a submodule; run.py commits there."""

from __future__ import annotations

import subprocess
from pathlib import Path

import pilot_repo
import pytest

ID = ["-c", "user.name=t", "-c", "user.email=t@t"]


def g(repo: Path, *args: str) -> str:
    r = subprocess.run(["git", "-C", str(repo), *ID, *args], capture_output=True, text=True)
    assert r.returncode == 0, r.stderr
    return r.stdout.strip()


def make_main(tmp: Path) -> tuple[Path, str]:
    up = tmp / "upstream"
    up.mkdir()
    g(up, "init", "-q", "-b", "main")
    (up / "a.c").write_text("int f(void){return 1;}\n")
    g(up, "add", "."), g(up, "commit", "-qm", "u1")
    commit = g(up, "rev-parse", "HEAD")
    root = tmp / "main"
    ws = root / "pilots" / "p"
    (ws / "migration").mkdir(parents=True)
    (ws / "rust").mkdir()
    (ws / "migration" / "parts.tsv").write_text("x\n")
    (ws / "rust" / "lib.rs").write_text("// v1\n")
    (ws / "UPSTREAM").write_text(f"url={up}\ncommit={commit}\n")
    (root / ".gitignore").write_text("pilots/*/src/\n")
    g(root, "init", "-q", "-b", "main")
    g(root, "add", "."), g(root, "commit", "-qm", "main")
    (ws / "rust" / "lib.rs").write_text("// v2\n")
    g(root, "commit", "-qam", "pilot work")
    subprocess.run(["git", "clone", "-q", str(up), str(ws / "src")], check=True)
    return root, commit


def test_split_keeps_history_and_makes_src_a_submodule(tmp_path: Path) -> None:
    root, commit = make_main(tmp_path)
    ws = root / "pilots" / "p"
    assert pilot_repo.main(["split", "p", "--root", str(root)]) == 0
    assert (ws / ".git").exists() and g(ws, "rev-parse", "--show-toplevel") == str(ws.resolve())
    assert "pilot work" in g(ws, "log", "--format=%s")  # history kept
    assert g(ws, "submodule", "status").split()[0].lstrip("+-") == commit  # src pinned to UPSTREAM commit
    assert g(ws, "status", "--porcelain") == ""
    assert g(root, "ls-files", "pilots/p") == ""  # no longer tracked by the main repo


def test_remote_registers_the_pilot_as_a_submodule_of_the_main_repo(tmp_path: Path) -> None:
    root, _ = make_main(tmp_path)
    assert pilot_repo.main(["split", "p", "--root", str(root), "--remote", str(tmp_path / "remote.git")]) == 0
    assert "pilots/p" in (root / ".gitmodules").read_text()
    assert g(root, "ls-files", "-s", "pilots/p").startswith("160000")  # a gitlink


def test_split_refuses_a_dirty_pilot(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    root, _ = make_main(tmp_path)
    (root / "pilots" / "p" / "rust" / "lib.rs").write_text("// dirty\n")
    assert pilot_repo.main(["split", "p", "--root", str(root)]) == 1
    assert "uncommitted" in capsys.readouterr().err
