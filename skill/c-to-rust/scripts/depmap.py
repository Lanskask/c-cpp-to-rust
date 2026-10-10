#!/usr/bin/env python3
"""Build include/call graphs, SCCs and a leaf-first Part order (edges.tsv, cycles.txt, order.txt).

Stub (E0). Contract: PLAN.md §4. Implemented in E2.
Exit code 0 on success, 1 on failure.
"""

from __future__ import annotations

import argparse
import sys


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("inventory")
    p.add_argument("--out-dir", default="migration")
    return p


def main(argv: list[str] | None = None) -> int:
    build_parser().parse_args(argv)
    print("depmap: not implemented yet (E2)", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
