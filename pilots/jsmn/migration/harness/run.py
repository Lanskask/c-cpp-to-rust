#!/usr/bin/env python3
"""Judge for the jsmn Pilot, one feature set: original tests -> Rust crate, then the Differential harness.

Usage: python3 -I run.py [FEATURES]      FEATURES = comma list of `strict`, `parent-links` (default: none)
Steps (all outputs under migration/harness/build/<features>/):
  1. cargo build the crate (staticlib) with FEATURES;
  2. compile upstream test/tests.c with -DJSMN_HEADER (declarations only) and link the Rust staticlib
     (decision 20: test-only linking); it must report FAILED: 0;
  3. compile dump.c twice (original jsmn.h vs Rust staticlib) and run harness.py on inputs/.
Exit 0 only if all three pass.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
PILOT = HERE.parent.parent
CRATE = PILOT / "rust"
SRC = PILOT / "src"
HARNESS = HERE.parent.parent.parent.parent / "skill" / "c-to-rust" / "scripts" / "harness.py"


def sh(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, capture_output=True, text=True)


def main() -> int:
    feats = sys.argv[1] if len(sys.argv) > 1 else ""
    names = [f for f in feats.split(",") if f]
    defines = [{"strict": "-DJSMN_STRICT=1", "parent-links": "-DJSMN_PARENT_LINKS=1"}[f] for f in names]
    build = HERE / "build" / (feats.replace(",", "+") or "default")
    build.mkdir(parents=True, exist_ok=True)
    inputs = HERE / "inputs"
    r = sh([sys.executable, "-I", str(HERE / "make_inputs.py"), str(inputs)])  # deterministic, so always regenerate
    if r.returncode:
        print(r.stderr, file=sys.stderr)
        return 1

    # 1. Rust staticlib. A per-feature target dir keeps feature sets from clobbering each other.
    tdir = build / "cargo-target"
    cmd = ["cargo", "build", "-q", "--manifest-path", str(CRATE / "Cargo.toml"), "--target-dir", str(tdir)]
    if feats:
        cmd += ["--features", feats]
    r = sh(cmd)
    if r.returncode:
        print(f"run: cargo build failed\n{r.stderr}", file=sys.stderr)
        return 1
    lib = tdir / "debug" / "libjsmn.a"

    # 2. original upstream tests against the Rust crate
    tests = build / "tests_vs_rust"
    r = sh(["cc", "-DJSMN_HEADER", *defines, str(SRC / "test" / "tests.c"), str(lib), "-o", str(tests)])
    if r.returncode:
        print(f"run: compiling upstream tests failed\n{r.stderr}", file=sys.stderr)
        return 1
    r = sh([str(tests)])
    print(f"original tests vs Rust [{feats or 'default'}]: {r.stdout.strip().splitlines()[-1] if r.stdout else r.stderr}")
    if r.returncode != 0 or "FAILED: 0" not in r.stdout:
        return 1

    # 3. differential harness: same dump.c, original header vs Rust staticlib
    orig, rust = build / "dump_orig", build / "dump_rust"
    inc = ["-I", str(SRC), "-I", str(HERE)]
    for cmd in (
        ["cc", *inc, *defines, str(HERE / "dump.c"), "-o", str(orig)],
        ["cc", *inc, "-DJSMN_HEADER", *defines, str(HERE / "dump.c"), str(lib), "-o", str(rust)],
    ):
        r = sh(cmd)
        if r.returncode:
            print(f"run: driver build failed\n{r.stderr}", file=sys.stderr)
            return 1
    r = sh([sys.executable, "-I", str(HARNESS), "--orig", str(orig), "--rust", str(rust),
            "--inputs", str(inputs), "--out", str(build / "harness-report.tsv")])
    print(r.stderr.strip())
    return r.returncode


if __name__ == "__main__":
    sys.exit(main())
