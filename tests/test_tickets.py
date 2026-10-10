"""tickets.py: the seven-ticket chain of a translation run."""

from __future__ import annotations

from pathlib import Path

import tickets


def run(ws: Path, *a: str) -> int:
    return tickets.main(["--ws", str(ws), *a])


def test_init_writes_seven_tickets_and_refuses_overwrite(tmp_path: Path) -> None:
    ws = tmp_path / "cjson"
    assert run(ws, "init") == 0
    names = sorted(p.name for p in (ws / "migration" / "tickets").glob("*.md"))
    assert len(names) == 7 and names[0] == "01-mapping.md" and names[-1] == "07-report.md"
    assert "(cjson)" in (ws / "migration" / "tickets" / names[0]).read_text()
    assert run(ws, "init") == 1
    assert run(ws, "init", "--force") == 0


def test_blocked_ticket_cannot_start_and_status_shows_ready(tmp_path: Path, capsys) -> None:  # noqa: ANN001
    ws = tmp_path / "p"
    run(ws, "init")
    assert run(ws, "mark", "02", "in_progress") == 1  # blocked by 01
    assert run(ws, "mark", "01", "done") == 0
    assert run(ws, "mark", "02", "in_progress") == 0
    run(ws, "status")
    rows = [ln.split("\t") for ln in capsys.readouterr().out.splitlines()]
    assert rows[0][:2] == ["01", "done"] and rows[1][:2] == ["02", "in_progress"]
    assert [r[0] for r in rows if r[2] == "READY"] == []  # 02 is running, 03 waits on it


def test_ready_marks_first_unblocked_todo(tmp_path: Path, capsys) -> None:  # noqa: ANN001
    ws = tmp_path / "p"
    run(ws, "init")
    run(ws, "status")
    rows = [ln.split("\t") for ln in capsys.readouterr().out.splitlines()]
    assert [r[0] for r in rows if r[2] == "READY"] == ["01"]


def test_status_without_tickets_fails(tmp_path: Path) -> None:
    assert run(tmp_path / "none", "status") == 1
