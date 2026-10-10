#!/usr/bin/env python3
"""Translation-memory cache (SQLite FTS5): build from migration/ and git, search by error or snippet.

The cache is disposable (decision 15): `build` recreates it from files, never the other way round.
Indexed documents (kind: key):
  pair:   accepted Parts (state=done in parts.tsv): source text and the Rust at the Part's commit
  error:  `last_error` of failed/stubbed Parts
  note:   sections of migration/RUN-NOTES.md (split on `## ` headings)
  commit: git log subject and body of commits that touched a Part's target path
`--db` is migration/tm.sqlite by default; the workspace root is the parent of its directory.
`search "<error or snippet>"` prints the best matches as TSV: kind, key, snippet.
Exit code 0 on success, 1 on failure (including: SQLite without FTS5).
"""

from __future__ import annotations

import argparse
import re
import sqlite3
import subprocess
import sys
from pathlib import Path

Doc = tuple[str, str, str]  # kind, key, body


def read_parts(path: Path) -> list[dict[str, str]]:
    lines = [ln.split("\t") for ln in path.read_text().splitlines() if ln.strip()]
    return [dict(zip(lines[0], cells, strict=False)) for cells in lines[1:]] if lines else []


def git(root: Path, *args: str) -> str:
    r = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else ""


def collect(root: Path) -> list[Doc]:
    docs: list[Doc] = []
    parts_file = root / "migration" / "parts.tsv"
    for r in read_parts(parts_file) if parts_file.exists() else []:
        pid = r.get("part_id", "")
        if r.get("state") == "done" and r.get("commit"):
            src = "\n".join(
                (root / s).read_text(errors="replace")
                for s in r.get("source_paths", "").split(",")
                if (root / s).is_file()
            )
            rust = git(root, "show", f"{r['commit']}:{r.get('target_path', '')}") or ""
            if not rust and (root / r.get("target_path", "")).is_file():
                rust = (root / r["target_path"]).read_text(errors="replace")
            docs.append(("pair", pid, f"{src}\n---\n{rust}"))
        if r.get("last_error"):
            docs.append(("error", pid, r["last_error"]))
        tp = r.get("target_path", "")
        if tp:
            for entry in git(root, "log", "--format=%h%x1f%s%n%b%x1e", "--", tp).split("\x1e"):
                if "\x1f" in entry:
                    h, body = entry.strip().split("\x1f", 1)
                    docs.append(("commit", f"{pid}@{h}", body))
    notes = root / "migration" / "RUN-NOTES.md"
    if notes.exists():
        for sec in re.split(r"^## ", notes.read_text(), flags=re.M)[1:]:
            title, _, body = sec.partition("\n")
            docs.append(("note", title.strip(), body))
    return docs


def fts_query(text: str) -> str:
    words = re.findall(r"[A-Za-z0-9_]{2,}", text)
    unique = list(dict.fromkeys(words))[:20]
    return " OR ".join(f'"{w}"' for w in unique)


def build(db: Path, root: Path) -> int:
    db.parent.mkdir(parents=True, exist_ok=True)
    db.unlink(missing_ok=True)
    con = sqlite3.connect(db)
    try:
        con.execute("CREATE VIRTUAL TABLE tm USING fts5(kind, key, body)")
    except sqlite3.OperationalError as e:
        print(f"tm: this SQLite has no FTS5 ({e})", file=sys.stderr)
        return -1
    docs = collect(root)
    con.executemany("INSERT INTO tm VALUES (?, ?, ?)", docs)
    con.commit()
    con.close()
    return len(docs)


def search(db: Path, query: str, limit: int = 5) -> list[tuple[str, str, str]]:
    q = fts_query(query)
    if not q:
        return []
    con = sqlite3.connect(db)
    rows = con.execute(
        "SELECT kind, key, snippet(tm, 2, '[', ']', ' ... ', 24) FROM tm WHERE tm MATCH ? ORDER BY bm25(tm) LIMIT ?",
        (q, limit),
    ).fetchall()
    con.close()
    return [(k, key, " ".join(sn.split())) for k, key, sn in rows]


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--db", default="migration/tm.sqlite")
    sub = p.add_subparsers(dest="cmd", required=True)
    sub.add_parser("build")
    s = sub.add_parser("search")
    s.add_argument("query")
    s.add_argument("--limit", type=int, default=5)
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    db = Path(args.db)
    if args.cmd == "build":
        n = build(db, db.resolve().parent.parent)
        if n < 0:
            return 1
        print(f"tm: indexed {n} document(s) -> {db}", file=sys.stderr)
        return 0
    if not db.exists():
        print(f"tm: {db} missing; run `tm.py build` first", file=sys.stderr)
        return 1
    hits = search(db, args.query, args.limit)
    for h in hits:
        print("\t".join(h))
    print(f"tm: {len(hits)} match(es)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
