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
    assert run(ws, "mark", "01", "done", "--tick-all") == 1  # no brief
    assert run(ws, "mark", "01", "done", "--tick-all", "--brief", "inventory and rulebook") == 0
    assert run(ws, "mark", "02", "in_progress") == 0
    capsys.readouterr()
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
    run(ws, "mark", "01", "done", "--tick-all", "--brief", "x")
    f.write_text(f.read_text().replace("- [x] every config-macro", "- [ ] every config-macro", 1))
    run(ws, "status", "--tasks")
    lines = capsys.readouterr().out.splitlines()
    assert lines[0] == "Ticket_01 done Mapping (Stage 0) (p)"
    assert "  - [x] inventory.tsv, edges.tsv, order.txt, parts.tsv exist" in lines
    assert "  - [ ] every config-macro has a RULEBOOK §3 row" in lines
    assert any(ln == "Ticket_02 todo READY Stage 1 Judge (p)" for ln in lines)


def test_done_requires_every_task_ticked(tmp_path: Path) -> None:
    ws = tmp_path / "p"
    run(ws, "init")
    f = next((ws / "migration" / "tickets").glob("01-*.md"))
    f.write_text(f.read_text().replace("- [ ] ", "- [x] "))
    assert run(ws, "mark", "01", "done", "--brief", "b") == 0
    f.write_text(f.read_text().replace("- [x] the user approved", "- [ ] the user approved", 1))
    assert run(ws, "mark", "01", "done", "--brief", "b") == 1  # re-marking with an open task is refused too
    assert run(ws, "mark", "01", "done", "--tick-all", "--brief", "b") == 0
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


def test_tasks_carry_how_steps_and_blocker_names(tmp_path: Path) -> None:
    ws = tmp_path / "p"
    run(ws, "init")
    d = ws / "migration" / "tickets"
    t4 = next(d.glob("04-*.md")).read_text()
    assert "**Blocked by:** 03 Rulebook stress test" in t4
    assert "## Tasks" in t4 and "## Acceptance" in t4
    assert "- [ ] Port every Part in batches\n  - `workqueue.py" in t4
    for spec in tickets.CHAIN:
        assert spec.tasks and all(t.text for t in spec.tasks)
        assert all(sum(ln.startswith("- [ ]") for ln in t.text.splitlines()) <= 1 for t in spec.tasks)


def test_how_steps_name_existing_files() -> None:
    root = Path(tickets.__file__).resolve().parent.parent
    import re

    for spec in tickets.CHAIN:
        for t in spec.tasks:
            for step in t.how:
                for ref in re.findall(r"`(references/[\w.-]+\.md)`", step):
                    assert (root / ref).is_file(), (spec.num, ref)


def test_done_writes_brief_below_the_rule_and_show_prints_blocker_briefs(tmp_path: Path, capsys) -> None:  # noqa: ANN001
    ws = tmp_path / "p"
    run(ws, "init")
    run(ws, "mark", "01", "done", "--tick-all", "--brief", "41 Parts done, commit 4cea277\n- gate 3881/3881")
    f = next((ws / "migration" / "tickets").glob("01-*.md"))
    text = f.read_text()
    above, _, below = text.partition("\n-----------\n")
    assert "**Status:** done" in above and "- [ ]" not in above
    assert "## Done" in below and "- 41 Parts done, commit 4cea277" in below and "- gate 3881/3881" in below
    capsys.readouterr()
    assert run(ws, "mark", "02", "in_progress") == 0
    out = capsys.readouterr().out
    assert "== brief of 01 Mapping (Stage 0)" in out and "41 Parts done" in out
    assert run(ws, "show", "02") == 0
    shown = capsys.readouterr().out
    assert "# 02: Stage 1 Judge" in shown and "== brief of 01 Mapping (Stage 0)" in shown


def test_brief_is_required_and_bounded(tmp_path: Path) -> None:
    ws = tmp_path / "p"
    run(ws, "init")
    assert run(ws, "mark", "01", "done", "--tick-all", "--brief", "   ") == 1
    assert run(ws, "mark", "01", "done", "--tick-all", "--brief", "\n".join(["a"] * 9)) == 1
    assert run(ws, "mark", "01", "done", "--tick-all", "--brief", "x" * 130) == 1
    assert "- [ ]" in next((ws / "migration" / "tickets").glob("01-*.md")).read_text()  # refused: nothing written


def test_legacy_ticket_without_tasks_or_brief_still_works(tmp_path: Path, capsys) -> None:  # noqa: ANN001
    ws = tmp_path / "p"
    d = ws / "migration" / "tickets"
    d.mkdir(parents=True)
    (d / "01-a.md").write_text(
        "# 01: A (p)\n\n**What to build:** x\n\n**Blocked by:** None\n\n**Status:** done\n\n- [x] one\n"
    )
    (d / "02-b.md").write_text(
        "# 02: B (p)\n\n**What to build:** y\n\n**Blocked by:** 01\n\n**Status:** todo\n\n- [ ] two\n"
    )
    assert run(ws, "mark", "02", "in_progress") == 0
    assert "== brief of 01 A\n(no brief yet)" in capsys.readouterr().out
    assert run(ws, "tick", "02", "two") == 0
    assert run(ws, "mark", "02", "done", "--brief", "did y") == 0


def test_cpp_init_adds_reference_build_tasks(tmp_path: Path) -> None:
    ws = tmp_path / "tinyjson"
    assert run(ws, "init", "--lang", "cpp") == 0
    tickets = ws / "migration" / "tickets"
    first = (tickets / "01-mapping.md").read_text()
    assert (
        "Pick and record the reference build" in first
        and "idiom-table-cpp.md" in first
        and "rulebook-template-cpp.md" in first
    )
    assert "Patch the upstream tests" in (tickets / "02-stage1-judge.md").read_text()
    c_ws = tmp_path / "cjson"
    assert run(c_ws, "init") == 0
    assert "reference build" not in (c_ws / "migration" / "tickets" / "01-mapping.md").read_text()
