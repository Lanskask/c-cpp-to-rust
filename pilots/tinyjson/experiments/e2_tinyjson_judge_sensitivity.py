#!/usr/bin/env python3
"""e2: can the tinyjson Judge fail? One-character mutants of the C++ header against the corpus and the upstream tests.

QUESTION (judge.md section 6). The Rust stub can never be green, so it cannot show that the corpus plus driver are
able to fail on a real behaviour change. Does each planted one-character mutant of `tinyjson.h` get caught, and by
which Judge piece: the Differential corpus (C++ driver built from the mutated header against the original driver,
`harness.py`), the upstream Catch tests built against the mutated header, or both? A mutant that survives means the
corpus must be extended.

METHOD. For every mutant: copy `src/src/tinyjson.h` to `harness/work/mutants/<id>/tinyjson.h` and change exactly
one character (checked: edit distance 1; `src/` is never written). Build, with the reference flags of
`ref/build_ref.sh` (GCC 16, -std=c++17 -include locale -O0), (a) `dump.cpp` and (b) the patched
`ref/tinyjsonTests.cpp` against the mutated copy. Run the upstream tests as `-r xml` and compare the set of failing
test cases and the assertion counts with the unmutated build (17 of 22 pass, 189 assertions, 184 passed: the 5
failures are D3). Run `skill/c-to-rust/scripts/harness.py --orig work/dump --rust <mutated dump>` on the whole
corpus, one mutant at a time. The mutated driver plays the Rust side; it is the same experiment as running a
mutated Rust crate, minus translation noise. Run: `python3 -I e2_tinyjson_judge_sensitivity.py [ID ...]`.

ANSWER (measured 2026-10-12, GCC 16.1.0 -O0, 5534 inputs, ~95 s per mutant; summary.tsv in harness/work/mutants/):
  All 11 mutants are caught by the corpus; none survived, so the corpus needs no extension. The upstream tests
  catch 8 of 11 and miss 3 (m05, m07, m10). Corpus differences per mutant, in brackets:

    m01 isxdigit -> isdigit          corpus (53)   and upstream: assertion count only (failing set unchanged)
    m02 hex loop 4 -> 3              corpus (49)   and upstream (SimpleJson String Parsing Failure)
    m03 trim: TAB -> VT              corpus (42)   and upstream (Tiny Json Number Parsing)
    m04 number ends at '[' not ']'   corpus (3880) and upstream (Tiny Json Array Parsing)
    m05 value may start with ','     corpus (25)   NOT upstream: no test parses a value that starts with '.' through
                                                   parse_value (the number tests call parse_number directly)
    m06 escape f -> F                 corpus (19)   and upstream (Tiny Json String Parsing)
    m07 object separator ' :'        corpus (5534) NOT upstream: no test asserts the serialised text, only the
                                                   round trip (Q6 format is invisible to the upstream suite)
    m08 false prints 'fals'          corpus (5534) and upstream: assertion count only
    m09 array comma after last       corpus (5534) and upstream: assertion count only
    m10 index == size accepted       corpus (5534) NOT upstream (no test reads one past the end; the mutated driver
                                                   aborts, exit -6: an out-of-bounds read)
    m11 to_integer tail check ==     corpus (1160) and upstream (4+ cases)

  "Assertion count only" (m01, m08, m09): the five D3 cases already fail on the reference, so the SET of failing
  cases does not change; the mutant is visible only because the failing case (SimpleJson Serialization) stops
  earlier and passes 10 fewer assertions (184 -> 174). The C++ upstream suite alone is a weak oracle for these
  three; the translated Rust tests assert the D3 value (U+0001) instead, so there the same mutants flip the case
  from pass to fail. m02 was first planted as 4 -> 5: that mutant also made the driver's own D3 pin probe throw,
  so all 5534 inputs differed trivially; 4 -> 3 is the non-trivial version kept here. m07, m08, m09 and m10 differ
  on all 5534 inputs because the `build` and `matrix` sections of the driver run for every input.
"""
from __future__ import annotations

import os
import re
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
PILOT = HERE.parent
REPO = PILOT.parent.parent
HARNESS_DIR = PILOT / "migration" / "harness"
WORK = HARNESS_DIR / "work"
MUT = WORK / "mutants"
SRC = PILOT / "src"
HEADER = SRC / "src" / "tinyjson.h"
HARNESS_PY = REPO / "skill" / "c-to-rust" / "scripts" / "harness.py"
CXX = os.environ.get("CXX", "/opt/homebrew/bin/g++-16")
FLAGS = ["-std=c++17", "-include", "locale", "-O0", "-w"]

