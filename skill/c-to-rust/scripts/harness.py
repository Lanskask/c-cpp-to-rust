#!/usr/bin/env python3
"""Differential harness: run original and Rust drivers on every input, diff stdout and exit code.

Stub (E0). Contract: PLAN.md §4. Implemented in E2.
Exit code 0 on success, 1 on failure.
"""

from __future__ import annotations

import argparse
import sys


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--orig", required=True, help="original-side driver binary")
    p.add_argument("--rust", required=True, help="Rust-side driver binary")
    p.add_argument("--inputs", default="migration/harness/inputs")
    p.add_argument("--out", default="migration/harness-report.tsv")
    return p


def main(argv: list[str] | None = None) -> int:
    build_parser().parse_args(argv)
    print("harness: not implemented yet (E2)", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
