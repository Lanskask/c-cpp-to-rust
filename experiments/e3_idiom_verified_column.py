#!/usr/bin/env python3
"""E3: write the `verified` column of skill/c-to-rust/references/idiom-table-c.md from tests/idiom/RESULTS-*.tsv.

Question: which idiom-table rows does a running test exercise? Rows that were written from general knowledge stay
`no` until a test compiles the C and the Rust mapping and compares them (decision 37).
Answer (2026-10-10): 70 of 72 rows verified by tests/idiom (80 tests, `make idiom`); A6 and M4 are design rules
with no observable behaviour, so they stay `no` with a note. No row was disproved (see WORK-LOG 19 for the caveat).

It also narrows the `tags` of the rows in TAGS from `*` to the new inventory tag (G15), so a prompt only gets them
when the Part has the construct.

Usage: python3 -I experiments/e3_idiom_verified_column.py COMMIT   (repo root; rewrites the table, re-pads it)
"""

from __future__ import annotations

import csv
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TABLE = ROOT / "skill/c-to-rust/references/idiom-table-c.md"
RESULTS = sorted((ROOT / "tests/idiom").glob("RESULTS-*.tsv"))
ROW = re.compile(r"^\|\s*([A-Z]+\d+)\s*\|")
PIPE = re.compile(r"(?<!\\)\|")  # a cell may contain an escaped pipe `\|`

# rows whose `tags` cell is narrowed from `*` (always sent) to the inventory tag that makes them relevant (G15)
TAGS = {
    "T11": "bitfield",
    "T12": "fnptr",
    "I7": "float-format",
    "I8": "strtod",
    "L10": "qsort",
    "L4": "ctype",
    "L5": "ctype",
    "L6": "ctype",
}


def pad_tables(lines: list[str]) -> list[str]:
    out: list[str] = []
    block: list[list[str]] = []

    def flush() -> None:
        if not block:
            return
        n = max(len(r) for r in block)
        rows = [r + [""] * (n - len(r)) for r in block]
        widths = [max(len(r[i]) for r in rows if not re.fullmatch(r":?-+:?", r[i])) for i in range(n)]
        for r in rows:
            cells = [
                ("-" * (w + 2)) if re.fullmatch(r":?-+:?", c) else f" {c.ljust(w)} "
                for c, w in zip(r, widths, strict=True)
            ]
            out.append("|" + "|".join(cells) + "|")
        block.clear()

    for ln in lines:
        if ln.startswith("|"):
            block.append([c.strip() for c in PIPE.split(ln.strip()[1:-1])])
        else:
            flush()
            out.append(ln)
    flush()
    return out


def main() -> int:
    commit = sys.argv[1]
    status = {r["row_id"]: r for f in RESULTS for r in csv.DictReader(f.open(), delimiter="\t")}
    out = []
    for ln in TABLE.read_text().splitlines():
        m = ROW.match(ln)
        if m and m.group(1) in status:
            cells = [c.strip() for c in PIPE.split(ln.strip()[1:-1])]
            r = status[m.group(1)]
            cells[-1] = f"tests/idiom@{commit}" if r["status"] == "verified" else "no (design rule, no behaviour)"
            if m.group(1) in TAGS and cells[-2] == "*":
                cells[-2] = TAGS[m.group(1)]
            ln = "| " + " | ".join(cells) + " |"
        out.append(ln)
    TABLE.write_text("\n".join(pad_tables(out)) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