# id, what it breaks, text to find (first occurrence), replacement (edit distance 1)
MUTANTS: list[tuple[str, str, str, str]] = [
    ("m01_hex_class", "parse_hex: hex digit class loses a-f (isxdigit -> isdigit)",
     "isxdigit(c)", "isdigit(c)"),
    ("m02_hex_len", "parse_hex: reads 3 digits instead of 4 (loop boundary)",
     "for(int i = 0; i < 4; i++)", "for(int i = 0; i < 3; i++)"),
    ("m03_trim_class", "trim: TAB no longer trimmed (byte class of the default whitespace)",
     'const std::string& whitespace = " \\t\\r\\n"', 'const std::string& whitespace = " \\v\\r\\n"'),
    ("m04_number_end", "parse_number: token ends at '[' instead of ']' (boundary)",
     "c != U',' && c != U']' && c != U'}' )", "c != U',' && c != U'[' && c != U'}' )"),
    ("m05_value_start", "parse_value: a value may start with ',' instead of '.' (byte class)",
     "case U'.':", "case U',':"),
    ("m06_escape_f", "escape_char: \\f becomes \\F",
     "case U'f':\n            c = U'\\f';", "case U'F':\n            c = U'\\f';"),
    ("m07_to_string_sep", "to_string: object separator ' : ' loses the trailing space",
     'ss << " : " ;', 'ss << " :" ;'),
    ("m08_to_string_false", "to_string: boolean false prints 'fals'",
     '? "true" : "false"', '? "true" : "fals"'),
    ("m09_array_comma", "array_to_string: comma written after the LAST element instead of between (!= -> ==)",
     "if (std::next(it) != jarray.end())", "if (std::next(it) == jarray.end())"),
    ("m10_index_bound", "operator[](int): index == size is accepted (>= -> >), reads past the end",
     "(size_t)index >= array->size()", "(size_t)index > array->size()"),
    ("m11_integer_tail", "to_integer: trailing-garbage check inverted (!= -> ==)",
     "if (pos != str.size())", "if (pos == str.size())"),
]


def edit_distance_is_one(a: str, b: str) -> bool:
    if a == b:
        return False
    if len(a) == len(b):
        return sum(x != y for x, y in zip(a, b)) == 1
    if abs(len(a) - len(b)) != 1:
        return False
    s, l = (a, b) if len(a) < len(b) else (b, a)
    for i in range(len(l)):
        if l[:i] + l[i + 1:] == s:
            return True
    return False


def run(cmd: list[str], timeout: int = 600, cwd: Path = WORK) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, capture_output=True, text=True, cwd=cwd, timeout=timeout)


def build(incdir: Path, outdir: Path) -> tuple[bool, str]:
    outdir.mkdir(parents=True, exist_ok=True)
    r1 = run([CXX, *FLAGS, "-I", str(incdir), str(HARNESS_DIR / "dump.cpp"), "-o", str(outdir / "dump")])
    r2 = run([CXX, *FLAGS, "-I", str(incdir), "-I", str(SRC / "test"), str(HARNESS_DIR / "ref" / "tinyjsonTests.cpp"),
              "-o", str(outdir / "tests")])
    ok = r1.returncode == 0 and r2.returncode == 0
    return ok, (r1.stderr + r2.stderr)[-1500:]


def run_tests(exe: Path) -> tuple[frozenset[str], int, int, str]:
    """(failing test names, assertions passed, assertions failed, note) from Catch's xml reporter."""
    try:
        r = run([str(exe), "-r", "xml"], timeout=120)
    except subprocess.TimeoutExpired:
        return frozenset(), -1, -1, "timeout"
    out = r.stdout
    failing = frozenset(re.findall(r'<TestCase name="([^"]*)"[^>]*>(?:(?!</TestCase>).)*?<OverallResult success="false"', out, re.S))
    m = re.search(r'<OverallResults successes="(\d+)" failures="(\d+)"', out)
    if not m:
        return frozenset(), -1, -1, f"no result (exit {r.returncode}: crash)"
    return failing, int(m.group(1)), int(m.group(2)), ""


