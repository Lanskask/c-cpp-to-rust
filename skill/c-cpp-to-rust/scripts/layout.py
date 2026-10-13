#!/usr/bin/env python3
"""Where a workspace keeps its source and its Rust crate (decision 72).

`<ws>` is the folder the Skill works in. Everything the Skill creates is under `<ws>/migration/`; the two places
that differ between layouts are recorded in `<ws>/migration/layout.json` by `tickets.py init`:

    inplace  (default for a new workspace)  source = `<ws>` itself, crate = `<ws>/migration/rust`
    legacy   (workspaces made before decision 72: pilots/<name>)  source = `<ws>/src`, crate = `<ws>/rust`

Without a layout.json the layout is `legacy`, so an existing workspace keeps working. `source_paths` in parts.tsv
are relative to `source`; `target_path` is relative to `<ws>`.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

KINDS = {
    "inplace": {"source": ".", "crate": "migration/rust"},
    "legacy": {"source": "src", "crate": "rust"},
}


@dataclass(frozen=True)
class Layout:
    ws: Path
    kind: str
    source_rel: str
    crate_rel: str

    @property
    def source(self) -> Path:
        return (self.ws / self.source_rel).resolve()

    @property
    def crate(self) -> Path:
        return (self.ws / self.crate_rel).resolve()

    @property
    def migration(self) -> Path:
        return self.ws / "migration"


def load(ws: Path) -> Layout:
    ws = ws.resolve()
    f = ws / "migration" / "layout.json"
    if f.is_file():
        data = json.loads(f.read_text())
        return Layout(ws, str(data.get("kind", "custom")), str(data["source"]), str(data["crate"]))
    return Layout(ws, "legacy", **{f"{k}_rel": v for k, v in KINDS["legacy"].items()})


def detect_kind(ws: Path) -> str:
    """`legacy` when the workspace already has `src/` or `rust/` next to `migration/`, else `inplace`."""
    return "legacy" if (ws / "src").is_dir() or (ws / "rust").is_dir() else "inplace"


def write(ws: Path, kind: str) -> Layout:
    if kind not in KINDS:
        raise ValueError(f"layout {kind!r}: use one of {', '.join(KINDS)}")
    (ws / "migration").mkdir(parents=True, exist_ok=True)
    (ws / "migration" / "layout.json").write_text(json.dumps({"kind": kind, **KINDS[kind]}, indent=2) + "\n")
    return load(ws)


def git_dir(ws: Path) -> Path:
    """Directory to run git in: `<ws>/migration` when it is its own repository (decision 73), else `<ws>`."""
    m = ws / "migration"
    return m if (m / ".git").exists() else ws


def in_repo(ws: Path, rel: str) -> str:
    """`rel` (relative to `<ws>`) as a path relative to `git_dir(ws)`."""
    if git_dir(ws) != ws and rel.startswith("migration/"):
        return rel.removeprefix("migration/")
    return rel


def init_repo(ws: Path) -> bool:
    """`git init` in `<ws>/migration` unless `<ws>` or `migration/` is already a repository root. True if created."""
    m = ws / "migration"
    if (m / ".git").exists():
        return False
    r = subprocess.run(["git", "-C", str(m), "init", "-q", "-b", "main"], capture_output=True, text=True)
    return r.returncode == 0


def commit(ws: Path, message: str) -> str:
    """Commit everything under `<ws>/migration` in its own repository; "" when there is nothing or no repository."""
    m = ws / "migration"
    if not (m / ".git").exists():
        return ""
    subprocess.run(["git", "-C", str(m), "add", "-A"], capture_output=True)
    if subprocess.run(["git", "-C", str(m), "diff", "--cached", "--quiet"]).returncode == 0:
        return ""
    r = subprocess.run(["git", "-C", str(m), "commit", "-q", "-m", message], capture_output=True, text=True)
    if r.returncode != 0:
        return f"commit failed: {r.stderr.strip()[:200]}"
    return subprocess.run(
        ["git", "-C", str(m), "rev-parse", "--short", "HEAD"], capture_output=True, text=True
    ).stdout.strip()


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("ws", type=Path, help="the workspace folder; prints its source and crate directories")
    a = p.parse_args(argv)
    lay = load(a.ws)
    print(f"kind\t{lay.kind}\nsource\t{lay.source}\ncrate\t{lay.crate}\nmigration\t{lay.migration}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
