#!/usr/bin/env python3
"""Reference build of a C or C++ original (judge.md §8): build and run the upstream test with candidate flags.

    python3 -I skill/c-to-rust/scripts/refbuild.py pilots/<name> --test xmltest.cpp --source tinyxml2.cpp \\
        [--resource resources] [--lang c++|c] [--std c++11 --std c++17] [--opt=-O0 --opt=-O2] [--cc c++ --cc clang++]

Paths after `--test`, `--source`, `--resource` are relative to `<ws>/src`. Nothing is built inside `src/` and the shell
never changes into `<ws>`: files are copied to `<ws>/migration/harness/work/refbuild/` (git-ignored) and compiled and
run there. Every candidate (compiler x standard x optimisation) is tried in order; each attempt is a row of the report
`<ws>/migration/harness/refbuild.md`, which also names the first passing candidate for the RULEBOOK `Reference build`
slot. Exit 0: a candidate built and the test exited 0. Exit 1: none did (stop and tell the user: judge.md §8).
Patches to the upstream test are not made here; if the test needs one, copy it into `work/refbuild` by hand and
record it in the report's `Patches` line.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path

TIMEOUT = 300
DEFAULTS = {
    "c++": (["c++", "clang++", "g++"], ["c++17", "c++14", "c++11"]),
    "c": (["cc", "clang", "gcc"], ["c11", "c99"]),
}


@dataclass
class Attempt:
    cc: str
    std: str
    opt: str
    built: bool
    passed: bool
    tail: str

    def flags(self) -> str:
        return f"-std={self.std} {self.opt}"

    def result(self) -> str:
        return "pass" if self.passed else ("test failed" if self.built else "build failed")


def tail(text: str, n: int = 3) -> str:
    return " | ".join(ln.strip() for ln in text.strip().splitlines()[-n:]).replace("\n", " ")[:300]


def version(cc: str) -> str:
    r = subprocess.run([cc, "--version"], capture_output=True, text=True)
    return (r.stdout or r.stderr).splitlines()[0] if r.returncode == 0 and (r.stdout or r.stderr) else "unknown"


def stage(ws: Path, work: Path, names: list[str]) -> list[str]:
    """Copy `src/<name>` files and directories to `work/<name>`; return the names."""
    for n in names:
        s = ws / "src" / n
        d = work / n
        d.parent.mkdir(parents=True, exist_ok=True)
        if s.is_dir():
            shutil.copytree(s, d, dirs_exist_ok=True)
        elif s.is_file():
            shutil.copy2(s, d)
        else:
            raise FileNotFoundError(f"{s} does not exist")
    return names


def attempt(cc: str, std: str, opt: str, work: Path, files: list[str], includes: list[str]) -> Attempt:
    exe = work / "refbuild.bin"
    exe.unlink(missing_ok=True)
    cmd = [cc, f"-std={std}", opt, *[f"-I{i}" for i in includes], *files, "-o", str(exe.name)]
    b = subprocess.run(cmd, cwd=work, capture_output=True, text=True, timeout=TIMEOUT)
    if b.returncode != 0:
        return Attempt(cc, std, opt, False, False, tail(b.stderr))
    r = subprocess.run([str(exe)], cwd=work, capture_output=True, text=True, timeout=TIMEOUT)
    return Attempt(cc, std, opt, True, r.returncode == 0, tail(r.stdout + r.stderr))


def report(ws: Path, attempts: list[Attempt], tests: list[str], sources: list[str]) -> str:
    ok = next((a for a in attempts if a.passed), None)
    lines = [
        f"# Reference build: {ws.name}",
        "",
        f"Date: {datetime.now(UTC).date()}",
        f"Test: {', '.join(tests)}; sources: {', '.join(sources) or 'none (header only)'}",
        "Patches: none (run by refbuild.py as copied from src/)",
        "",
    ]
    if ok:
        lines += [
            "## Chosen (RULEBOOK `Reference build` slot)",
            "",
            f"- compiler: {version(ok.cc)} (`{ok.cc}`)",
            f"- flags: `{ok.flags()}`",
            f"- optimisation level of the driver: `{ok.opt}`",
            f"- result: the upstream test exited 0; last output: {ok.tail}",
            "",
        ]
    else:
        lines += [
            "## No candidate passed",
            "",
            "Stop and tell the user: there is no reference behaviour (judge.md §8).",
            "",
        ]
    lines += ["## All attempts", "", "| compiler | flags | result | last lines |", "|---|---|---|---|"]
    lines += [f"| `{a.cc}` | `{a.flags()}` | {a.result()} | {a.tail.replace('|', '/')} |" for a in attempts]
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("ws", help="pilots/<name>")
    p.add_argument("--test", action="append", required=True, help="upstream test source, relative to src/")
    p.add_argument("--source", action="append", default=[], help="library source, relative to src/")
    p.add_argument("--resource", action="append", default=[], help="file or directory the test reads, relative to src/")
    p.add_argument("--include", action="append", default=[], help="-I directory, relative to src/")
    p.add_argument("--lang", choices=("c++", "c"), default="c++")
    p.add_argument("--cc", action="append", help="compiler candidate (default: c++, clang++, g++ / cc, clang, gcc)")
    p.add_argument("--std", action="append", help="standard candidate, e.g. c++17 (default per --lang)")
    p.add_argument("--opt", action="append", help="optimisation candidate (default -O0 and -O2)")
    a = p.parse_args(argv)
    ws = Path(a.ws).resolve()
    work = ws / "migration" / "harness" / "work" / "refbuild"
    try:
        if work.exists():
            shutil.rmtree(work)
        work.mkdir(parents=True)
        files = stage(ws, work, [*a.test, *a.source])
        stage(ws, work, a.resource)
        stage(ws, work, a.include)
    except (OSError, FileNotFoundError) as e:
        print(f"refbuild: {e}", file=sys.stderr)
        return 1
    ccs = [c for c in (a.cc or DEFAULTS[a.lang][0]) if shutil.which(c)]
    stds = a.std or DEFAULTS[a.lang][1]
    opts = a.opt or ["-O0", "-O2"]
    if not ccs:
        print("refbuild: no compiler candidate found on PATH", file=sys.stderr)
        return 1
    attempts: list[Attempt] = []
    for cc in ccs:
        for std in stds:
            for opt in opts:
                at = attempt(
                    cc, std, opt, work, [f for f in files if f.endswith((".c", ".cc", ".cpp", ".cxx"))], a.include
                )
                attempts.append(at)
                print(f"refbuild: {cc} -std={std} {opt}: {at.result()}", flush=True)
                if at.passed:
                    break
            if attempts[-1].passed:
                break
        if attempts[-1].passed:
            break
    out = ws / "migration" / "harness" / "refbuild.md"
    out.write_text(report(ws, attempts, a.test, a.source))
    print(f"refbuild: wrote {out}")
    return 0 if attempts[-1].passed else 1


if __name__ == "__main__":
    sys.exit(main())