def corpus(driver: Path, tsv: Path, jobs: int) -> tuple[int, int, str]:
    r = run([sys.executable, "-I", str(HARNESS_PY), "--orig", str(WORK / "dump"), "--rust", str(driver),
             "--inputs", str(HARNESS_DIR / "inputs"), "--out", str(tsv), "--jobs", str(jobs)], timeout=1800)
    rows = [l.rstrip("\n").split("\t") for l in tsv.read_text().splitlines()[1:]] if tsv.is_file() else []
    diffs = [x for x in rows if len(x) > 3 and x[3] == "DIFF"]
    first = ""
    if diffs:
        d = diffs[0]
        m = re.search(r"orig=b'(.*?)'", d[4]) if len(d) > 4 else None
        first = f"{d[0]}: {(m.group(1) if m else d[4])[:70]}" if len(d) > 4 else d[0]
    return len(diffs), len(rows), first


def main() -> int:
    only = set(sys.argv[1:])
    if not (WORK / "dump").is_file():
        print("work/dump missing: run migration/harness/ref/build_ref.sh first", file=sys.stderr)
        return 2
    ver = run([CXX, "--version"]).stdout.splitlines()[0]
    print(f"reference compiler: {ver}", flush=True)
    jobs = os.cpu_count() or 4
    MUT.mkdir(parents=True, exist_ok=True)

    # baseline: the unmutated tests (the reference result: 5 D3 failures)
    ok, err = build(SRC / "src", MUT / "base")
    if not ok:
        print("baseline build failed\n" + err, file=sys.stderr)
        return 2
    base_fail, base_pass, base_fcount, _ = run_tests(MUT / "base" / "tests")
    again = run_tests(MUT / "base" / "tests")
    print(f"baseline upstream tests: {len(base_fail)} failing cases, {base_pass} assertions passed, "
          f"{base_fcount} failed (second run identical: {again[:3] == (base_fail, base_pass, base_fcount)})", flush=True)

    header = HEADER.read_text()
    rows: list[list[str]] = []
    survived: list[str] = []
    for mid, desc, old, new in MUTANTS:
        if only and mid not in only:
            continue
        assert old in header, f"{mid}: pattern not found"
        assert edit_distance_is_one(old, new), f"{mid}: not a one-character edit"
        mdir = MUT / mid
        mdir.mkdir(parents=True, exist_ok=True)
        (mdir / "tinyjson.h").write_text(header.replace(old, new, 1))
        t0 = time.time()
        ok, err = build(mdir, mdir)
        if not ok:
            print(f"{mid}: BUILD FAILED\n{err}", file=sys.stderr)
            rows.append([mid, desc, "build failed", "", "", ""])
            continue
        fail, p, f, note = run_tests(mdir / "tests")
        if note:
            up = f"CAUGHT ({note})"
        elif fail != base_fail:
            up = "CAUGHT (new failing: " + ", ".join(sorted(fail - base_fail))[:80] + ")"
            if base_fail - fail:
                up += " fixed: " + ", ".join(sorted(base_fail - fail))
        elif (p, f) != (base_pass, base_fcount):
            up = f"CAUGHT (assertion counts {p}/{f} vs {base_pass}/{base_fcount})"
        else:
            up = "survived"
        nd, nt, first = corpus(mdir / "dump", mdir / "diff.tsv", jobs)
        co = f"CAUGHT ({nd} of {nt} inputs differ)" if nd else "survived"
        which = "both" if up.startswith("CAUGHT") and nd else "upstream only" if up.startswith("CAUGHT") else \
            "corpus only" if nd else "NONE (survived)"
        if which.startswith("NONE"):
            survived.append(mid)
        rows.append([mid, desc, up, co, which, first])
        print(f"{mid}: upstream {up}; corpus {co}; => {which}  [{time.time() - t0:.0f} s]", flush=True)
    (MUT / "summary.tsv").write_text(
        "id\tmutant\tupstream tests\tcorpus\tcaught by\tfirst corpus diff\n" + "\n".join("\t".join(r) for r in rows) + "\n")
    print(f"\n{len(rows) - len(survived)} of {len(rows)} mutants caught; survivors: {survived or 'none'}")
    print(f"summary: {MUT / 'summary.tsv'}")
    return 1 if survived else 0


if __name__ == "__main__":
    sys.exit(main())
