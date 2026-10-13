#!/usr/bin/env python3
"""Install the c-to-rust Skill as the global Skill `cpp-to-rust` (default `~/.claude/skills/cpp-to-rust`).

    python3 -I skill/install_global.py [--dest DIR] [--name cpp-to-rust]

Copies `skill/c-to-rust/` (SKILL.md, scripts/, references/, assets/; no caches) and rewrites what only worked from
this repository: every `skill/c-to-rust/` path becomes the installed directory, the Skill name becomes `--name`,
`make spend-now` / `make spend ...` become `spend.py` calls, the Pilot Makefile template finds its scripts in the
installed directory, and the guard hook in assets/settings.json runs the installed guard.py. The repository stays
the source of truth: rerun after a change. `~/.claude/settings.json` and permission rules are never touched.
"""

from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path

SRC = Path(__file__).resolve().parent / "c-to-rust"
TEXT = {".md", ".py", ".json", ".tsv", ".Makefile", ".txt"}


def rewrites(name: str, home_dir: str) -> list[tuple[str, str]]:
    """Ordered (old, new) substitutions; the specific ones come before the generic path and name ones."""
    scripts = f"{home_dir}/scripts"
    return [
        ("$(abspath $(MIGRATION)/../../../skill/c-to-rust/scripts)", f"$(HOME)/.claude/skills/{name}/scripts"),
        ("$CLAUDE_PROJECT_DIR/skill/c-to-rust/scripts/guard.py", f"$HOME/.claude/skills/{name}/scripts/guard.py"),
        ("`make spend-now`", f"`python3 -I {scripts}/spend.py now`"),
        (
            '`make spend PHASE=<phase> SINCE=<timestamp> NOTE="..."`',
            f'`python3 -I {scripts}/spend.py log --phase <phase> --since <timestamp> --note "..."`',
        ),
        ("`make spend PHASE=<phase> SINCE=<timestamp> NOTE=...`", f"`python3 -I {scripts}/spend.py log ...`"),
        ("`make spend ...`", f"`python3 -I {scripts}/spend.py log ...`"),
        ("skill/c-to-rust/", f"{home_dir}/"),
        ("c-to-rust", name),
    ]


def install(dest: Path, name: str) -> list[str]:
    if dest.exists():
        shutil.rmtree(dest)
    shutil.copytree(SRC, dest, ignore=shutil.ignore_patterns("__pycache__", "*.pyc", ".DS_Store"))
    home_dir = f"~/.claude/skills/{name}"
    changed: list[str] = []
    for f in sorted(dest.rglob("*")):
        if not f.is_file() or (f.suffix not in TEXT and f.name != "migration.Makefile"):
            continue
        text = f.read_text()
        new = text
        for old, rep in rewrites(name, home_dir):
            new = new.replace(old, rep)
        if new != text:
            f.write_text(new)
            changed.append(str(f.relative_to(dest)))
    return changed


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--name", default="cpp-to-rust")
    p.add_argument("--dest", help="default ~/.claude/skills/<name>")
    a = p.parse_args(argv)
    dest = Path(a.dest) if a.dest else Path.home() / ".claude" / "skills" / a.name
    if not SRC.is_dir():
        print(f"install_global: {SRC} missing", file=sys.stderr)
        return 1
    changed = install(dest, a.name)
    print(f"install_global: {dest} ({len(changed)} files rewritten)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
