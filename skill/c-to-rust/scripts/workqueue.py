#!/usr/bin/env python3
"""Resumable Part queue over parts.tsv: status, next, mark, verify. Truth comes from files and git.

`parts.tsv` columns: part_id kind source_paths target_path tier deps state attempts model last_error commit updated.
`deps` is a comma-separated list of part_ids. Paths are relative to the workspace root (parent of `migration/`).
A Part is `done` only if its target file is non-empty, its commit exists and touched the target, and the gate
recorded a pass for it (`migration/gate-passed.tsv`, written by gate.py; `verify --no-gate-record` skips that
last check for Parts accepted before the record existed). A record taken after the commit also counts if the
target is unchanged between the commit and the recorded HEAD. A `stubbed` Part must contain `todo!()` and have a
`## <part>` section in RUN-NOTES.md (`verify`).
After MAX_ATTEMPTS failed attempts a Part becomes `stubbed` (PLAN.md §4 retry policy).
Exit code 0 on success, 1 on failure (verify: 1 if any done Part is not really done).
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from datetime import UTC, datetime
from pathlib import Path

COLUMNS = [
    "part_id",
    "kind",
    "source_paths",
    "target_path",
    "tier",
    "deps",
    "state",
    "attempts",
    "model",
    "last_error",
    "commit",
    "updated",
]
STATES = ["todo", "in_progress", "done", "failed", "stubbed"]
MAX_ATTEMPTS = 5

Row = dict[str, str]


def read_parts(path: Path) -> list[Row]:
    lines = [ln for ln in path.read_text().splitlines() if ln.strip()]
    if not lines or lines[0].split("\t") != COLUMNS:
        raise ValueError(f"{path}: header must be the tab-separated columns {COLUMNS}")
    rows: list[Row] = []
    for n, ln in enumerate(lines[1:], start=2):
        cells = ln.split("\t")
        if len(cells) != len(COLUMNS):
            raise ValueError(f"{path}:{n}: expected {len(COLUMNS)} columns, got {len(cells)}")
        rows.append(dict(zip(COLUMNS, cells, strict=True)))
    return rows


def write_parts(path: Path, rows: list[Row]) -> None:
    text = "\n".join("\t".join(r[c] for c in COLUMNS) for r in [dict(zip(COLUMNS, COLUMNS, strict=True)), *rows]) + "\n"
    with tempfile.NamedTemporaryFile("w", dir=path.parent, delete=False, suffix=".tmp") as f:
        f.write(text)
    Path(f.name).replace(path)  # atomic: a crash never leaves a half-written parts.tsv


def deps_of(row: Row) -> list[str]:
    return [d for d in row["deps"].split(",") if d]


def next_parts(rows: list[Row], batch: int) -> list[Row]:
    """in_progress Parts first (a crashed session resumes them), then todo Parts whose deps are all finished."""
    finished = {r["part_id"] for r in rows if r["state"] in ("done", "stubbed")}
    resume = [r for r in rows if r["state"] == "in_progress"]
    ready = [r for r in rows if r["state"] == "todo" and all(d in finished for d in deps_of(r))]
    return (resume + ready)[:batch]


def mark(rows: list[Row], part: str, state: str, commit: str | None, error: str | None) -> Row:
    row = next((r for r in rows if r["part_id"] == part), None)
    if row is None:
        raise ValueError(f"unknown part {part!r}")
    if state == "in_progress":
        row["attempts"] = str(int(row["attempts"] or "0") + 1)
    if state == "failed" and int(row["attempts"] or "0") >= MAX_ATTEMPTS:
        state = "stubbed"
    row["state"] = state
    if commit is not None:
        row["commit"] = commit
    if error is not None:
        row["last_error"] = " ".join(error.split())  # one line, no tabs
    row["updated"] = datetime.now(UTC).isoformat(timespec="seconds")
    return row


def git_has_commit(root: Path, commit: str) -> bool:
    r = subprocess.run(["git", "-C", str(root), "cat-file", "-e", f"{commit}^{{commit}}"], capture_output=True)
    return r.returncode == 0


def output_nonempty(path: Path) -> bool:
    if path.is_file():
        return path.stat().st_size > 0
    return path.is_dir() and any(p.is_file() and p.stat().st_size > 0 for p in path.rglob("*"))


def stage_of(parts: Path) -> int:
    m = re.search(r"stage(\d+)", parts.name)
    return int(m.group(1)) if m else 1


def git_is_ancestor(root: Path, ancestor: str, descendant: str) -> bool:
    r = subprocess.run(
        ["git", "-C", str(root), "merge-base", "--is-ancestor", ancestor, descendant], capture_output=True
    )
    return r.returncode == 0


def commit_touches(root: Path, commit: str, target: str) -> bool:
    r = subprocess.run(
        ["git", "-C", str(root), "show", "--name-only", "--format=", commit, "--", target],
        capture_output=True,
        text=True,
    )
    return r.returncode == 0 and bool(r.stdout.strip())


def unchanged_since(root: Path, commit: str, head: str, target: str) -> bool:
    """`target` has the same content at `head` as at `commit` (a gate re-run after the fact still counts)."""
    r = subprocess.run(["git", "-C", str(root), "diff", "--quiet", commit, head, "--", target], capture_output=True)
    return r.returncode == 0


def gate_records(root: Path) -> list[dict[str, str]]:
    path = root / "migration" / "gate-passed.tsv"
    if not path.is_file():
        return []
    lines = [ln.split("\t") for ln in path.read_text().splitlines() if ln.strip()]
    return [dict(zip(lines[0], cells, strict=False)) for cells in lines[1:]]


def verify(rows: list[Row], root: Path, stage: int = 1, require_gate_record: bool = True) -> list[str]:
    problems: list[str] = []
    records = gate_records(root)
    for r in rows:
        part = r["part_id"]
        if r["state"] == "stubbed":
            target = root / r["target_path"]
            if not target.is_file() or "todo!()" not in target.read_text():
                problems.append(f"{part}: stubbed but {r['target_path']} has no todo!()")
            notes = root / "migration" / "RUN-NOTES.md"
            if not notes.is_file() or not re.search(rf"^## {re.escape(part)}\b", notes.read_text(), re.M):
                problems.append(f"{part}: stubbed but RUN-NOTES.md has no '## {part}' section")
            continue
        if r["state"] != "done":
            continue
        if not output_nonempty(root / r["target_path"]):
            problems.append(f"{part}: target {r['target_path']} missing or empty")
        if not r["commit"] or not git_has_commit(root, r["commit"]):
            problems.append(f"{part}: commit {r['commit'] or '(none)'} not found")
            continue
        if not commit_touches(root, r["commit"], r["target_path"]):
            problems.append(f"{part}: commit {r['commit'][:8]} did not touch {r['target_path']}")
        if require_gate_record:
            mine = [x for x in records if x.get("part") == part and x.get("stage") == str(stage)]
            if not any(
                x.get("head")
                and (
                    git_is_ancestor(root, x["head"], r["commit"])
                    or (
                        git_is_ancestor(root, r["commit"], x["head"])
                        and unchanged_since(root, r["commit"], x["head"], r["target_path"])
                    )
                )
                for x in mine
            ):
                problems.append(
                    f"{part}: no gate pass recorded for stage {stage} at or before commit {r['commit'][:8]}"
                )
    return problems


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--parts", default="migration/parts.tsv")
    sub = p.add_subparsers(dest="cmd", required=True)
    sub.add_parser("status")
    n = sub.add_parser("next")
    n.add_argument("--batch", type=int, default=1)
    m = sub.add_parser("mark")
    m.add_argument("part")
    m.add_argument("state", choices=STATES)
    m.add_argument("--commit")
    m.add_argument("--error")
    v = sub.add_parser("verify")
    v.add_argument(
        "--no-gate-record", action="store_true", help="do not require migration/gate-passed.tsv (legacy Parts)"
    )
    v.add_argument("--fix", action="store_true", help="reset Parts that fail verification to todo")
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    path = Path(args.parts)
    try:
        rows = read_parts(path)
        if args.cmd == "status":
            counts = {s: sum(r["state"] == s for r in rows) for s in STATES}
            print("\t".join(f"{s}={n}" for s, n in counts.items()))
            print(f"workqueue: {len(rows)} parts, {counts['done']} done", file=sys.stderr)
        elif args.cmd == "next":
            chosen = next_parts(rows, args.batch)
            for r in chosen:
                print(f"{r['part_id']}\t{r['state']}\t{r['tier']}\t{r['source_paths']}\t{r['target_path']}")
            print(f"workqueue: {len(chosen)} part(s) ready", file=sys.stderr)
        elif args.cmd == "mark":
            row = mark(rows, args.part, args.state, args.commit, args.error)
            write_parts(path, rows)
            print(f"workqueue: {row['part_id']} -> {row['state']} (attempts {row['attempts']})", file=sys.stderr)
        elif args.cmd == "verify":
            root = path.resolve().parent.parent
            problems = verify(rows, root, stage_of(path), not args.no_gate_record)
            for pr in problems:
                print(pr)
            if problems and args.fix:
                bad = {pr.split(":")[0] for pr in problems}
                for r in rows:
                    if r["part_id"] in bad:
                        r["state"], r["commit"] = "todo", ""
                write_parts(path, rows)
            print(f"workqueue: verify {'FAILED' if problems else 'ok'} ({len(problems)} problem(s))", file=sys.stderr)
            return 1 if problems else 0
    except (ValueError, OSError) as e:
        print(f"workqueue: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
