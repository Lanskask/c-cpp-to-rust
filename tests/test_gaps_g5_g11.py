"""G5 (gate pass record checked by workqueue verify) and G11 (stubbed and done Parts verified against files)."""

from __future__ import annotations

import subprocess
from pathlib import Path

import gate
import workqueue as wq

HEADER = "part_id\tkind\tsource_paths\ttarget_path\ttier\tdeps\tstate\tattempts\tmodel\tlast_error\tcommit\tupdated\n"


def git(root: Path, *a: str) -> str:
    r = subprocess.run(
        ["git", "-C", str(root), "-c", "user.name=t", "-c", "user.email=t@t", *a],
        check=True,
        capture_output=True,
        text=True,
    )
    return r.stdout.strip()


def workspace(tmp_path: Path) -> Path:
    git(tmp_path, "init", "-q")
    (tmp_path / "rust" / "src").mkdir(parents=True)
    (tmp_path / "migration").mkdir()
    (tmp_path / "rust" / "src" / "lib.rs").write_text("pub fn f() {}\n")
    (tmp_path / "other.rs").write_text("x\n")
    git(tmp_path, "add", ".")
    git(tmp_path, "commit", "-q", "-m", "base")
    return tmp_path


def row(part: str, state: str, commit: str = "", target: str = "rust/src/lib.rs") -> wq.Row:
    cells = [part, "file", "a.c", target, "judgment", "", state, "1", "", "", commit, ""]
    return dict(zip(HEADER.strip().split("\t"), cells, strict=True))


def test_record_pass_appends_a_tracked_row(tmp_path: Path) -> None:
    root = workspace(tmp_path)
    gate.record_pass(root, "p", 2, ["", "strict"])
    lines = (root / "migration" / "gate-passed.tsv").read_text().splitlines()
    assert lines[0] == "part\tstage\thead\tfeatures\ttime"
    cells = lines[1].split("\t")
    assert cells[:2] == ["p", "2"] and cells[2] == git(root, "rev-parse", "HEAD") and cells[3] == "|strict"


def test_done_part_needs_a_gate_record_at_or_before_its_commit(tmp_path: Path) -> None:
    root = workspace(tmp_path)
    (root / "rust" / "src" / "lib.rs").write_text("pub fn f() { /* new */ }\n")
    gate.record_pass(root, "p", 1, [""])  # recorded at the pre-commit HEAD, like the real flow
    git(root, "add", ".")
    git(root, "commit", "-q", "-m", "part")
    commit = git(root, "rev-parse", "HEAD")
    rows = [row("p", "done", commit)]
    assert wq.verify(rows, root, stage=1) == []
    problems = wq.verify(rows, root, stage=2)  # a stage 1 pass does not count for stage 2
    assert len(problems) == 1 and "no gate pass recorded for stage 2" in problems[0]
    assert wq.verify(rows, root, stage=2, require_gate_record=False) == []


def test_done_part_without_any_record_fails_unless_legacy_flag(tmp_path: Path) -> None:
    root = workspace(tmp_path)
    commit = git(root, "rev-parse", "HEAD")
    problems = wq.verify([row("p", "done", commit)], root)
    assert any("no gate pass recorded" in p for p in problems)
    assert wq.verify([row("p", "done", commit)], root, require_gate_record=False) == []


def test_commit_must_touch_the_target(tmp_path: Path) -> None:
    root = workspace(tmp_path)
    (root / "other.rs").write_text("y\n")
    git(root, "add", ".")
    git(root, "commit", "-q", "-m", "unrelated")
    commit = git(root, "rev-parse", "HEAD")
    problems = wq.verify([row("p", "done", commit)], root, require_gate_record=False)
    assert any("did not touch rust/src/lib.rs" in p for p in problems)


def test_stubbed_part_needs_todo_and_run_notes_section(tmp_path: Path) -> None:
    root = workspace(tmp_path)
    rows = [row("p", "stubbed")]
    problems = wq.verify(rows, root)
    assert len(problems) == 2 and "no todo!()" in problems[0] and "RUN-NOTES.md" in problems[1]
    (root / "rust" / "src" / "lib.rs").write_text("pub fn f() { todo!() }\n")
    (root / "migration" / "RUN-NOTES.md").write_text("# notes\n\n## p\n\nlast error: E0308\n")
    assert wq.verify(rows, root) == []


def test_stage_is_read_from_the_queue_file_name() -> None:
    assert wq.stage_of(Path("migration/parts.tsv")) == 1
    assert wq.stage_of(Path("migration/parts-stage2.tsv")) == 2


def test_gate_rerun_after_the_commit_counts_while_target_is_unchanged(tmp_path: Path) -> None:
    root = workspace(tmp_path)
    (root / "rust" / "src" / "lib.rs").write_text("pub fn f() { /* v2 */ }\n")
    git(root, "add", ".")
    git(root, "commit", "-q", "-m", "part")
    commit = git(root, "rev-parse", "HEAD")
    (root / "other.rs").write_text("later\n")
    git(root, "add", ".")
    git(root, "commit", "-q", "-m", "unrelated later work")
    gate.record_pass(root, "p", 1, [""])  # re-gated at the newer HEAD
    assert wq.verify([row("p", "done", commit)], root, stage=1) == []
    (root / "rust" / "src" / "lib.rs").write_text("pub fn f() { /* v3 */ }\n")
    git(root, "add", ".")
    git(root, "commit", "-q", "-m", "target changed")
    (root / "migration" / "gate-passed.tsv").unlink()
    gate.record_pass(root, "p", 1, [""])  # a pass of different content must not vouch for the old commit
    assert wq.verify([row("p", "done", commit)], root, stage=1)
