"""run.py: preconditions, dry run spends nothing, one Part accepted with a fake `claude`, review BLOCK retried."""

from __future__ import annotations

import os
import stat
import subprocess
import sys
from pathlib import Path

import pytest
import run

PARTS_HEADER = "\t".join(
    ["part_id", "kind", "source_paths", "target_path", "tier", "deps", "state", "attempts", "model", "last_error"]
    + ["commit", "updated"]
)

FAKE = """#!PYEXE
import json, os, sys
prompt = sys.stdin.read()
log = os.environ["FAKE_LOG"]
if "adversarial reviewer" in prompt:
    count_file = log + ".reviews"
    n = int(open(count_file).read()) if os.path.exists(count_file) else 0
    open(count_file, "w").write(str(n + 1))
    bad = os.environ.get("FAKE_BLOCK_FIRST") == "1" and n == 0
    text = "BLOCK\\t1\\tlib.rs:1\\twrong\\tinput\\nVERDICT: fix" if bad else "VERDICT: accept"
else:
    open(os.environ["FAKE_TARGET"], "w").write("pub fn f() -> i32 { 1 }\\n")
    text = "STATUS: done"
open(log, "a").write("call\\n")
print(json.dumps({"result": text}))
"""


def make_ws(tmp: Path, rulebook_status: str = "deviations-approved (2026-10-12)", ticket4: str = "todo") -> Path:
    root = tmp
    ws = root / "pilots" / "p"
    (ws / "migration" / "tickets").mkdir(parents=True)
    (ws / "src").mkdir()
    (ws / "src" / "a.c").write_text("int f(void){return 1;}\n")
    (ws / "rust" / "src").mkdir(parents=True)
    (ws / "rust" / "src" / "lib.rs").write_text("// stub\n")
    (ws / "migration" / "RULEBOOK.md").write_text(
        f"# RULEBOOK: p\n\nStatus: {rulebook_status}\n\n## 0. Scope\n\nx\n\n## 8. Deviations\n\n"
        "| id | a | b | c | Status |\n|---|---|---|---|---|\n| D1 | a | b | no | approved (policy) |\n"
    )
    (ws / "migration" / "parts.tsv").write_text(
        PARTS_HEADER + "\n" + "p\tfile\ta.c\trust/src/lib.rs\tmechanical\t\ttodo\t0\t\t\t\t\n"
    )
    for num, status in (("01", "done"), ("02", "done"), ("03", "done"), ("04", ticket4)):
        (ws / "migration" / "tickets" / f"{num}-x.md").write_text(f"# {num}\n\n**Status:** {status}\n")
    for cmd in (["init", "-q"], ["add", "."], ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "base"]):
        subprocess.run(["git", "-C", str(root), *cmd], check=True, capture_output=True)
    return ws


def fake_claude(tmp: Path) -> Path:
    f = tmp / "fake_claude.py"
    f.write_text(FAKE.replace("PYEXE", sys.executable))
    f.chmod(f.stat().st_mode | stat.S_IEXEC)
    return f


def argv(ws: Path, claude: Path, *extra: str) -> list[str]:
    return [
        str(ws),
        "--claude",
        str(claude),
        "--gate-cmd",
        "true",
        "--snapshot-cmd",
        "true",
        "--root",
        str(ws.parent.parent),
        *extra,
    ]


