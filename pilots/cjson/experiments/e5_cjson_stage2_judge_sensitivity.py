#!/usr/bin/env python3
"""E5: can the cJSON STAGE 2 Judge detect a one-character behaviour change in the Faithful port?

Question: `run.py --stage 2` (the translated tests `rust/tests/upstream.rs` through the adapter, then the
Rust example driver `rust/examples/dump.rs` against the original C driver) passes on the accepted crate
`pilots/cjson/rust` in both feature sets. Is that evidence, or is the Stage 2 Judge blind? This plants
one-character mutants (one integer literal inside one function: a byte-class or boundary check) in
`rust/src/cJSON.rs` and `rust/src/cJSON_Utils.rs`, runs `migration/harness/run.py --stage 2 <set>` for both
feature sets (the files are shared by both sets), requires a red result, restores the file byte for byte, and
reports which Judge piece went red: the translated tests ("tests") or the Differential harness alone
(`--skip-tests`, "harness"). Afterwards it checks that `git diff` of `rust/src` is empty.

This is the Stage 2 counterpart of e4_cjson_judge_sensitivity.py (Stage 1: upstream C tests + harness).
The mutants are the same kind and, where possible, the same sites, so the two tables can be compared.

Usage: python3 -I e5_cjson_stage2_judge_sensitivity.py [--only NAME] [--probe FUNCTION FILE]
       `--probe` prints the integer literals (value: count) that occur in one function of the crate, used to
       choose mutants; it prints no source text. One mutant costs 2 feature sets x 2 runs of about a minute.

Answer: see the table printed by a run and recorded in pilots/cjson/migration/RUN-NOTES.md ("Stage 2 Judge").
Rule when a mutant survives: extend the corpus (make_inputs.py) or the translated tests, do not change the
mutant.
"""

from __future__ import annotations

import re
import signal
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
PILOT = HERE.parent
REPO = PILOT.parent.parent
RUN = PILOT / "migration" / "harness" / "run.py"
SRC = PILOT / "rust" / "src"
FEATURE_SETS = ["", "locales"]


@dataclass(frozen=True)
class Mutant:
    name: str
    file: str  # cJSON.rs or cJSON_Utils.rs
    function: str
    old: str  # integer literal (whole token) to change
    new: str
    nth: int  # which occurrence inside the function, 0-based
    why: str  # the C site and what the change breaks


MUTANTS = [
    Mutant("ws-boundary", "cJSON.rs", "buffer_skip_whitespace", "32", "31", 0,
           "cJSON.c:1102 whitespace is every byte <= 32 -> <= 31: a space is no longer skipped"),
    Mutant("hex-digit", "cJSON.rs", "parse_hex4", "9", "8", 0,
           "cJSON.c:675 hex digit class '0'..'9' -> '0'..'8': \\u0009 is no longer a valid escape"),
    Mutant("string-escape-boundary", "cJSON.rs", "print_string_ptr", "32", "31", 0,
           "cJSON.c:1040 bytes < 32 are escaped as \\u00xx -> < 31: byte 0x1f printed raw"),
    Mutant("array-index-digit", "cJSON_Utils.rs", "decode_array_index_from_pointer", "9", "8", 0,
           "cJSON_Utils.c:285 index digits '0'..'9' -> '0'..'8': a '9' in an array index is rejected"),
]


def body_span(text: str, function: str) -> tuple[int, int] | None:
    m = re.search(rf"^(?:pub )?unsafe extern \"C\" fn {re.escape(function)}\(", text, flags=re.M)
    if m is None:
        return None
    end = re.search(r"^(?:#\[|pub |unsafe |static |extern |type |const |impl |fn )", text[m.end():], flags=re.M)
    return m.start(), m.end() + (end.start() if end else len(text) - m.end())


def literal_positions(body: str, token: str) -> list[int]:
    return [m.start() for m in re.finditer(rf"(?<![\w.]){re.escape(token)}(?![\w.])", body)]


def probe(function: str, file: str) -> int:
    text = (SRC / file).read_text()
    span = body_span(text, function)
    if span is None:
        print(f"{function}: not found in {file}")
        return 1
    body = text[span[0]:span[1]]
    lits: dict[str, int] = {}
    for m in re.finditer(r"(?<![\w.])\d+(?![\w.])", body):
        lits[m.group()] = lits.get(m.group(), 0) + 1
    shown = dict(sorted(lits.items(), key=lambda kv: int(kv[0])))
    print(f"{function}: {len(body.splitlines())} lines, integer literals {shown}")
    return 0


