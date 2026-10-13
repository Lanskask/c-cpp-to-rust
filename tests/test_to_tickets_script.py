"""skill/to-tickets: the generic local-tickets script and its installer (loaded by path: `tickets` is taken)."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path
from types import ModuleType

ROOT = Path(__file__).resolve().parent.parent / "skill" / "to-tickets"


def load(name: str, rel: str) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, ROOT / rel)
    assert spec and spec.loader
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


gen = load("to_tickets_script", "scripts/tickets.py")
installer = load("to_tickets_install", "install.py")

SPEC = [
    {
        "slug": "parse",
        "title": "Parse the input",
        "build": "A user can load a file.",
        "blocked_by": [],
        "tasks": [{"text": "Write the reader", "how": ["start from reader.py"]}, "Add a fixture"],
        "acceptance": ["the fixture loads"],
    },
    {
        "slug": "report",
        "title": "Report",
        "build": "Records print as a table.",
        "blocked_by": ["parse"],
        "tasks": ["Format the table"],
        "acceptance": ["output matches the snapshot"],
    },
]


def setup(tmp: Path) -> Path:
    spec = tmp / "spec.json"
    spec.write_text(json.dumps(SPEC))
    d = tmp / "issues"
    assert gen.main(["--dir", str(d), "init", "--spec", str(spec)]) == 0
    return d


def test_init_writes_numbered_tickets_script_and_makefile(tmp_path: Path) -> None:
    d = setup(tmp_path)
    assert sorted(p.name for p in d.glob("*")) == ["01-parse.md", "02-report.md", "Makefile", "tickets.py"]
    second = (d / "02-report.md").read_text()
    assert "**Blocked by:** 01 Parse the input" in second and "**Status:** todo" in second
    assert "- [ ] Write the reader\n  - start from reader.py" in (d / "01-parse.md").read_text()
    r = subprocess.run(["make", "-C", str(d), "status"], capture_output=True, text=True)
    assert r.returncode == 0 and "01\ttodo\tREADY\tparse" in r.stdout and "02\ttodo\t\treport" in r.stdout


def test_blocking_ticking_and_briefs(tmp_path: Path, capsys) -> None:  # noqa: ANN001
    d = setup(tmp_path)
    run = lambda *a: gen.main(["--dir", str(d), *a])  # noqa: E731
    assert run("mark", "02", "in_progress") == 1  # blocked by 01
    assert run("mark", "01", "in_progress") == 0
    assert run("mark", "01", "done", "--brief", "reader and fixture") == 1  # open tasks
    assert run("tick", "01", "fixture") == 1  # two open tasks contain it: ambiguous
    assert run("tick", "01", "reader") == 0 and run("tick", "01", "Add a fixture") == 0
    assert run("tick", "01", "fixture") == 0  # an acceptance criterion is ticked like a task
    assert run("mark", "01", "done") == 1  # no brief
    assert run("mark", "01", "done", "--brief", "reader committed, 3 tests") == 0
    assert "## Done" in (d / "01-parse.md").read_text()
    capsys.readouterr()
    assert run("mark", "02", "in_progress") == 0
    assert "reader committed, 3 tests" in capsys.readouterr().out  # blocker brief printed


def test_spec_validation() -> None:
    import pytest

    bad_order = [{**SPEC[0], "blocked_by": ["report"]}, SPEC[1]]
    with pytest.raises(ValueError, match="earlier ticket"):
        gen.parse_spec(bad_order)
    with pytest.raises(ValueError, match="acceptance"):
        gen.parse_spec([{**SPEC[0], "acceptance": []}])


def test_init_refuses_to_overwrite_without_force(tmp_path: Path) -> None:
    d = setup(tmp_path)
    assert gen.main(["--dir", str(d), "init", "--spec", str(tmp_path / "spec.json")]) == 1


def test_installer_is_idempotent(tmp_path: Path) -> None:
    target = tmp_path / "to-tickets"
    target.mkdir()
    (target / "SKILL.md").write_text("# To Tickets\n\nbody\n")
    installer.install(target)
    installer.install(target)
    text = (target / "SKILL.md").read_text()
    assert text.count(installer.BEGIN) == 1 and "tickets.py" in text and text.startswith("# To Tickets")
    assert (target / "scripts" / "tickets.py").is_file()
