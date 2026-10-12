#!/usr/bin/env python3
"""Stage 1 Judge for the tinyjson Pilot (C++ route, judge.md sections 4, 4a, 5, 8). Exit 0 = pass.

Usage: python3 -I run.py [--stage 1|2] [--crate DIR] [--skip-tests] [--limit N] [--jobs N]
       --crate DIR     crate to judge: a directory with Cargo.toml whose library is named `tinyjson` and offers the
                       API the adapter calls (default: pilots/tinyjson/experiments/scratch-crate, the STUB of
                       ticket 02; ticket 04's crate is pilots/tinyjson/rust)
       --stage N       1 (default): the Stage 1 adapter (C++ names, RULEBOOK section 5); 2: adapter_stage2.rs
                       (RULEBOOK section 6 API), installed together with stage2_shim.rs, a shim of the section 6 API
                       over the Stage 1 crate (validation scaffolding, deleted when the Stage 2 crate exists)
       --skip-tests    skip `cargo test` (translated tests), run the Differential harness only
       --limit N       judge about N inputs, taken at an even stride over the sorted corpus (default: all)
       --jobs N        parallel inputs for harness.py (default: CPU count)

Steps (outputs under migration/harness/work/, git-ignored; nothing runs with a pilot directory as cwd):
  0. print the Reference compiler version (RULEBOOK section 10 / RUN-NOTES "Reference build");
  1. regenerate nothing: inputs/ comes from make_inputs.py (run here only when inputs/ is missing);
  2. build the C++ driver and the patched upstream tests with ref/build_ref.sh (GCC 16, -O0);
  3. install the Rust side into the crate (copies, see below) and `cargo build --example dump`;
  4. port check: the string literals of dump.rs, in order, equal those of dump.cpp;
  5. `cargo test --test upstream` (22 translated tests; skipped by --skip-tests);
  6. skill/c-to-rust/scripts/harness.py: C++ driver (--orig) against the Rust driver (--rust) on the corpus.
A crash or panic of the Rust driver is a difference (exit code and output compared), never a runner error. A
build error of the crate or of the drivers is a runner error (exit 2): the Judge itself is broken.

Where the Rust side lives: harness/rust/{adapter.rs,dump.rs,upstream.rs} are the single copy. Step 3 installs them
into the crate as
    <crate>/tests/adapter/mod.rs   (adapter.rs: the only file that calls the crate)
    <crate>/examples/dump.rs       (dump.rs, `#[path = "../tests/adapter/mod.rs"] mod adapter;`)
    <crate>/tests/upstream.rs      (upstream.rs, `mod adapter;`)
so the same files judge the stub now and the real crate later. For the scratch crate these are git-ignored; for the
real crate ticket 04 commits them (the Gate then protects them). Stage 2 changes adapter.rs only.
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
PILOT = HERE.parent.parent
REPO = PILOT.parent.parent
WORK = HERE / "work"
INPUTS = HERE / "inputs"
RUST_SRC = HERE / "rust"
HARNESS_PY = REPO / "skill" / "c-to-rust" / "scripts" / "harness.py"
DEFAULT_CRATE = PILOT / "experiments" / "scratch-crate"
CXX = os.environ.get("CXX", "/opt/homebrew/bin/g++-16")
N_TESTS = 22


def sh(cmd: list[str], cwd: Path = WORK, timeout: int | None = None) -> subprocess.CompletedProcess[str]:
    WORK.mkdir(parents=True, exist_ok=True)
    return subprocess.run(cmd, capture_output=True, text=True, cwd=cwd, timeout=timeout)


def say(msg: str) -> None:
    print(msg, flush=True)


def runner_error(msg: str) -> int:
    print(f"run: ERROR (the Judge is broken, not the port): {msg}", file=sys.stderr, flush=True)
    return 2


# ---------------------------------------------------------------------------------------------------------------
# step 3: install the Rust side into the crate
# ---------------------------------------------------------------------------------------------------------------

def install(crate: Path, stage: int = 1) -> None:
    adapter = RUST_SRC / ("adapter_stage2.rs" if stage == 2 else "adapter.rs")
    pairs = [
        (adapter, crate / "tests" / "adapter" / "mod.rs"),
        (RUST_SRC / "dump.rs", crate / "examples" / "dump.rs"),
        (RUST_SRC / "upstream.rs", crate / "tests" / "upstream.rs"),
    ]
    shim_dst = crate / "tests" / "adapter" / "stage2_shim.rs"
    if stage == 2:
        pairs.append((RUST_SRC / "stage2_shim.rs", shim_dst))
    elif shim_dst.exists():
        shim_dst.unlink()  # a Stage 1 adapter does not declare the shim
    for src, dst in pairs:
        dst.parent.mkdir(parents=True, exist_ok=True)
        data = src.read_bytes()
        if not dst.exists() or dst.read_bytes() != data:  # keep mtimes for incremental cargo builds
            dst.write_bytes(data)


# ---------------------------------------------------------------------------------------------------------------
# step 4: port check (dump.rs is a line-for-line port of dump.cpp)
# ---------------------------------------------------------------------------------------------------------------

def literals(text: str) -> list[str]:
    """String literals in order, comments and char literals skipped (works for C++ and Rust sources)."""
    out: list[str] = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i):
            while i < n and text[i] != "\n":
                i += 1
        elif text.startswith("/*", i):
            i = text.find("*/", i) + 2
        elif c == "'" and re.match(r"'(?:\\.[0-9a-fA-F]*|[^'\\])'", text[i:i + 12]):
            i += re.match(r"'(?:\\.[0-9a-fA-F]*|[^'\\])'", text[i:i + 12]).end()  # type: ignore[union-attr]
        elif c == '"':
            j = i + 1
            buf = []
            while text[j] != '"':
                if text[j] == "\\":
                    buf.append(text[j:j + 2])
                    j += 2
                else:
                    buf.append(text[j])
                    j += 1
            out.append("".join(buf))
            i = j + 1
        else:
            i += 1
    return out


def pieces_rust(lits: list[str]) -> list[str]:
    """Rust format strings: `{}`/`{:..}` split a literal into the pieces C++ concatenates."""
    out: list[str] = []
    for lit in lits:
        for p in re.split(r"\{[^}]*\}", lit):
            if p:
                out.append(p)
    return out


def port_literals() -> tuple[list[str], list[str]]:
    cpp = (HERE / "dump.cpp").read_text()
    rs = (RUST_SRC / "dump.rs").read_text()
    # C++: from the equality guard to the D3 pin (driver only), then main
    a = cpp.index("equality guard")
    b = cpp.index("D3 pinning")
    m = cpp.index("// main")
    cpp_region = cpp[a:b] + cpp[m:]
    cpp_region = "\n".join(l for l in cpp_region.splitlines() if "fprintf" not in l and "pin_d3_value" not in l)
    rs_region = rs[rs.index("equality guard"):]
    rs_region = "\n".join(l for l in rs_region.splitlines() if "eprintln!" not in l)
    rs_region = re.sub(r'\.expect\("[^"]*"\)', "", rs_region)
    rs_region = re.sub(r'#\[path = "[^"]*"\]', "", rs_region)
    # single characters and empty strings are built differently on the two sides (`r += " "` against `push(' ')`)
    cl = [l for l in literals(cpp_region) if l != "other" and len(l) > 1]
    rl = [l for l in pieces_rust(literals(rs_region)) if len(l) > 1]
    return cl, rl


def port_check() -> tuple[bool, str]:
    cl, rl = port_literals()
    if cl == rl:
        return True, f"{len(cl)} literals identical"
    n = min(len(cl), len(rl))
    for k in range(n):
        if cl[k] != rl[k]:
            return False, f"first mismatch at literal {k}: C++ {cl[k]!r} vs Rust {rl[k]!r} ({len(cl)} vs {len(rl)})"
    return False, f"lengths differ: C++ {len(cl)} vs Rust {len(rl)}"


# ---------------------------------------------------------------------------------------------------------------

def sample_inputs(limit: int) -> Path:
    files = sorted(p for p in INPUTS.iterdir() if p.is_file())
    if limit <= 0 or limit >= len(files):
        return INPUTS
    sub = WORK / "inputs-limit"
    if sub.exists():
        shutil.rmtree(sub)
    sub.mkdir(parents=True)
    stride = len(files) / limit
    chosen = {files[int(k * stride)] for k in range(limit)}
    for p in sorted(chosen):
        os.symlink(p, sub / p.name)
    return sub


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--stage", type=int, choices=(1, 2), default=1)
    ap.add_argument("--crate", type=Path, default=DEFAULT_CRATE)
    ap.add_argument("--skip-tests", action="store_true")
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--jobs", type=int, default=os.cpu_count() or 4)
    args = ap.parse_args()
    crate = args.crate.resolve()
    t_all = time.time()
    timings: dict[str, float] = {}

    # 0. reference compiler first
    v = sh([CXX, "--version"])
    say(f"reference compiler: {v.stdout.splitlines()[0] if v.returncode == 0 and v.stdout else CXX + ' NOT FOUND'}")
    say("reference flags : -std=c++17 -include locale -O0 -w (ref/build_ref.sh), libstdc++")
    say(f"crate           : {crate}")
    say(f"stage           : {args.stage}")
    if v.returncode:
        return runner_error(f"{CXX} not found")
    if not (crate / "Cargo.toml").is_file():
        return runner_error(f"{crate}/Cargo.toml not found")

    # 1. corpus
    if not INPUTS.is_dir() or not any(INPUTS.iterdir()):
        r = sh([sys.executable, "-I", str(HERE / "make_inputs.py"), str(INPUTS)])
        if r.returncode:
            return runner_error("make_inputs.py failed\n" + r.stderr[-2000:])
    n_inputs = sum(1 for p in INPUTS.iterdir() if p.is_file())

    # 2. C++ side
    t = time.time()
    r = sh(["bash", str(HERE / "ref" / "build_ref.sh")])
    timings["build C++"] = time.time() - t
    if r.returncode:
        return runner_error("C++ build failed\n" + (r.stdout + r.stderr)[-3000:])
    cpp_dump = WORK / "dump"

    # 3. Rust side
    install(crate, args.stage)
    tdir = WORK / f"cargo-{crate.parent.name}-{crate.name}"
    t = time.time()
    r = sh(["cargo", "build", "-q", "--example", "dump", "--manifest-path", str(crate / "Cargo.toml"),
            "--target-dir", str(tdir)])
    timings["build Rust"] = time.time() - t
    if r.returncode:
        return runner_error("cargo build --example dump failed (crate or driver does not compile)\n" + r.stderr[-4000:])
    rust_dump = tdir / "debug" / "examples" / "dump"

    # 4. port check
    ok_port, msg = port_check()
    say(f"port check      : {'ok' if ok_port else 'MISMATCH'}: {msg}")

    # 5. translated tests
    tests_failed = 0
    tests_total = 0
    failing: list[str] = []
    if args.skip_tests:
        say("translated tests: skipped (--skip-tests)")
    else:
        t = time.time()
        r = sh(["cargo", "test", "-q", "--test", "upstream", "--manifest-path", str(crate / "Cargo.toml"),
                "--target-dir", str(tdir)])
        timings["cargo test"] = time.time() - t
        out = r.stdout + r.stderr
        if "error[" in out or "could not compile" in out:
            return runner_error("cargo test: the translated tests do not compile\n" + out[-4000:])
        m = re.search(r"test result: \w+\. (\d+) passed; (\d+) failed", out)
        if not m:
            return runner_error("cargo test: no result line\n" + out[-2000:])
        passed, tests_failed = int(m.group(1)), int(m.group(2))
        tests_total = passed + tests_failed
        failing = re.findall(r"^test (\S+) \.\.\. FAILED", out, re.M) or re.findall(r"^\s+(\S+)$", out.split("failures:")[-1], re.M)
        say(f"translated tests: {passed} passed, {tests_failed} failed of {tests_total} (expected {N_TESTS})")
        if tests_total != N_TESTS:
            say(f"                  WARNING: {tests_total} tests found, upstream has {N_TESTS}")
        for name in failing[:30]:
            say(f"                  FAILED {name}")

    # 6. Differential harness
    inputs = sample_inputs(args.limit)
    n_run = sum(1 for p in inputs.iterdir() if p.is_file())
    tsv = WORK / "diff.tsv"
    t = time.time()
    r = sh([sys.executable, "-I", str(HARNESS_PY), "--orig", str(cpp_dump), "--rust", str(rust_dump),
            "--inputs", str(inputs), "--out", str(tsv), "--jobs", str(args.jobs)])
    timings["harness"] = time.time() - t
    if not tsv.is_file():
        return runner_error("harness.py wrote no report\n" + (r.stdout + r.stderr)[-2000:])
    rows = [l.rstrip("\n").split("\t") for l in tsv.read_text().splitlines()[1:]]
    diffs = [x for x in rows if len(x) > 3 and x[3] == "DIFF"]
    unstable = [x for x in rows if len(x) > 3 and x[3] == "UNSTABLE"]
    crashed = [x for x in diffs if len(x) > 2 and x[2] not in ("0", "2")]
    say(f"corpus          : {len(diffs)} DIFF of {len(rows)} inputs ({n_run} judged of {n_inputs} in the corpus); "
        f"{len(unstable)} unstable; Rust driver exit code not 0 on {len(crashed)} (panic or crash)")
    for x in diffs[:3]:
        say(f"                  e.g. {x[0]}  orig_exit={x[1]} rust_exit={x[2]}  {(x[4] if len(x) > 4 else '')[:160]}")
    say(f"report          : {tsv}")

    timings["total"] = time.time() - t_all
    say("timings (s)    : " + ", ".join(f"{k} {v:.1f}" for k, v in timings.items()))
    green = ok_port and tests_failed == 0 and not diffs and (args.skip_tests or tests_total == N_TESTS)
    say("JUDGE: " + ("GREEN" if green else "RED"))
    return 0 if green else 1


if __name__ == "__main__":
    sys.exit(main())
