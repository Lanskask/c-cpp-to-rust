#!/usr/bin/env python3
"""E3 stress test, prompt builder: fill skill/c-to-rust/references/stage1-prompt.md for the jsmn Part.

Question: when two agents translate jsmn.h with the same Stage 1 prompt (LLM engine, no c2rust text), which
decisions do they take differently? Each such decision is a missing rule (SKILL.md step 2, decision 36).
Answer: see pilots/jsmn/migration/RUN-NOTES.md, "## Rule proposals".

Usage: python3 -I e3_jsmn_stress_prompt.py TARGET_PATH OUT_PROMPT
Slots filled as stage1-prompt.md says: rulebook §0,2,3,4,5,7,8,9,11; idiom rows tagged `*`, `goto` or
`config-macro:*`; c2rust = none; previous = empty; tm_hits = none; no callees or structs (single Part).
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PROMPT = ROOT / "skill/c-to-rust/references/stage1-prompt.md"
RULEBOOK = ROOT / "pilots/jsmn/migration/RULEBOOK.md"
IDIOMS = ROOT / "skill/c-to-rust/references/idiom-table-c.md"
SOURCE = ROOT / "pilots/jsmn/src/jsmn.h"
SECTIONS = {0, 2, 3, 4, 5, 7, 8, 9, 11}
TAGS = {"*", "goto"}


def rulebook_sections() -> str:
    parts = re.split(r"(?m)^(?=## \d+\. )", RULEBOOK.read_text())
    keep = [p for p in parts if (m := re.match(r"## (\d+)\. ", p)) and int(m.group(1)) in SECTIONS]
    return "".join(keep)


def idiom_rows() -> str:
    rows = []
    for line in IDIOMS.read_text().splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 7 or not re.fullmatch(r"[A-Z]+\d+", cells[0]):
            continue
        tags = {t.strip() for t in cells[-2].split(",")}
        if tags & TAGS or any(t.startswith("config-macro") for t in tags):
            rows.append(line)
    return "\n".join(rows)


def main() -> int:
    target, out = sys.argv[1], sys.argv[2]
    text = PROMPT.read_text()
    template = text.split("```text\n", 1)[1].split("\n```", 1)[0]
    source = "\n".join(f"{i:4d}  {ln}" for i, ln in enumerate(SOURCE.read_text().splitlines(), 1))
    filled = (
        template.replace("{part_id}", "jsmn")
        .replace("{tier}", "judgment")
        .replace("{target_path}", target)
        .replace("{c2rust}", "none")
        .replace("{rulebook}", rulebook_sections())
        .replace("{idiom_rows}", idiom_rows())
        .replace("{structs}", "none (single Part)")
        .replace("{callee_signatures}", "none (jsmn calls no other Part)")
        .replace("{source}", source)
        .replace("{previous}", "none (attempt 1)")
        .replace("{tm_hits}", "none")
    )
    Path(out).write_text(filled)
    print(f"{out}: {len(filled)} chars, {filled.count(chr(10))} lines")
    return 0


if __name__ == "__main__":
    sys.exit(main())
