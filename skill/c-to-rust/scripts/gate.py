#!/usr/bin/env python3
"""Baseline-aware gate for one Part: check, test, harness, unsafe count, clippy, rustfmt; path-scoped rollback.

Stub (E0). Contract: PLAN.md §4. Implemented in E2.
Exit code 0 on success, 1 on failure.
"""

from __future__ import annotations

import argparse
import sys


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("part")
    p.add_argument("--crate", required=True, help="path to the Rust crate (Cargo.toml dir)")
    p.add_argument("--stage", type=int, choices=[1, 2], required=True)
    p.add_argument("--parts", default="migration/parts.tsv")
    return p


def main(argv: list[str] | None = None) -> int:
    build_parser().parse_args(argv)
    print("gate: not implemented yet (E2)", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
