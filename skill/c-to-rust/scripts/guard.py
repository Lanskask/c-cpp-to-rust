#!/usr/bin/env python3
"""PreToolUse hook: protect the Rulebook and the Judge only while a run is in fan-out (decision 44).

Wired in `assets/settings.json` for Edit|Write|MultiEdit|NotebookEdit|Bash. Claude Code passes the tool call as JSON
on stdin; exit 2 blocks it and the message on stderr goes back to the model. The stage-dependent paths of Pilot P
(`migration/RULEBOOK.md`, `migration/parts*.tsv`, `migration/harness/**`, `rust/Cargo.toml`, `rust/tests/*.rs`,
`rust/examples/**`) are refused only while ticket 04 (Stage 1 fan-out) or 06 (Stage 2 fan-out) of P is
`in_progress` in `pilots/P/migration/tickets/`. Outside fan-out they are writable: Mapping and Judge building must
write them. A Judge change during fan-out means: `tickets.py mark 04 todo`, commit the change, mark it in_progress.

Bash commands are checked crudely: a command that names a protected path and contains a write marker (`>`, `tee`,
`sed -i`, `mv`, `cp`, `rm`, `truncate`, `perl -i`) is refused. Writing through an interpreter (`python -c`) is not
detected. Standalone on purpose (runs with `python3 -I`; decision 30). Errors fail closed (exit 2).

`guard.py check PATH` prints ALLOW or REFUSE for PATH (working directory = repo root) and exits 0 / 1.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

PROTECTED = (
    re.compile(r"^pilots/([^/]+)/migration/RULEBOOK\.md$"),
    re.compile(r"^pilots/([^/]+)/migration/parts[^/]*\.tsv$"),
    re.compile(r"^pilots/([^/]+)/migration/harness/"),
    re.compile(r"^pilots/([^/]+)/rust/Cargo\.toml$"),
    re.compile(r"^pilots/([^/]+)/rust/tests/[^/]+\.rs$"),
    re.compile(r"^pilots/([^/]+)/rust/examples/"),
)
FANOUT_TICKETS = ("04", "06")
WRITE_MARKERS = re.compile(r"(>|\btee\b|\bsed\s+-i|\bmv\b|\bcp\b|\brm\b|\btruncate\b|\bperl\s+-i|\binstall\b)")
PATH_IN_COMMAND = re.compile(r"pilots/[^\s'\"`;|&)]+")


def fanout_active(root: Path, pilot: str) -> bool:
    tdir = root / "pilots" / pilot / "migration" / "tickets"
    for num in FANOUT_TICKETS:
        for f in tdir.glob(f"{num}-*.md"):
            m = re.search(r"^\*\*Status:\*\* (\w+)", f.read_text(), re.M)
            if m and m.group(1) == "in_progress":
                return True
    return False


def protected_pilot(rel: str) -> str | None:
    for rx in PROTECTED:
        m = rx.match(rel)
        if m:
            return m.group(1)
    return None


def relative(path: str, root: Path) -> str:
    p = Path(path)
    if p.is_absolute():
        try:
            return p.resolve().relative_to(root.resolve()).as_posix()
        except ValueError:
            return p.as_posix()
    return Path(*p.parts).as_posix().removeprefix("./")


def refusal(root: Path, rel: str) -> str | None:
    pilot = protected_pilot(rel)
    if pilot and fanout_active(root, pilot):
        return (
            f"guard: {rel} is read-only while ticket 04 or 06 of {pilot} is in_progress (fan-out). "
            f"For a Judge change: tickets.py mark 04 todo, commit the change, mark 04 in_progress again."
        )
    return None


def check_call(root: Path, tool: str, tool_input: dict[str, object]) -> str | None:
    if tool == "Bash":
        cmd = str(tool_input.get("command", ""))
        if not WRITE_MARKERS.search(cmd):
            return None
        for cand in PATH_IN_COMMAND.findall(cmd):
            msg = refusal(root, cand)
            if msg:
                return msg
        return None
    for key in ("file_path", "notebook_path", "path"):
        value = tool_input.get(key)
        if isinstance(value, str):
            msg = refusal(root, relative(value, root))
            if msg:
                return msg
    return None


def main(argv: list[str]) -> int:
    if argv[:1] == ["check"] and len(argv) == 2:
        root = Path.cwd()
        msg = refusal(root, relative(argv[1], root))
        print(msg or "ALLOW")
        return 1 if msg else 0
    try:
        data = json.load(sys.stdin)
        root = Path(str(data.get("cwd") or Path.cwd()))
        msg = check_call(root, str(data.get("tool_name", "")), data.get("tool_input") or {})
    except Exception as e:  # fail closed: a broken guard must be noticed, not skipped
        print(f"guard: internal error, refusing: {e}", file=sys.stderr)
        return 2
    if msg:
        print(msg, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
