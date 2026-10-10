#!/usr/bin/env python3
"""E4: can the cJSON Stage 1 Judge detect a one-character behaviour change in the Rust crate?

Question: the upstream tests plus the Differential harness pass on the c2rust SCRATCH crates (ticket 02). Is that
evidence, or is the Judge blind? For each feature set (`""`, `locales`) this plants one-character mutants in the
scratch crate's Rust source (one integer literal inside one function: a byte-class or boundary check), runs
`migration/harness/run.py --scratch <set>`, requires a red result, restores the file, and reports which Judge
piece went red (the upstream tests linked to the staticlib, or the Differential harness).

Usage: python3 -I e4_cjson_judge_sensitivity.py [--only NAME] [--probe FUNCTION FILE]
       Restores each mutated file even on error. `--probe` prints the integer literals (value: count) that
       occur in one function of the scratch crates, used to choose mutants; it prints no source text.
       One full Judge run takes about 3 minutes; the whole experiment about 12 runs.

Answer: see the results table printed by a run and recorded in pilots/cjson/migration/RUN-NOTES.md
("Judge sensitivity"). Rule when a mutant survives: extend the corpus (make_inputs.py), do not change the mutant.
"""

from __future__ import annotations

import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
HARNESS = HERE if (HERE / "run.py").exists() else HERE.parent / "migration" / "harness"
RUN = HARNESS / "run.py"
SCRATCH = HARNESS / "build" / "scratch"


@dataclass(frozen=True)
class Mutant:
    name: str
    feature_set: str  # "" or "locales"
    file: str  # cJSON.rs or cJSON_Utils.rs
    function: str
    old: str  # integer literal (whole token) to change
    new: str
    nth: int  # which occurrence inside the function, 0-based
    why: str  # the C site and what the change breaks


MUTANTS = [
    Mutant("ws-boundary", "", "cJSON.rs", "buffer_skip_whitespace", "32", "31", 0,
           "cJSON.c:1102 whitespace is every byte <= 32 -> <= 31: a space is no longer skipped"),
    Mutant("string-escape-boundary", "", "cJSON.rs", "print_string_ptr", "32", "31", 0,
           "cJSON.c:1040 bytes < 32 are escaped as \\u00xx -> < 31: byte 0x1f printed raw"),
    Mutant("array-index-digit", "", "cJSON_Utils.rs", "decode_array_index_from_pointer", "9", "8", 0,
           "cJSON_Utils.c:285 index digits '0'..'9' -> '0'..'8': a '9' in an array index is rejected"),
    Mutant("hex-digit", "", "cJSON.rs", "parse_hex4", "9", "8", 0,
           "cJSON.c:675 hex digit class '0'..'9' -> '0'..'8': \\u0009 is no longer a valid escape"),
    Mutant("ws-boundary", "locales", "cJSON.rs", "buffer_skip_whitespace", "32", "31", 0,
           "as above, in the locales crate"),
    Mutant("array-index-digit", "locales", "cJSON_Utils.rs", "decode_array_index_from_pointer", "9", "8", 0,
           "as above, in the locales crate"),
    Mutant("number-dot", "locales", "cJSON.rs", "parse_number", "46", "47", 0,
           "cJSON.c:370 '.' is accepted inside a number and replaced by the locale decimal point -> '/'"),
    Mutant("decimal-point-offset", "locales", "cJSON.rs", "get_decimal_point", "0", "1", 0,
           "cJSON.c:281-286 first byte of localeconv()->decimal_point -> second byte (NUL)"),
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
    for fs in ("default", "locales"):
        text = (SCRATCH / fs / "src" / file).read_text()
        span = body_span(text, function)
        if span is None:
            print(f"[{fs}] {function}: not found in {file}")
            continue
        body = text[span[0]:span[1]]
        lits: dict[str, int] = {}
        for m in re.finditer(r"(?<![\w.])\d+(?![\w.])", body):
            lits[m.group()] = lits.get(m.group(), 0) + 1
        shown = dict(sorted(lits.items(), key=lambda kv: int(kv[0])))
        print(f"[{fs}] {function}: {len(body.splitlines())} lines, integer literals {shown}")
    return 0


def run_judge(feature_set: str, skip_tests: bool = False) -> tuple[int, str]:
    cmd = [sys.executable, "-I", str(RUN), "--scratch"] + (["--skip-tests"] if skip_tests else [])
    cmd += [feature_set] if feature_set else []
    r = subprocess.run(cmd, capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def classify(out: str) -> str:
    """Which Judge piece went red first."""
    if "cargo build failed" in out:
        return "BUILD BROKEN (not a behaviour mutant)"
    if "cannot be built/linked" in out:
        return "link failure"
    red = re.findall(r"^\s+(\w+)\s+\d+ tests \d+ failures[^|]*\| \d+ tests ([1-9]\d*) failures", out, flags=re.M)
    if red:
        return "upstream tests: " + ", ".join(f"{n} ({k} failures)" for n, k in red)
    m = re.search(r"harness: (\d+)/(\d+) inputs match", out)
    if m and m.group(1) != m.group(2):
        rows = re.findall(r"^  ([\w.-]+\.json)\t", out, flags=re.M)
        return (f"differential harness: {int(m.group(2)) - int(m.group(1))} of {m.group(2)} inputs DIFF; "
                f"first: {', '.join(rows[:4])}")
    return "NOT DETECTED"


def main() -> int:
    argv = sys.argv[1:]
    if argv[:1] == ["--probe"] and len(argv) == 3:
        return probe(argv[1], argv[2])
    only = argv[1] if argv[:1] == ["--only"] and len(argv) == 2 else None
    survived = 0
    for mu in MUTANTS:
        if only and mu.name != only:
            continue
        label = f"{mu.name} [{mu.feature_set or 'default'}]"
        path = SCRATCH / (mu.feature_set or "default") / "src" / mu.file
        original = path.read_text()
        span = body_span(original, mu.function)
        if span is None:
            print(f"e4: {label}: function {mu.function} not found in {mu.file}")
            survived += 1
            continue
        pos = literal_positions(original[span[0]:span[1]], mu.old)
        if len(pos) <= mu.nth:
            print(f"e4: {label}: literal {mu.old} occurrence {mu.nth} not found ({len(pos)} occurrences)")
            survived += 1
            continue
        p = span[0] + pos[mu.nth]
        line_no = original.count("\n", 0, p) + 1
        line = original.splitlines()[line_no - 1].strip()
        print(f"e4: {label}: {mu.file}:{line_no} `{line}`  {mu.old} -> {mu.new}  ({mu.why})", flush=True)
        try:
            path.write_text(original[:p] + mu.new + original[p + len(mu.old):])
            code, out = run_judge(mu.feature_set)
            # second run: the Differential harness alone, to see whether it would also have caught it
            code2, out2 = run_judge(mu.feature_set, skip_tests=True)
        finally:
            path.write_text(original)
        verdict = classify(out)
        detected = code != 0 and verdict != "NOT DETECTED" and not verdict.startswith("BUILD BROKEN")
        survived += not detected
        print(f"e4: {label}: {'DETECTED' if detected else 'NOT DETECTED'} (exit {code}): {verdict}", flush=True)
        print(f"e4: {label}: harness alone: exit {code2}: {classify(out2)}", flush=True)
    print(f"e4: {survived} mutant(s) not detected" if survived else "e4: all mutants detected", file=sys.stderr)
    return 1 if survived else 0


if __name__ == "__main__":
    sys.exit(main())
