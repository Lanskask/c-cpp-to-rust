"""workqueue.py: dependency order, resume, retry cap, verify against files and git."""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest
import workqueue as wq


def row(pid: str, state: str = "todo", deps: str = "", **kw: str) -> wq.Row:
    base = dict.fromkeys(wq.COLUMNS, "")
    base.update(part_id=pid, kind="file", source_paths="a.c", target_path=f"rust/{pid}.rs", state=state, deps=deps)
    base["attempts"] = "0"
    base.update(kw)
    return base


def test_next_respects_deps_and_resumes_in_progress() -> None:
    rows = [row("a", "done"), row("b", deps="a"), row("c", deps="b"), row("d", "in_progress")]
    assert [r["part_id"] for r in wq.next_parts(rows, 5)] == ["d", "b"]


def test_stubbed_dep_counts_as_finished() -> None:
    rows = [row("a", "stubbed"), row("b", deps="a")]
    assert [r["part_id"] for r in wq.next_parts(rows, 1)] == ["b"]


def test_fifth_failure_stubs_the_part() -> None:
    rows = [row("a")]
    for _ in range(wq.MAX_ATTEMPTS):
        wq.mark(rows, "a", "in_progress", None, None)
        wq.mark(rows, "a", "failed", None, "E0308\tmismatched")
    assert rows[0]["state"] == "stubbed"
    assert "\t" not in rows[0]["last_error"]


def test_roundtrip_and_bad_header(tmp_path: Path) -> None:
    p = tmp_path / "parts.tsv"
    wq.write_parts(p, [row("a"), row("b", deps="a")])
    assert [r["part_id"] for r in wq.read_parts(p)] == ["a", "b"]
    p.write_text("part_id\tkind\n")
    with pytest.raises(ValueError):
        wq.read_parts(p)


def test_verify_checks_output_and_commit(tmp_path: Path) -> None:
    def git(*a: str) -> str:
        return subprocess.run(["git", "-C", str(tmp_path), *a], check=True, capture_output=True, text=True).stdout

    git("init", "-q")
    (tmp_path / "rust").mkdir()
    (tmp_path / "rust" / "a.rs").write_text("fn main() {}\n")
    (tmp_path / "rust" / "b.rs").write_text("")
    git("add", ".")
    git("-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "x")
    head = git("rev-parse", "HEAD").strip()
    rows = [row("a", "done", commit=head), row("b", "done", commit=head), row("c", "done", commit="deadbeef")]
    problems = wq.verify(rows, tmp_path, require_gate_record=False)
    assert len(problems) == 3  # b empty, c missing file, c missing commit
    assert not any(p.startswith("a:") for p in problems)
