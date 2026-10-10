#!/usr/bin/env python3
"""Judge for the jsmn Pilot, one feature set: original tests -> Rust crate, then the Differential harness.

Usage: python3 -I run.py [--crate DIR] [--stage 1|2] [FEATURES]
       FEATURES = comma list of `strict`, `parent-links` (default: none)
       --crate  crate to judge (default: pilots/jsmn/rust); stress-test scratch copies use this
       --stage  1 (default): C-ABI steps below. 2: `cargo test` (translated tests), then the C driver vs
                `cargo build --example dump` (judge.md §5), no C ABI needed.
Steps (all outputs under migration/harness/build/<features>/, or build/<crate-name>-<hash>/<features>/
for a non-default crate):
  1. cargo build the crate (staticlib) with FEATURES;
  2. compile upstream test/tests.c with -DJSMN_HEADER (declarations only) and link the Rust staticlib
     (decision 20: test-only linking); it must report FAILED: 0;
  3. compile dump.c twice (original jsmn.h vs Rust staticlib) and run harness.py on inputs/.
Exit 0 only if all three pass.
"""

from __future__ import annotations

import argparse
import hashlib
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
PILOT = HERE.parent.parent
DEFAULT_CRATE = PILOT / "rust"
SRC = PILOT / "src"
HARNESS = HERE.parent.parent.parent.parent / "skill" / "c-to-rust" / "scripts" / "harness.py"


def sh(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, capture_output=True, text=True)


def stage2(crate: Path, feats: str, defines: list[str], build: Path, inputs: Path) -> int:
    """Translated tests, then the C driver (original header) vs the Rust example driver, byte for byte."""
    tdir = build / "cargo-target"
    base = ["--manifest-path", str(crate / "Cargo.toml"), "--target-dir", str(tdir)] + (
        ["--features", feats] if feats else []
    )
    r = sh(["cargo", "test", "-q", *base])
    summary = [ln for ln in (r.stdout + r.stderr).splitlines() if ln.startswith("test result")]
    print(f"translated tests [{feats or 'default'}]: {' | '.join(summary) or 'no test result line'}")
    if r.returncode:
        print(r.stdout + r.stderr, file=sys.stderr)
        return 1
    r = sh(["cargo", "build", "-q", "--example", "dump", *base])
    if r.returncode:
        print(f"run: cargo build --example dump failed\n{r.stderr}", file=sys.stderr)
        return 1
    rust = tdir / "debug" / "examples" / "dump"
    orig = build / "dump_orig"
    r = sh(["cc", "-I", str(SRC), "-I", str(HERE), *defines, str(HERE / "dump.c"), "-o", str(orig)])
    if r.returncode:
        print(f"run: driver build failed\n{r.stderr}", file=sys.stderr)
        return 1
    r = sh(
        [
            sys.executable,
            "-I",
            str(HARNESS),
            "--orig",
            str(orig),
            "--rust",
            str(rust),
            "--inputs",
            str(inputs),
            "--out",
            str(build / "harness-report.tsv"),
        ]
    )
    print(r.stderr.strip())
    return r.returncode


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("features", nargs="?", default="")
    ap.add_argument("--crate", type=Path, default=DEFAULT_CRATE)
    ap.add_argument("--stage", type=int, choices=[1, 2], default=1)
    args = ap.parse_args(argv)
    feats: str = args.features
    crate = args.crate.resolve()
    names = [f for f in feats.split(",") if f]
    defines = [{"strict": "-DJSMN_STRICT=1", "parent-links": "-DJSMN_PARENT_LINKS=1"}[f] for f in names]
    build = HERE / "build"
    if crate != DEFAULT_CRATE.resolve():
        build = build / f"{crate.name}-{hashlib.sha1(str(crate).encode()).hexdigest()[:8]}"
    build = build / (feats.replace(",", "+") or "default")
    build.mkdir(parents=True, exist_ok=True)
    inputs = HERE / "inputs"
    r = sh([sys.executable, "-I", str(HERE / "make_inputs.py"), str(inputs)])  # deterministic, so always regenerate
    if r.returncode:
        print(r.stderr, file=sys.stderr)
        return 1

    if args.stage == 2:
        return stage2(crate, feats, defines, build, inputs)

    # 1. Rust staticlib. A per-feature target dir keeps feature sets from clobbering each other.
    tdir = build / "cargo-target"
    cmd = ["cargo", "build", "-q", "--manifest-path", str(crate / "Cargo.toml"), "--target-dir", str(tdir)]
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
    print(
        f"original tests vs Rust [{feats or 'default'}]: {r.stdout.strip().splitlines()[-1] if r.stdout else r.stderr}"
    )
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
    r = sh(
        [
            sys.executable,
            "-I",
            str(HARNESS),
            "--orig",
            str(orig),
            "--rust",
            str(rust),
            "--inputs",
            str(inputs),
            "--out",
            str(build / "harness-report.tsv"),
        ]
    )
    print(r.stderr.strip())
    return r.returncode


if __name__ == "__main__":
    sys.exit(main())
