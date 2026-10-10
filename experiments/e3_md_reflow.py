#!/usr/bin/env python3
"""E3: enforce ~/.claude/md-style.md on the repo's Markdown: no prose line over 120 columns, aligned tables.

Question: can the style be applied mechanically without changing any word? Conservative rules: lines are never
joined (CONTEXT.md relies on one-line-per-field); a prose or list line over 120 is wrapped at spaces outside
backtick spans with the continuation indented to the text start; headings, fences, front matter and short lines are
untouched; table cells are padded (escaped pipes respected). The script verifies word-for-word equality after the
rewrite and refuses to write a file that changed any word.
Answer (2026-10-10): see WORK-LOG 21.

Usage: python3 -I experiments/e3_md_reflow.py [--check] [FILES...]   (default: every tracked *.md)
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WIDTH = 120
PIPE = re.compile(r"(?<!\\)\|")
SEP = re.compile(r":?-+:?")
LIST = re.compile(r"^(\s*(?:[-*+]|\d+[.)])\s+|\s*>\s?)")
TOKEN = re.compile(r"(?:`[^`]*`|\S)+")  # a run of non-space chars; backtick spans may hold spaces


def cells_of(line: str) -> str:
    """The text between the outer pipes; a row may lack the closing pipe."""
    s = line.strip()[1:]
    return s[:-1] if s.endswith("|") and not s.endswith("\\|") else s


def pad(block: list[str]) -> list[str]:
    rows = [[c.strip() for c in PIPE.split(cells_of(ln))] for ln in block]
    n = max(len(r) for r in rows)
    rows = [r + [""] * (n - len(r)) for r in rows]
    widths = [max([len(r[i]) for r in rows if not SEP.fullmatch(r[i])] or [3]) for i in range(n)]
    out = []
    for r in rows:
        cells = [("-" * (w + 2)) if SEP.fullmatch(c) else f" {c.ljust(w)} " for c, w in zip(r, widths, strict=True)]
        out.append("|" + "|".join(cells) + "|")
    return out


def wrap(line: str) -> list[str]:
    m = LIST.match(line)
    prefix = m.group(1) if m else re.match(r"\s*", line).group(0)  # type: ignore[union-attr]
    cont = " " * len(prefix) if not prefix.strip().startswith(">") else prefix
    words = TOKEN.findall(line[len(prefix) :])
    out: list[str] = []
    cur = prefix
    for w in words:
        if cur.strip() and len(cur) + 1 + len(w) > WIDTH and cur != cont:
            out.append(cur.rstrip())
            cur = cont + w
        else:
            cur = (cur + w) if cur in (prefix, cont) else cur + " " + w
    out.append(cur.rstrip())
    return out


def reflow(text: str) -> str:
    lines = text.split("\n")
    out: list[str] = []
    fence = False
    front = lines[:1] == ["---"]
    i = 0
    while i < len(lines):
        ln = lines[i]
        if front:
            out.append(ln)
            front = not (i > 0 and ln == "---")
            i += 1
            continue
        if ln.lstrip().startswith(("```", "~~~")):
            fence = not fence
            out.append(ln)
            i += 1
            continue
        if fence:
            out.append(ln)
            i += 1
            continue
        if ln.startswith("|"):
            j = i
            while j < len(lines) and lines[j].startswith("|"):
                j += 1
            out.extend(pad(lines[i:j]))
            i = j
            continue
        if len(ln) > WIDTH and not ln.startswith("#") and not ln.lstrip().startswith(("<", "[")):
            out.extend(wrap(ln))
        else:
            out.append(ln)
        i += 1
    return "\n".join(out)


def words(text: str) -> list[str]:
    flat = re.sub(r"\|", " ", text)
    return [w for w in flat.split() if not SEP.fullmatch(w)]


def main() -> int:
    args = [a for a in sys.argv[1:] if a != "--check"]
    check = "--check" in sys.argv
    files = (
        args
        or subprocess.run(
            ["git", "-C", str(ROOT), "ls-files", "*.md"], capture_output=True, text=True, check=True
        ).stdout.split()
    )
    changed = 0
    for f in files:
        p = ROOT / f
        if p.is_symlink() or "/src/" in f or not p.is_file():
            continue
        old = p.read_text()
        new = reflow(old)
        if new == old:
            continue
        if words(old) != words(new):
            print(f"REFUSED (words changed): {f}", file=sys.stderr)
            continue
        changed += 1
        print(f"{'would change' if check else 'reflowed'}: {f}")
        if not check:
            p.write_text(new)
    return 0 if not (check and changed) else 1


if __name__ == "__main__":
    sys.exit(main())
