#!/usr/bin/env python3
"""Stage 1 Judge for the cJSON Pilot, one feature set (judge.md §4): original tests -> Rust crate, then the
Differential harness. Exit 0 = pass.

Usage: python3 -I run.py [--crate DIR | --scratch] [--stage 1|2] [FEATURES]
       FEATURES = comma list; the only macro feature is `locales` (ENABLE_LOCALES). Default: none.
       --crate   crate to judge (default: pilots/cjson/rust, the accepted Stage 1 target)
       --scratch use the c2rust SCRATCH crate for the feature set (work/scratch/<default|locales>/, made by
                 make_scratch.py on first use). Ticket 02 validates the Judge on these; they are not the target.
       --stage   1 (default): the steps below. 2: the Stage 2 Judge of judge.md section 5: `cargo test` of the
                 translated tests (rust/tests/upstream.rs through rust/tests/adapter/mod.rs), the release stack
                 check of RULEBOOK D10 (`cargo test --release`, one test on a 2 MiB thread), then the original
                 C driver vs `cargo build --example dump` (rust/examples/dump.rs) on the same corpus. No C ABI
                 and no `test-internals` needed. Outputs under work/run/<crate-tag>/<set>-stage2/.
Steps (outputs under work/<crate-tag>/<features>/, git-ignored):
  0. regenerate inputs/ (make_inputs.py, fixed seed);
  1. cargo build the crate twice: with `test-internals` (upstream tests only) and without (the driver);
  2. baseline: the 21 upstream test programs built from the original C (real tests/common.h), run;
  3. the same 21 programs built with the shim common.h (public headers only) and linked to the Rust
     staticlib; each must report `0 Failures`. A test that cannot link fails the run unless it is listed
     in C_ORIGINAL_ONLY with the exact reason (then its inputs must be covered by the corpus);
  4. dump.c built twice (original C vs Rust staticlib), harness.py --repeat 2 on inputs/.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
PILOT = HERE.parent.parent
REPO = PILOT.parent.parent
SRC = PILOT / "src"
TESTS = SRC / "tests"
DEFAULT_CRATE = PILOT / "rust"
HARNESS = REPO / "skill" / "c-to-rust" / "scripts" / "harness.py"

CJSON_TESTS = [
    "parse_examples", "parse_number", "parse_hex4", "parse_string", "parse_array", "parse_object", "parse_value",
    "print_string", "print_number", "print_array", "print_object", "print_value", "misc_tests", "parse_with_opts",
    "compare_tests", "cjson_add", "readme_examples", "minify_tests",
]
UTILS_TESTS = ["json_patch_tests", "old_utils_tests", "misc_utils_tests"]
DEFINES = {"locales": "-DENABLE_LOCALES"}
# test -> exact reason it cannot link to the Rust crate (filled only from a measured link failure)
C_ORIGINAL_ONLY: dict[str, str] = {}
CFLAGS = ["-O0", "-w"]
UNITY_RE = re.compile(r"(\d+) Tests (\d+) Failures (\d+) Ignored")


def sh(cmd: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, capture_output=True, text=True, cwd=cwd)


def cargo_build(crate: Path, feats: list[str], tdir: Path) -> Path | None:
    cmd = ["cargo", "build", "-q", "--manifest-path", str(crate / "Cargo.toml"), "--target-dir", str(tdir)]
    if feats:
        cmd += ["--features", ",".join(feats)]
    r = sh(cmd)
    if r.returncode:
        print(f"run: cargo build failed [{','.join(feats) or 'default'}]\n{r.stderr[-3000:]}", file=sys.stderr)
        return None
    name = tomllib.loads((crate / "Cargo.toml").read_text())["package"]["name"].replace("-", "_")
    return tdir / "debug" / f"lib{name}.a"


def run_unity(exe: Path, cwd: Path) -> tuple[int, int, int, str]:
    """Run one Unity test program; returns (tests, failures, ignored, note)."""
    r = sh([str(exe)], cwd=cwd)
    m = UNITY_RE.search(r.stdout)
    if not m:
        return -1, -1, -1, f"no Unity result line (exit {r.returncode}): {(r.stdout + r.stderr)[-200:]!r}"
    t, f, i = (int(g) for g in m.groups())
    note = "" if r.returncode == 0 and f == 0 else f"exit {r.returncode}"
    if f:
        note += " " + "; ".join(ln for ln in r.stdout.splitlines() if ":FAIL" in ln)[:300]
    return t, f, i, note.strip()


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("features", nargs="?", default="")
    ap.add_argument("--crate", type=Path, default=DEFAULT_CRATE)
    ap.add_argument("--scratch", action="store_true")
    ap.add_argument("--stage", type=int, choices=[1, 2], default=1)
    ap.add_argument("--skip-tests", action="store_true",
                    help="diagnostic: skip the upstream tests, run only the Differential harness (judge.md §6)")
    args = ap.parse_args(argv)
    feats = [f for f in args.features.split(",") if f]
    unknown = [f for f in feats if f not in DEFINES]
    if unknown:
        print(f"run: unknown feature(s) {unknown}; known: {sorted(DEFINES)}", file=sys.stderr)
        return 2
    defines = [DEFINES[f] for f in feats]
    setname = "+".join(feats) or "default"

    if args.scratch:
        crate = HERE / "work" / "scratch" / ("locales" if "locales" in feats else "default")
        if not crate.is_dir():
            r = sh([sys.executable, "-I", str(HERE / "make_scratch.py")])
            if r.returncode:
                print(r.stdout + r.stderr, file=sys.stderr)
                return 1
        tag = "scratch"
    else:
        crate = args.crate.resolve()
        tag = "default" if crate == DEFAULT_CRATE.resolve() else f"{crate.name}-{hashlib.sha1(str(crate).encode()).hexdigest()[:8]}"
    if args.stage == 2:
        if args.scratch:
            print("run: --stage 2 judges the real crate; --scratch is Stage 1 only", file=sys.stderr)
            return 2
        build = HERE / "work" / "run" / tag / f"{setname}-stage2"
        build.mkdir(parents=True, exist_ok=True)  # keeps cargo-target between runs (incremental builds)
    else:
        build = HERE / "work" / "run" / tag / setname
        shutil.rmtree(build, ignore_errors=True)
        build.mkdir(parents=True)
    print(f"judge: crate={crate} features=[{','.join(feats) or 'default'}]")

    # 0. corpus
    inputs = HERE / "inputs"
    r = sh([sys.executable, "-I", str(HERE / "make_inputs.py"), str(inputs), str(TESTS)])
    if r.returncode:
        print(r.stderr, file=sys.stderr)
        return 1
    print(f"judge: {r.stdout.strip()}")

    if args.stage == 2:
        return stage2(crate, feats, defines, build, inputs, args.skip_tests)

    # 1. staticlibs
    lib_tests = None if args.skip_tests else cargo_build(crate, [*feats, "test-internals"], build / "cargo-target-tests")
    lib_plain = cargo_build(crate, feats, build / "cargo-target")
    if (lib_tests is None and not args.skip_tests) or lib_plain is None:
        return 1

    # unity (shared)
    unity_o = build / "unity.o"
    r = sh(["cc", *CFLAGS, "-c", str(TESTS / "unity" / "src" / "unity.c"), "-o", str(unity_o)])
    if r.returncode:
        print(r.stderr, file=sys.stderr)
        return 1

    # data files the tests read relative to their cwd
    def stage_data(cwd: Path) -> None:
        cwd.mkdir(parents=True, exist_ok=True)
        for d in ("inputs", "json-patch-tests"):
            shutil.copytree(TESTS / d, cwd / d, dirs_exist_ok=True)

    failures = 0
    rows: list[tuple[str, str, str]] = []

    if args.skip_tests:
        return differential(inputs, build, defines, lib_plain)

    # 2. baseline: upstream tests on the original C (real common.h includes ../cJSON.c)
    orig_dir = build / "orig_tests"
    stage_data(orig_dir)
    for name in CJSON_TESTS + UTILS_TESTS:
        exe = orig_dir / name
        cmd = ["cc", *CFLAGS, *defines, f"-I{TESTS}", f"-I{SRC}", str(TESTS / f"{name}.c"), str(unity_o)]
        if name in UTILS_TESTS:
            cmd.append(str(SRC / "cJSON_Utils.c"))
        r = sh([*cmd, "-lm", "-o", str(exe)])
        if r.returncode:
            print(f"run: original test {name} failed to build\n{r.stderr[-1500:]}", file=sys.stderr)
            return 1
        t, f, i, note = run_unity(exe, orig_dir)
        rows.append((name, f"{t} tests {f} failures {i} ignored", ""))
        failures += f != 0 or t <= 0
        if f != 0 or t <= 0:
            print(f"run: ORIGINAL C test {name} not clean: {note}", file=sys.stderr)
    orig_rows = dict((n, o) for n, o, _ in rows)
    rows.clear()

    # 3. same tests, shim common.h, linked to the Rust staticlib
    stage = build / "stage"
    (stage / "tests").mkdir(parents=True)
    for h in ("cJSON.h", "cJSON_Utils.h"):
        shutil.copy(SRC / h, stage / h)
    shutil.copy(HERE / "shim" / "common.h", stage / "tests" / "common.h")
    run_dir = build / "rust_tests"
    stage_data(run_dir)
    total_tests = 0
    for name in CJSON_TESTS + UTILS_TESTS:
        shutil.copy(TESTS / f"{name}.c", stage / "tests" / f"{name}.c")
        exe = run_dir / name
        r = sh([
            "cc", *CFLAGS, *defines, f"-I{TESTS}", str(stage / "tests" / f"{name}.c"), str(unity_o),
            str(lib_tests), "-lm", "-o", str(exe),
        ])
        if r.returncode:
            undefined = sorted(set(re.findall(r'Undefined symbols.*?"(_?\w+)"', r.stderr)) | set(re.findall(r'"(_\w+)", referenced', r.stderr)))
            reason = C_ORIGINAL_ONLY.get(name)
            if reason:
                rows.append((name, "C-original-only", reason))
                continue
            print(f"run: upstream test {name} cannot be built/linked against the Rust crate: undefined={undefined}\n"
                  f"{r.stderr[-1500:]}", file=sys.stderr)
            rows.append((name, "LINK FAILURE", ",".join(undefined)))
            failures += 1
            continue
        t, f, i, note = run_unity(exe, run_dir)
        total_tests += max(t, 0)
        rows.append((name, f"{t} tests {f} failures {i} ignored", note))
        if f != 0 or t <= 0 or note:
            failures += 1
    print(f"upstream tests [{','.join(feats) or 'default'}] (C original | Rust staticlib with test-internals):")
    for name, res, note in rows:
        print(f"  {name:18s} {orig_rows.get(name, '?'):32s} | {res}{'  ' + note if note else ''}")
    print(f"upstream tests total on Rust: {total_tests} tests, {failures} failing file(s)")
    if failures:
        return 1

    return differential(inputs, build, defines, lib_plain)


ADAPTER = "tests/adapter/mod.rs"
RELEASE_STACK_TEST = "extra_release_build_reaches_the_limits_on_a_2_mib_stack"


def stage2_test_numbers(out: str) -> tuple[int, int, int]:
    """(passed, failed, ignored) summed over every `test result:` line of a cargo test run."""
    p = f = i = 0
    for m in re.finditer(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", out):
        p, f, i = p + int(m.group(1)), f + int(m.group(2)), i + int(m.group(3))
    return p, f, i


def stage2(crate: Path, feats: list[str], defines: list[str], build: Path, inputs: Path, skip_tests: bool) -> int:
    """Stage 2 Judge (judge.md section 5.4): translated tests, then the C driver vs the Rust example driver."""
    tdir = build / "cargo-target"
    base = ["--manifest-path", str(crate / "Cargo.toml"), "--target-dir", str(tdir)]
    if feats:
        base += ["--features", ",".join(feats)]
    adapter_src = (crate / ADAPTER).read_text()
    layout = re.search(r"pub const C_LAYOUT: bool = (true|false);", adapter_src)
    # RULEBOOK D1 (revised 2026-10-10): every adapter has the allocator hooks; the C driver is always built with
    # its allocator section, so the allocation counts and the fail-the-Nth sweep are compared in Stage 2 too.
    print(f"judge: stage 2, adapter C_LAYOUT={layout.group(1) if layout else '?'}")

    if not skip_tests:
        r = sh(["cargo", "test", *base, "--", "--nocapture"])
        out = r.stdout + r.stderr
        p, f, i = stage2_test_numbers(out)
        skipped = sorted({ln.strip() for ln in out.splitlines() if ln.startswith("SKIPPED: ")})
        print(f"translated tests [{','.join(feats) or 'default'}]: {p} passed, {f} failed, {i} ignored, "
              f"{len(skipped)} skipped step kind(s)")
        for ln in skipped:
            print(f"  {ln}")
        if r.returncode or f or p == 0:
            fails = [ln for ln in out.splitlines() if ln.startswith("test ") and ln.endswith("FAILED")]
            print("\n".join(fails) or out[-3000:], file=sys.stderr)
            return 1
        # RULEBOOK D10: the release build reaches the nesting and circular limits on a 2 MiB thread
        r = sh(["cargo", "test", "--release", *base, "--test", "upstream", RELEASE_STACK_TEST])
        out = r.stdout + r.stderr
        p, f, _ = stage2_test_numbers(out)
        print(f"release stack check [{','.join(feats) or 'default'}]: {p} passed, {f} failed")
        if r.returncode or f or p != 1:
            print(out[-3000:], file=sys.stderr)
            return 1

    r = sh(["cargo", "build", "-q", "--example", "dump", *base])
    if r.returncode:
        print(f"run: cargo build --example dump failed\n{r.stderr[-3000:]}", file=sys.stderr)
        return 1
    rust = tdir / "debug" / "examples" / "dump"
    orig = build / "dump_orig"
    cmd = ["cc", *CFLAGS, f"-I{SRC}", *defines, str(HERE / "dump.c"), str(SRC / "cJSON.c"),
           str(SRC / "cJSON_Utils.c"), "-lm", "-o", str(orig)]
    r = sh(cmd)
    if r.returncode:
        print(f"run: driver build failed\n{r.stderr[-2000:]}", file=sys.stderr)
        return 1
    return run_harness(inputs, build, orig, rust)


def differential(inputs: Path, build: Path, defines: list[str], lib_plain: Path) -> int:
    """Step 4: the same dump.c, original C vs Rust staticlib (public headers only), harness.py --repeat 2."""
    orig, rust = build / "dump_orig", build / "dump_rust"
    inc = [f"-I{SRC}"]
    for cmd in (
        ["cc", *CFLAGS, *inc, *defines, str(HERE / "dump.c"), str(SRC / "cJSON.c"), str(SRC / "cJSON_Utils.c"), "-lm", "-o", str(orig)],
        ["cc", *CFLAGS, *inc, str(HERE / "dump.c"), str(lib_plain), "-lm", "-o", str(rust)],
    ):
        r = sh(cmd)
        if r.returncode:
            print(f"run: driver build failed\n{r.stderr[-2000:]}", file=sys.stderr)
            return 1
    return run_harness(inputs, build, orig, rust)


def run_harness(inputs: Path, build: Path, orig: Path, rust: Path) -> int:
    r = sh([
        sys.executable, "-I", str(HARNESS), "--orig", str(orig), "--rust", str(rust), "--inputs", str(inputs),
        "--out", str(build / "harness-report.tsv"), "--repeat", "2",
    ])
    print(r.stderr.strip()[-1500:])
    report = build / "harness-report.tsv"
    if r.returncode and report.exists():
        # harness.py prints only the first lines of a difference; list every DIFF row with both exit codes
        rows = [ln.split("\t") for ln in report.read_text().splitlines()[1:]]
        diffs = [x for x in rows if len(x) >= 4 and x[3] == "DIFF"]
        print(f"judge: {len(diffs)} DIFF input(s) (name, orig exit, rust exit, detail):")
        for x in diffs[:25]:
            print(f"  {x[0]}\t{x[1]}\t{x[2]}\t{x[4] if len(x) > 4 else ''}"[:260])
    return r.returncode


if __name__ == "__main__":
    sys.exit(main())
