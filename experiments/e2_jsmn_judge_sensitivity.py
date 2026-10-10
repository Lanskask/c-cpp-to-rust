#!/usr/bin/env python3
"""E2: can the jsmn Judge detect a one-character behaviour change in the Rust port?

Question: the upstream tests plus the Differential harness pass on the merged crate. Is that evidence
or is the Judge blind? Mutate `pilots/jsmn/rust/src/lib.rs` (primitive-byte range `32..127` -> `32..126`,
i.e. reject `~`), run the Judge for the default feature set, restore the file, and report.

Usage: python3 -I experiments/e2_jsmn_judge_sensitivity.py [--stage 2]
       (repo root; restores the file even on error). `--stage 2` runs the Stage 2 Judge (translated tests +
       Rust example driver, judge.md §5) for all four feature sets and reports each one.

Answer, Stage 1 Judge (run 2026-10-10): upstream tests alone (16/16, `FAILED: 0`) do NOT notice the mutation. The first
harness corpus (164 hand-written + mutated inputs) did not either: no input contained byte 0x7e. After adding
every byte value 0..255 as primitive, string char and escape char (932 inputs), the harness reports 1 DIFF
(`[~a]`: orig count r=2, Rust r=-2). Lesson: a differential corpus needs exhaustive single-byte cases for
byte-class checks; passing on a hand-picked corpus is weak evidence.
Answer, Stage 2 Judge on the Faithful port through the Stage 1 adapter: see RUN-NOTES.md "Stage 2 Judge validation".
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LIB = ROOT / "pilots" / "jsmn" / "rust" / "src" / "lib.rs"
RUN = ROOT / "pilots" / "jsmn" / "migration" / "harness" / "run.py"


FEATURE_SETS = ["", "strict", "parent-links", "strict,parent-links"]


def stage2() -> int:
    original = LIB.read_text()
    if "(32..127)" not in original:
        print("e2: mutation target not found", file=sys.stderr)
        return 1
    missed = 0
    try:
        LIB.write_text(original.replace("(32..127)", "(32..126)"))
        for feats in FEATURE_SETS:
            r = subprocess.run([sys.executable, "-I", str(RUN), "--stage", "2", feats], capture_output=True, text=True)
            detected = r.returncode != 0
            missed += not detected
            last = (r.stderr.strip().splitlines() or r.stdout.strip().splitlines() or [""])[-1]
            print(f"e2 stage 2 [{feats or 'default'}]: {'DETECTED' if detected else 'NOT detected'}: {last[:150]}")
    finally:
        LIB.write_text(original)
    return 0 if missed == 0 else 1


def main() -> int:
    if sys.argv[1:] == ["--stage", "2"]:
        return stage2()
    original = LIB.read_text()
    if "(32..127)" not in original:
        print("e2: mutation target not found", file=sys.stderr)
        return 1
    try:
        LIB.write_text(original.replace("(32..127)", "(32..126)"))
        r = subprocess.run([sys.executable, "-I", str(RUN), ""], capture_output=True, text=True)
    finally:
        LIB.write_text(original)
    print(r.stdout.strip())
    print(r.stderr.strip())
    detected = r.returncode != 0
    print(f"e2: mutation {'DETECTED' if detected else 'NOT detected'} by the Judge", file=sys.stderr)
    return 0 if detected else 1


if __name__ == "__main__":
    sys.exit(main())
