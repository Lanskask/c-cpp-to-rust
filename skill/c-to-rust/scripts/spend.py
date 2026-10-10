#!/usr/bin/env python3
"""Sum Claude Code token usage from session transcripts and log it to SPEND.md.

Claude Code writes every API response into `~/.claude/projects/<project>/*.jsonl`
(subagent transcripts live in subdirectories). Each assistant line carries
`message.usage`; one message is split over several lines that repeat the same
usage, so totals are de-duplicated by `message.id` (decision 22).

Subcommands:
  now                       print the current UTC timestamp (mark a phase start)
  sum --since T [--until T] print totals as JSON
  log --phase P --since T   append one row to SPEND.md and print the totals

Exit code 0 on success, 1 on bad input.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass, field
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

SPEND_HEADER = (
    "| date | phase | part | input | output | cache_create | cache_read | messages | models | note |\n"
    "|---|---|---|---|---|---|---|---|---|---|\n"
)


@dataclass
class Totals:
    input: int = 0
    output: int = 0
    cache_create: int = 0
    cache_read: int = 0
    messages: int = 0
    by_model: dict[str, int] = field(default_factory=dict)  # model -> output tokens

    def add(self, model: str, usage: dict[str, Any]) -> None:
        out = int(usage.get("output_tokens") or 0)
        self.input += int(usage.get("input_tokens") or 0)
        self.output += out
        self.cache_create += int(usage.get("cache_creation_input_tokens") or 0)
        self.cache_read += int(usage.get("cache_read_input_tokens") or 0)
        self.messages += 1
        self.by_model[model] = self.by_model.get(model, 0) + out


def default_transcripts_dir(cwd: Path) -> Path:
    """Claude Code names the project dir after the cwd with non-alphanumerics as '-'."""
    return Path.home() / ".claude" / "projects" / re.sub(r"[^A-Za-z0-9]", "-", str(cwd))


def parse_time(text: str) -> datetime:
    t = datetime.fromisoformat(text)
    return t if t.tzinfo else t.replace(tzinfo=UTC)


def sum_usage(root: Path, since: datetime, until: datetime) -> Totals:
    totals = Totals()
    seen: set[str] = set()
    for path in sorted(root.rglob("*.jsonl")):
        with path.open(encoding="utf-8", errors="replace") as fh:
            for line in fh:
                try:
                    rec = json.loads(line)
                except json.JSONDecodeError:
                    continue
                msg = rec.get("message")
                ts = rec.get("timestamp")
                if not isinstance(msg, dict) or not isinstance(ts, str):
                    continue
                usage = msg.get("usage")
                if not isinstance(usage, dict):
                    continue
                if not since <= parse_time(ts) < until:
                    continue
                key = str(msg.get("id") or rec.get("uuid"))
                if key in seen:
                    continue
                seen.add(key)
                totals.add(str(msg.get("model") or "unknown"), usage)
    return totals


def append_row(out: Path, phase: str, part: str, totals: Totals, note: str) -> None:
    models = ", ".join(f"{m}:{n}" for m, n in sorted(totals.by_model.items()))
    row = (
        f"| {datetime.now(UTC):%Y-%m-%d %H:%M} | {phase} | {part} | {totals.input} | {totals.output} "
        f"| {totals.cache_create} | {totals.cache_read} | {totals.messages} | {models} | {note} |\n"
    )
    new = not out.exists() or out.stat().st_size == 0
    with out.open("a", encoding="utf-8") as fh:
        if new:
            fh.write("# Token spend\n\nWritten by `spend.py` from Claude Code transcripts (decision 16).\n\n")
            fh.write(SPEND_HEADER)
        fh.write(row)


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    sub.add_parser("now", help="print the current UTC timestamp")
    for name in ("sum", "log"):
        s = sub.add_parser(name)
        s.add_argument("--since", required=True, help="ISO timestamp, inclusive")
        s.add_argument("--until", help="ISO timestamp, exclusive (default: now)")
        s.add_argument("--transcripts", type=Path, help="transcript dir (default: derived from cwd)")
        if name == "log":
            s.add_argument("--phase", required=True)
            s.add_argument("--part", default="-")
            s.add_argument("--note", default="")
            s.add_argument("--out", type=Path, default=Path("SPEND.md"))
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.cmd == "now":
        print(datetime.now(UTC).isoformat(timespec="seconds"))
        return 0
    root: Path = args.transcripts or default_transcripts_dir(Path.cwd())
    if not root.is_dir():
        print(f"spend: transcript dir not found: {root}", file=sys.stderr)
        return 1
    try:
        since = parse_time(args.since)
        until = parse_time(args.until) if args.until else datetime.now(UTC)
    except ValueError as e:
        print(f"spend: bad timestamp: {e}", file=sys.stderr)
        return 1
    totals = sum_usage(root, since, until)
    if args.cmd == "log":
        append_row(args.out, args.phase, args.part, totals, args.note)
    print(json.dumps(asdict(totals)))
    print(
        f"spend: {totals.messages} messages, {totals.output} output, {totals.cache_read} cache-read tokens",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
