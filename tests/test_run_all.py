"""run.py --all: tickets are worked in order from an empty workspace; QUESTIONS.md stops; dry run spends nothing."""

from __future__ import annotations

import stat
import sys
from pathlib import Path

import pytest
import run

SCRIPTS = Path(run.__file__).resolve().parent

FAKE = """#!PYEXE
import json, os, re, subprocess, sys
prompt = sys.stdin.read()
ws = re.search(r"^Workspace: (.*)$", prompt, re.M)
text = "VERDICT: accept" if "adversarial reviewer" in prompt else "STATUS: done"
if "governor of a C/C++" in prompt:
    num = re.search(r"TICKET_NUM=(\\d\\d)", prompt).group(1)
    mig = os.path.join(ws.group(1), "migration")
    if num == "01":
        open(os.path.join(mig, "RULEBOOK.md"), "w").write("# RULEBOOK\\n\\nStatus: deviations-approved (2026-10-12)\\n")
    if num == "05":
        rows = open(os.path.join(mig, "parts.tsv")).read().splitlines()
        open(os.path.join(mig, "parts-stage2.tsv"), "w").write("\\n".join(rows) + "\\n")
    if os.environ.get("FAKE_ASK") == num:
        open(os.path.join(mig, "QUESTIONS.md"), "w").write("Which allocator model?\\n")
    else:
        subprocess.run(
            [sys.executable, "-I", os.environ["FAKE_SCRIPTS"] + "/tickets.py", "--ws", ws.group(1), "mark", num,
             "done", "--tick-all", "--brief", "fake governor"], check=True, capture_output=True)
elif "adversarial reviewer" not in prompt:
    open(os.environ["FAKE_TARGET"], "w").write("pub fn f() -> i32 { 1 }\\n")
print(json.dumps({"result": text}))
"""


def setup(tmp: Path, monkeypatch: pytest.MonkeyPatch) -> tuple[Path, Path]:
    ws = tmp / "proj"
    ws.mkdir()
    (ws / "a.c").write_text("int f(void){return 1;}\n")
    fake = tmp / "fake_claude.py"
    fake.write_text(FAKE.replace("PYEXE", sys.executable))
    fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
    (ws / "migration" / "rust" / "src").mkdir(parents=True)
    for k, v in {"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t", "GIT_COMMITTER_NAME": "t"}.items():
        monkeypatch.setenv(k, v)
    monkeypatch.setenv("GIT_COMMITTER_EMAIL", "t@t")
    monkeypatch.setenv("FAKE_SCRIPTS", str(SCRIPTS))
    monkeypatch.setenv("FAKE_TARGET", str(ws / "migration" / "rust" / "src" / "lib.rs"))
    return ws, fake


def argv(ws: Path, fake: Path, *extra: str) -> list[str]:
    return [str(ws), "--all", "--claude", str(fake), "--gate-cmd", "true", "--snapshot-cmd", "true", *extra]


def test_dry_run_prints_the_ticket_plan_and_spends_nothing(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    ws, fake = setup(tmp_path, monkeypatch)
    assert run.main(argv(ws, fake)) == 0
    out = capsys.readouterr().out
    assert "tickets.py" in out and "--lang c" in out and "rerun with --go" in out
    assert not (ws / "migration" / "tickets").exists()


def test_every_ticket_is_worked_from_an_empty_workspace(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    ws, fake = setup(tmp_path, monkeypatch)
    code = run.main(argv(ws, fake, "--go", "--deviations", "policy", "--max-attempts", "2"))
    assert code == 0
    states = run.ticket_status(ws)
    assert sorted(states) == ["01", "02", "03", "04", "05", "06", "07"] and set(states.values()) == {"done"}
    rows = (ws / "migration" / "parts.tsv").read_text().splitlines()[1].split("\t")
    assert rows[6] == "done"
    assert (ws / "migration" / ".git").is_dir() and not (ws / ".git").exists()


def test_governor_question_stops_the_run(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    ws, fake = setup(tmp_path, monkeypatch)
    monkeypatch.setenv("FAKE_ASK", "02")
    assert run.main(argv(ws, fake, "--go", "--deviations", "policy")) == run.STOP
    assert "Which allocator model?" in capsys.readouterr().out
    assert run.ticket_status(ws)["01"] == "done" and run.ticket_status(ws)["02"] != "done"
