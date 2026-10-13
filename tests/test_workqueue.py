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


def test_done_without_a_recorded_attempt_is_refused() -> None:
    rows = [row("a")]
    with pytest.raises(ValueError, match="attempts is 0"):
        wq.mark(rows, "a", "done", "abc", None)
    wq.mark(rows, "a", "done", "abc", None, allow_unattempted=True)
    assert rows[0]["state"] == "done"


def test_retry_while_in_progress_counts_again() -> None:
    rows = [row("a")]
    wq.mark(rows, "a", "in_progress", None, None)
    wq.mark(rows, "a", "in_progress", None, None)
    assert wq.mark(rows, "a", "done", "abc", None)["attempts"] == "2"


def test_snapshot_log_raises_attempts_on_done_and_verify_flags_a_lower_value(tmp_path: Path) -> None:
    (tmp_path / "migration").mkdir()
    (tmp_path / "migration" / "attempts.tsv").write_text(
        "part\tstage\thead\ttime\n"
        "a\t2\th1\tt\n"
        "a\t2\th2\tt\n"
        "a\t2\th3\tt\n"
        "a\t1\th0\tt\n"  # another stage: not counted
        "b\t2\th4\tt\n"
    )
    assert wq.snapshot_count(tmp_path, "a", 2) == 3
    assert wq.snapshot_count(tmp_path, "a", 1) == 1
    assert wq.snapshot_count(tmp_path, "missing", 2) == 0
    parts = tmp_path / "migration" / "parts-stage2.tsv"
    rows = [row("a", "in_progress", attempts="1")]
    wq.write_parts(parts, rows)
    assert wq.main(["--parts", str(parts), "mark", "a", "done", "--commit", "abc"]) == 0
    assert wq.read_parts(parts)[0]["attempts"] == "3"  # 1 marked, 3 snapshots
    low = [row("a", "done", attempts="1", commit="abc")]
    problems = wq.verify(low, tmp_path, 2, require_gate_record=False)
    assert any("3 snapshot(s)" in p for p in problems)


def test_mark_done_refuses_a_gate_pass_that_excused_checks(tmp_path: Path) -> None:
    import gate

    ws = tmp_path
    (ws / "migration").mkdir()
    parts = ws / "migration" / "parts.tsv"
    parts.write_text("\t".join(wq.COLUMNS) + "\n" + "p\tfile\ta.c\tr.rs\tmechanical\t\tin_progress\t1\t\t\t\t\n")
    subprocess.run(["git", "-C", str(ws), "init", "-q"], check=True)
    subprocess.run(
        ["git", "-C", str(ws), "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "x"],
        check=True,
    )
    gate.record_pass(ws, "p", 1, [""], ["test[]", "harness[]"])
    assert wq.main(["--parts", str(parts), "mark", "p", "done"]) == 1
    assert wq.main(["--parts", str(parts), "mark", "p", "done", "--accept-excused"]) == 0