def test_missing_preconditions_stop_without_spending(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    ws = make_ws(tmp_path, rulebook_status="draft")
    assert run.main(argv(ws, fake_claude(tmp_path), "--go", "--deviations", "policy")) == run.STOP
    assert "RULEBOOK.md status is draft" in capsys.readouterr().err


def test_proposed_deviation_row_stops_for_the_user(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    ws = make_ws(tmp_path)
    rb = ws / "migration" / "RULEBOOK.md"
    rb.write_text(rb.read_text().replace("approved (policy)", "proposed"))
    assert run.main(argv(ws, fake_claude(tmp_path))) == run.STOP
    assert "D1" in capsys.readouterr().err


def test_policy_rows_need_the_deviations_flag(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    ws = make_ws(tmp_path)  # D1 is `approved (policy)`
    assert run.main(argv(ws, fake_claude(tmp_path), "--deviations", "ask")) == run.STOP
    assert "--deviations policy" in capsys.readouterr().err
    assert run.main(argv(ws, fake_claude(tmp_path), "--deviations", "policy")) == 0


def test_dry_run_prints_the_plan_and_runs_nothing(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    ws = make_ws(tmp_path)
    os.environ["FAKE_LOG"] = str(tmp_path / "calls")
    assert run.main(argv(ws, fake_claude(tmp_path), "--deviations", "policy")) == 0
    out = capsys.readouterr().out
    assert "--model sonnet" in out and "rerun with --go" in out
    assert not (tmp_path / "calls").exists()


def test_one_part_is_accepted_committed_and_marked_done(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    ws = make_ws(tmp_path)
    monkeypatch.setenv("FAKE_LOG", str(tmp_path / "calls"))
    monkeypatch.setenv("FAKE_TARGET", str(ws / "rust" / "src" / "lib.rs"))
    code = run.main(argv(ws, fake_claude(tmp_path), "--go", "--deviations", "policy"))
    assert code == 0
    rows = (ws / "migration" / "parts.tsv").read_text().splitlines()[1].split("\t")
    assert rows[6] == "done" and rows[10]  # state, commit
    assert "pub fn f" in (ws / "rust" / "src" / "lib.rs").read_text()
    assert (tmp_path / "WORK-LOG.md").read_text().count("run.py p stage 1") == 1


def test_review_block_costs_an_attempt_then_passes(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    ws = make_ws(tmp_path)
    monkeypatch.setenv("FAKE_LOG", str(tmp_path / "calls"))
    monkeypatch.setenv("FAKE_TARGET", str(ws / "rust" / "src" / "lib.rs"))
    monkeypatch.setenv("FAKE_BLOCK_FIRST", "1")
    assert run.main(argv(ws, fake_claude(tmp_path), "--go", "--deviations", "policy")) == 0
    row = (ws / "migration" / "parts.tsv").read_text().splitlines()[1].split("\t")
    assert row[6] == "done" and row[7] == "2"  # two attempts


def test_rust_items_extracts_types_and_signatures() -> None:
    text = (
        "#[repr(C)]\npub struct T {\n    a: i32,\n}\npub const N: i32 = 3;\n"
        'pub unsafe extern "C" fn go(x: *mut T) -> i32 {\n 0\n}\n'
    )
    items, sigs = run.rust_items(text)
    assert any("struct T" in i and "a: i32" in i for i in items)
    assert any("const N" in i for i in items)
    assert sigs == ['pub unsafe extern "C" fn go(x: *mut T) -> i32']


def test_rulebook_sections_keeps_only_the_named_ones() -> None:
    text = "# R\n\nStatus: x\n\n## 0. a\n\nzero\n\n## 6. b\n\nsix\n\n## 8. c\n\neight\n"
    got = run.rulebook_sections(text, (0, 8))
    assert "zero" in got and "eight" in got and "six" not in got and "Status: x" in got


def test_part_commit_lands_in_the_pilots_own_repository(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    ws = make_ws(tmp_path)
    for cmd in (["init", "-q"], ["add", "."], ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "pilot"]):
        subprocess.run(["git", "-C", str(ws), *cmd], check=True, capture_output=True)
    main_head = subprocess.run(["git", "-C", str(tmp_path), "rev-parse", "HEAD"], capture_output=True, text=True)
    monkeypatch.setenv("FAKE_LOG", str(tmp_path / "calls"))
    monkeypatch.setenv("FAKE_TARGET", str(ws / "rust" / "src" / "lib.rs"))
    assert run.main(argv(ws, fake_claude(tmp_path), "--go", "--deviations", "policy")) == 0
    log = subprocess.run(["git", "-C", str(ws), "log", "--format=%s"], capture_output=True, text=True).stdout
    assert "p: p stage 1 (run.py)" in log
    after = subprocess.run(["git", "-C", str(tmp_path), "rev-parse", "HEAD"], capture_output=True, text=True)
    assert after.stdout == main_head.stdout  # the main repo got no commit
