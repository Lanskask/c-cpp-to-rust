#!/usr/bin/env python3
"""Translation-memory cache (SQLite FTS5): build from migration/ and git, search by error or snippet.

Stub (E0). Contract: PLAN.md §4. Implemented in E2.
Exit code 0 on success, 1 on failure.
"""

from __future__ import annotations

import argparse
import sys


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--db", default="migration/tm.sqlite")
    sub = p.add_subparsers(dest="cmd", required=True)
    sub.add_parser("build")
    s = sub.add_parser("search")
    s.add_argument("query")
    return p


def main(argv: list[str] | None = None) -> int:
    build_parser().parse_args(argv)
    print("tm: not implemented yet (E2)", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
