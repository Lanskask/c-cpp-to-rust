#!/usr/bin/env python3
"""Differential harness: run original and Rust drivers on every input, diff stdout and exit code.

A driver is any command that takes one input file path as its last argument and prints a canonical dump.
`--orig` and `--rust` may contain arguments (they are split like a shell would). Inputs are the regular
files in `--inputs`, in sorted order. Report TSV columns: input, orig_exit, rust_exit, status, detail.
Status is `same`, `DIFF` or `UNSTABLE`. Output is compared as raw bytes; a timeout on either side is a DIFF.
`--repeat N` runs the original N times: an input whose output or exit code differs between runs is
`UNSTABLE` (judge.md §3 item 5), is not compared with the Rust side and does not fail the run.
Exit code 0 if no input is a DIFF, 1 otherwise. Drivers are written per Pilot (references/judge.md).
"""

from __future__ import annotations

import argparse
import shlex
import subprocess
import sys
from pathlib import Path

TIMEOUT_S = 20


TIMEOUT_CODE = -1


def run_driver(cmd: list[str], path: Path) -> tuple[int, bytes]:
    try:
        r = subprocess.run([*cmd, str(path)], capture_output=True, timeout=TIMEOUT_S)
    except subprocess.TimeoutExpired:
        return TIMEOUT_CODE, b"<timeout>"
    return r.returncode, r.stdout


def first_difference(a: bytes, b: bytes) -> str:
    la, lb = a.splitlines(), b.splitlines()
    for n, (x, y) in enumerate(zip(la, lb, strict=False), start=1):
        if x != y:
            return f"line {n}: orig={x!r} rust={y!r}"
    return f"length differs: orig {len(la)} lines, rust {len(lb)} lines"


def compare(
    orig: list[str], rust: list[str], inputs: list[Path], repeat: int = 1
) -> list[tuple[str, int, int, str, str]]:
    rows = []
    for path in inputs:
        runs = [run_driver(orig, path) for _ in range(max(repeat, 1))]
        if len(set(runs)) > 1:
            rows.append(
                (path.name, runs[0][0], 0, "UNSTABLE", f"{len(set(runs))} distinct outputs in {len(runs)} runs")
            )
            continue
        oc, oo = runs[0]
        rc, ro = run_driver(rust, path)
        same = oc == rc and oo == ro and TIMEOUT_CODE not in (oc, rc)
        detail = "" if same else (f"exit {oc} vs {rc}; " if oc != rc else "") + first_difference(oo, ro)
        rows.append((path.name, oc, rc, "same" if same else "DIFF", detail))
    return rows


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--orig", required=True, help="original-side driver command")
    p.add_argument("--rust", required=True, help="Rust-side driver command")
    p.add_argument("--inputs", default="migration/harness/inputs")
    p.add_argument("--out", default="migration/harness-report.tsv")
    p.add_argument("--repeat", type=int, default=1, help="run the original N times; unstable inputs are not compared")
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    inputs = sorted(p for p in Path(args.inputs).glob("*") if p.is_file()) if Path(args.inputs).is_dir() else []
    if not inputs:
        print(f"harness: no inputs in {args.inputs}", file=sys.stderr)
        return 1
    rows = compare(shlex.split(args.orig), shlex.split(args.rust), inputs, args.repeat)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    header = "input\torig_exit\trust_exit\tstatus\tdetail"
    out.write_text(header + "\n" + "\n".join("\t".join(map(str, r)) for r in rows) + "\n")
    diffs = [r for r in rows if r[3] == "DIFF"]
    unstable = [r for r in rows if r[3] == "UNSTABLE"]
    for r in diffs[:5]:
        print(f"{r[0]}: {r[4]}", file=sys.stderr)
    compared = len(rows) - len(unstable)
    extra = f", {len(unstable)} unstable (not compared)" if unstable else ""
    print(f"harness: {compared - len(diffs)}/{compared} inputs match{extra} -> {out}", file=sys.stderr)
    return 1 if diffs else 0


if __name__ == "__main__":
    sys.exit(main())