def run_judge(feature_set: str, skip_tests: bool = False) -> tuple[int, str]:
    cmd = [sys.executable, "-I", str(RUN), "--stage", "2"] + (["--skip-tests"] if skip_tests else [])
    cmd += [feature_set] if feature_set else []
    r = subprocess.run(cmd, capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def classify(out: str) -> str:
    """Which Judge piece went red first."""
    if "cargo build" in out and "failed" in out and "translated tests" not in out:
        return "BUILD BROKEN (not a behaviour mutant)"
    m = re.search(r"translated tests \[[^\]]*\]: (\d+) passed, (\d+) failed", out)
    if m and int(m.group(2)) > 0:
        names = re.findall(r"^test (\S+) \.\.\. FAILED$", out, flags=re.M)
        return f"translated tests: {m.group(2)} failing ({', '.join(names[:4])})"
    m = re.search(r"harness: (\d+)/(\d+) inputs match", out)
    if m and m.group(1) != m.group(2):
        return f"differential harness: {int(m.group(2)) - int(m.group(1))} of {m.group(2)} inputs DIFF"
    if m:
        return "NOT DETECTED"
    return "no verdict: " + out.strip().splitlines()[-1][:120] if out.strip() else "no output"


def src_diff_is_empty() -> bool:
    r = subprocess.run(["git", "-C", str(REPO), "diff", "--quiet", "--", str(SRC)])
    return r.returncode == 0


def main() -> int:
    argv = sys.argv[1:]
    if argv[:1] == ["--probe"] and len(argv) == 3:
        return probe(argv[1], argv[2])
    only = None
    if argv[:1] == ["--only"] and len(argv) == 2:
        only = argv[1]
    elif argv:
        print(__doc__.split("Usage:")[1].split("Answer:")[0].strip(), file=sys.stderr)
        return 2
    # a SIGTERM must run the `finally` that restores the mutated file
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(143))
    if not src_diff_is_empty():
        print("e5: rust/src has uncommitted changes; refusing to mutate a dirty tree", file=sys.stderr)
        return 2
    survived = 0
    rows: list[str] = []
    for mu in MUTANTS:
        if only and mu.name != only:
            continue
        path = SRC / mu.file
        original = path.read_bytes()
        text = original.decode()
        span = body_span(text, mu.function)
        if span is None:
            print(f"e5: {mu.name}: function {mu.function} not found in {mu.file}")
            survived += 1
            continue
        pos = literal_positions(text[span[0]:span[1]], mu.old)
        if len(pos) <= mu.nth:
            print(f"e5: {mu.name}: literal {mu.old} occurrence {mu.nth} not found ({len(pos)} occurrences)")
            survived += 1
            continue
        p = span[0] + pos[mu.nth]
        line_no = text.count("\n", 0, p) + 1
        line = text.splitlines()[line_no - 1].strip()
        print(f"e5: {mu.name}: {mu.file}:{line_no} `{line}`  {mu.old} -> {mu.new}  ({mu.why})", flush=True)
        try:
            path.write_bytes((text[:p] + mu.new + text[p + len(mu.old):]).encode())
            for fs in FEATURE_SETS:
                label = f"{mu.name} [{fs or 'default'}]"
                code, out = run_judge(fs)
                verdict = classify(out)
                # second run: the Differential harness alone, to see whether it would also have caught it
                code2, out2 = run_judge(fs, skip_tests=True)
                detected = code != 0 and verdict != "NOT DETECTED" and not verdict.startswith("BUILD BROKEN")
                survived += not detected
                print(f"e5: {label}: {'DETECTED' if detected else 'NOT DETECTED'} (exit {code}): {verdict}", flush=True)
                print(f"e5: {label}: harness alone: exit {code2}: {classify(out2)}", flush=True)
                rows.append(f"{mu.name}\t{fs or 'default'}\t{mu.file}:{line_no}\t{verdict}\t{classify(out2)}")
        finally:
            path.write_bytes(original)
    clean = src_diff_is_empty()
    print(f"e5: git diff of rust/src after restore: {'empty' if clean else 'NOT EMPTY'}")
    print("e5: mutant\tset\tsite\tfirst red piece\tharness alone")
    for r in rows:
        print("e5: " + r)
    print(f"e5: {survived} mutant run(s) not detected" if survived else "e5: all mutants detected", file=sys.stderr)
    return 1 if survived or not clean else 0


if __name__ == "__main__":
    sys.exit(main())
