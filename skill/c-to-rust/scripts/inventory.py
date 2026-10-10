#!/usr/bin/env python3
"""List symbols of a Source project with risky-construct tags into migration/inventory.tsv.

Stub (E0). Contract: PLAN.md §4. Implemented in E2.
Exit code 0 on success, 1 on failure.
"""

from __future__ import annotations

import argparse
import sys


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("source_root")
    p.add_argument("--lang", choices=["c", "cpp"], required=True)
    p.add_argument("--compile-commands", help="path to compile_commands.json")
    p.add_argument("--out", default="migration/inventory.tsv")
    return p


def main(argv: list[str] | None = None) -> int:
    build_parser().parse_args(argv)
    print("inventory: not implemented yet (E2)", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
