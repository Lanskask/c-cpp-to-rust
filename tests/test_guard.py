"""guard.py: stage-dependent paths are refused only while fan-out tickets are in progress."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

GUARD = Path(__file__).resolve().parent.parent / "skill" / "c-to-rust" / "scripts" / "guard.py"


def call(root: Path, tool: str, **tool_input: str) -> subprocess.CompletedProcess[str]:
    payload = json.dumps({"cwd": str(root), "tool_name": tool, "tool_input": tool_input})
    return subprocess.run([sys.executable, "-I", str(GUARD)], input=payload, capture_output=True, text=True)


def ticket(root: Path, num: str, status: str, pilot: str = "p") -> None:
    d = root / "pilots" / pilot / "migration" / "tickets"
    d.mkdir(parents=True, exist_ok=True)
    (d / f"{num}-x.md").write_text(f"# {num}\n\n**Blocked by:** None\n\n**Status:** {status}\n")


def test_rulebook_is_writable_outside_fanout_and_refused_during_it(tmp_path: Path) -> None:
    target = str(tmp_path / "pilots/p/migration/RULEBOOK.md")
    assert call(tmp_path, "Edit", file_path=target).returncode == 0  # no tickets yet: Mapping
    ticket(tmp_path, "04", "todo")
    assert call(tmp_path, "Write", file_path=target).returncode == 0
    ticket(tmp_path, "04", "in_progress")
    r = call(tmp_path, "Edit", file_path=target)
    assert r.returncode == 2 and "mark 04 todo" in r.stderr
    ticket(tmp_path, "04", "done")
    assert call(tmp_path, "Edit", file_path=target).returncode == 0


def test_all_stage_dependent_paths_and_stage2_ticket(tmp_path: Path) -> None:
    ticket(tmp_path, "06", "in_progress")
    for rel in (
        "pilots/p/migration/parts-stage2.tsv",
        "pilots/p/migration/harness/make_inputs.py",
        "pilots/p/rust/Cargo.toml",
        "pilots/p/rust/tests/upstream.rs",
        "pilots/p/rust/examples/dump.rs",
    ):
        assert call(tmp_path, "Edit", file_path=str(tmp_path / rel)).returncode == 2, rel
    # the adapter, the crate source and the logs stay writable during fan-out
    for rel in ("pilots/p/rust/tests/adapter/mod.rs", "pilots/p/rust/src/lib.rs", "pilots/p/migration/RUN-NOTES.md"):
        assert call(tmp_path, "Edit", file_path=str(tmp_path / rel)).returncode == 0, rel


def test_only_the_pilot_in_fanout_is_protected(tmp_path: Path) -> None:
    ticket(tmp_path, "04", "in_progress", pilot="a")
    assert call(tmp_path, "Edit", file_path=str(tmp_path / "pilots/b/migration/RULEBOOK.md")).returncode == 0
    assert call(tmp_path, "Edit", file_path=str(tmp_path / "pilots/a/migration/RULEBOOK.md")).returncode == 2


def test_bash_writes_to_protected_paths_are_refused_in_fanout(tmp_path: Path) -> None:
    ticket(tmp_path, "04", "in_progress")
    assert call(tmp_path, "Bash", command="echo x >> pilots/p/migration/RULEBOOK.md").returncode == 2
    assert call(tmp_path, "Bash", command="sed -i '' s/a/b/ pilots/p/rust/examples/dump.rs").returncode == 2
    assert call(tmp_path, "Bash", command="cat pilots/p/migration/RULEBOOK.md").returncode == 0  # read only
    assert call(tmp_path, "Bash", command="make test").returncode == 0


def test_broken_input_fails_closed() -> None:
    r = subprocess.run([sys.executable, "-I", str(GUARD)], input="not json", capture_output=True, text=True)
    assert r.returncode == 2 and "refusing" in r.stderr


def test_check_subcommand(tmp_path: Path) -> None:
    ticket(tmp_path, "04", "in_progress")
    r = subprocess.run(
        [sys.executable, "-I", str(GUARD), "check", "pilots/p/migration/RULEBOOK.md"],
        capture_output=True,
        text=True,
        cwd=tmp_path,
    )
    assert r.returncode == 1 and "read-only" in r.stdout
