#!/usr/bin/env python3
"""Resumable Part queue over parts.tsv: status, next, mark, verify. Truth comes from files and git.

Stub (E0). Contract: PLAN.md §4. Implemented in E2.
Exit code 0 on success, 1 on failure.
"""

from __future__ import annotations

import argparse
import sys


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--parts", default="migration/parts.tsv")
    sub = p.add_subparsers(dest="cmd", required=True)
    sub.add_parser("status")
    n = sub.add_parser("next")
    n.add_argument("--batch", type=int, default=1)
    m = sub.add_parser("mark")
    m.add_argument("part")
    m.add_argument("state", choices=["todo", "in_progress", "done", "failed", "stubbed"])
    m.add_argument("--commit")
    m.add_argument("--error")
    sub.add_parser("verify")
    return p


def main(argv: list[str] | None = None) -> int:
    build_parser().parse_args(argv)
    print("workqueue: not implemented yet (E2)", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
