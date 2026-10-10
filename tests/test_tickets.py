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
    assert run(ws, "mark", "01", "done") == 1  # open tasks
    assert run(ws, "mark", "01", "done", "--tick-all") == 0
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


def test_status_tasks_lists_checkboxes(tmp_path: Path, capsys) -> None:  # noqa: ANN001
    ws = tmp_path / "p"
    run(ws, "init")
    f = next((ws / "migration" / "tickets").glob("01-*.md"))
    f.write_text(f.read_text().replace("- [ ] inventory.tsv", "- [x] inventory.tsv", 1))
    run(ws, "mark", "01", "done", "--tick-all")
    f.write_text(f.read_text().replace("- [x] every config-macro", "- [ ] every config-macro", 1))
    run(ws, "status", "--tasks")
    lines = capsys.readouterr().out.splitlines()
    assert lines[0] == "Ticket_01 done Mapping (Stage 0) (p)"
    assert lines[1].startswith("  - [x] inventory.tsv") and lines[2].startswith("  - [ ] every config-macro")
    assert any(ln == "Ticket_02 todo READY Stage 1 Judge (p)" for ln in lines)


def test_done_requires_every_task_ticked(tmp_path: Path) -> None:
    ws = tmp_path / "p"
    run(ws, "init")
    f = next((ws / "migration" / "tickets").glob("01-*.md"))
    f.write_text(f.read_text().replace("- [ ] ", "- [x] "))
    assert run(ws, "mark", "01", "done") == 0
    f.write_text(f.read_text().replace("- [x] the user approved", "- [ ] the user approved", 1))
    assert run(ws, "mark", "01", "done") == 1  # re-marking a done ticket with an open task is refused too
    assert run(ws, "mark", "01", "done", "--tick-all") == 0
    assert "- [ ]" not in f.read_text()


def test_tick_needs_exactly_one_match(tmp_path: Path) -> None:
    ws = tmp_path / "p"
    run(ws, "init")
    assert run(ws, "tick", "01", "config-macro") == 0
    assert run(ws, "tick", "01", "config-macro") == 1  # already ticked: no open match
    assert run(ws, "tick", "01", "every") == 1  # several matches
    text = next((ws / "migration" / "tickets").glob("01-*.md")).read_text()
    assert "- [x] every config-macro" in text and "- [ ] inventory.tsv" in text


def test_init_writes_migration_makefile_that_runs(tmp_path: Path) -> None:
    import subprocess

    ws = tmp_path / "pilots" / "p"
    run(ws, "init")
    mk = ws / "migration" / "Makefile"
    assert mk.is_file()
    scripts = Path(tickets.__file__).resolve().parent
    for target, needle in (("tickets", "Ticket_01 todo READY"), ("status", "== tickets")):
        out = subprocess.run(
            ["make", "-s", "-C", str(ws / "migration"), target, f"SCRIPTS={scripts}"], capture_output=True, text=True
        )
        assert out.returncode == 0 and needle in out.stdout, out.stderr
