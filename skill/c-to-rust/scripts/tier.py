#!/usr/bin/env python3
"""Assign each Part a tier (mechanical, judgment, unknown) and write the parts.tsv skeleton.

Stub (E0). Contract: PLAN.md §4. Implemented in E2.
Exit code 0 on success, 1 on failure.
"""

from __future__ import annotations

import argparse
import sys


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("inventory")
    p.add_argument("order")
    p.add_argument("--out", default="migration/parts.tsv")
    return p


def main(argv: list[str] | None = None) -> int:
    build_parser().parse_args(argv)
    print("tier: not implemented yet (E2)", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
