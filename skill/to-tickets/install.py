#!/usr/bin/env python3
"""Install the local-tickets script and its SKILL.md section into the mattpocock `to-tickets` skill.

The plugin cache is replaced on a plugin update, so this repo keeps the canonical copy; run again after an update
(`make install-to-tickets`). Idempotent: the SKILL.md block between the markers is replaced, never duplicated.
`--target DIR` names the `to-tickets` skill directory (default: found through ~/.claude/plugins/installed_plugins.json).
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from pathlib import Path

BEGIN = "<!-- local-tickets-script:begin (installed from settlers_to_rust/skill/to-tickets; edit there) -->"
END = "<!-- local-tickets-script:end -->"
HERE = Path(__file__).resolve().parent


def find_target() -> Path:
    registry = Path.home() / ".claude" / "plugins" / "installed_plugins.json"
    plugins = json.loads(registry.read_text())["plugins"]
    for key, entries in plugins.items():
        if key.startswith("mattpocock-skills@"):
            for entry in entries:
                target = Path(entry["installPath"]) / "skills" / "engineering" / "to-tickets"
                if target.is_dir():
                    return target
    raise SystemExit("install: mattpocock-skills to-tickets not found; pass --target")


def install(target: Path) -> None:
    skill_md = target / "SKILL.md"
    if not skill_md.is_file():
        raise SystemExit(f"install: {skill_md} not found")
    (target / "scripts").mkdir(exist_ok=True)
    shutil.copyfile(HERE / "scripts" / "tickets.py", target / "scripts" / "tickets.py")
    block = f"{BEGIN}\n{(HERE / 'SKILL-addendum.md').read_text().rstrip()}\n{END}\n"
    text = skill_md.read_text()
    if BEGIN in text and END in text:
        head, rest = text.split(BEGIN, 1)
        _, tail = rest.split(END, 1)
        text = head + block + tail.lstrip("\n")
    else:
        text = text.rstrip() + "\n\n" + block
    skill_md.write_text(text)
    print(f"install: {target} updated (scripts/tickets.py, SKILL.md section)", file=sys.stderr)


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--target", type=Path)
    args = p.parse_args(argv)
    install(args.target or find_target())
    return 0


if __name__ == "__main__":
    sys.exit(main())
