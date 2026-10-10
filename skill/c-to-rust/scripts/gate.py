#!/usr/bin/env python3
"""Baseline-aware gate for one Part: check, test, harness, then (Stage 2) unsafe count, clippy, rustfmt.

Flow:  gate.py PART --crate DIR --stage N --snapshot    # before changing the Part: record HEAD + baseline
       ... edit the Part ...
       gate.py PART --crate DIR --stage N               # run checks; on regression restore the Part's path

Checks run once per feature set (`--features ""` `--features strict` ...; default: one run, no features), in
order, stopping at the first regression. A check that already failed in the baseline is not a regression.
`--harness-cmd` is a shell command with `{features}` replaced by the feature list; exit 0 = pass.
Rollback is path-scoped (decision 17): `git checkout <HEAD> -- <path>` plus `git clean` of that path, for the
Part's target_path and every `--also PATH` (relative to the workspace root, e.g. rust/tests/adapter/mod.rs).
Cargo is called with `--manifest-path` (decision 25); pass `--toolchain nightly-2023-04-15` if needed.
Output: TSV lines `check<TAB>features<TAB>pass|fail|skip<TAB>detail`, summary on stderr. Exit 0 pass, 1 fail.
"""

from __future__ import annotations

import argparse
import json
import re
import shlex
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

ERROR_CODE = re.compile(r"^error(?:\[(E\d+)\])?: (.*)$", re.M)


@dataclass(frozen=True)
class Result:
    check: str
    features: str
    status: str  # pass | fail | skip
    detail: str = ""

    @property
    def key(self) -> str:
        return f"{self.check}[{self.features}]"


def run(cmd: list[str]) -> tuple[int, str]:
    r = subprocess.run(cmd, capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def summarize_rustc(output: str) -> str:
    """First error text plus counts grouped by error code, e.g. `E0308 x3; first: mismatched types`."""
    found = [(m.group(1) or "error", m.group(2)) for m in ERROR_CODE.finditer(output)]
    found = [f for f in found if not f[1].startswith("aborting due to") and "could not compile" not in f[1]]
    if not found:
        return " ".join(output.split())[:300]
    counts: dict[str, int] = {}
    for code, _ in found:
        counts[code] = counts.get(code, 0) + 1
    groups = ", ".join(f"{c} x{n}" for c, n in counts.items())
    return f"{groups}; first: {found[0][1]}"[:300]


def count_unsafe(crate: Path) -> int:
    total = 0
    for p in (crate / "src").rglob("*.rs"):
        for line in p.read_text().splitlines():
            code = line.split("//", 1)[0]
            total += len(re.findall(r"\bunsafe\b", code))
    return total


def cargo_cmd(sub: list[str], crate: Path, features: str, toolchain: str | None) -> list[str]:
    cmd = ["cargo"] + ([f"+{toolchain}"] if toolchain else []) + sub + ["--manifest-path", str(crate / "Cargo.toml")]
    if features:
        cmd += ["--features", features]
    return cmd


def run_checks(
    crate: Path,
    stage: int,
    feature_sets: list[str],
    harness_cmd: str | None,
    toolchain: str | None,
    baseline: set[str],
    stop_on_regression: bool,
) -> list[Result]:
    results: list[Result] = []

    def record(res: Result) -> bool:
        results.append(res)
        return res.status == "fail" and res.key not in baseline and stop_on_regression

    for feats in feature_sets:
        steps: list[tuple[str, list[str] | None]] = [
            ("check", cargo_cmd(["check", "-q"], crate, feats, toolchain)),
            ("test", cargo_cmd(["test", "-q"], crate, feats, toolchain)),
        ]
        if harness_cmd:
            steps.append(("harness", shlex.split(harness_cmd.replace("{features}", feats))))
        if stage == 2:
            steps.append(("clippy", cargo_cmd(["clippy", "-q"], crate, feats, toolchain) + ["--", "-D", "warnings"]))
            fmt = ["cargo"] + ([f"+{toolchain}"] if toolchain else []) + ["fmt", "--check"]
            steps.append(("fmt", fmt + ["--manifest-path", str(crate / "Cargo.toml")]))
        for name, cmd in steps:
            assert cmd is not None
            code, out = run(cmd)
            status = "pass" if code == 0 else "fail"
            detail = "" if code == 0 else summarize_rustc(out)
            if record(Result(name, feats, status, detail)):
                return results
    if stage == 2:
        n = count_unsafe(crate)
        record(Result("unsafe", "", "pass" if n == 0 else "fail", f"{n} unsafe token(s)" if n else ""))
    else:
        record(Result("unsafe", "", "skip", f"{count_unsafe(crate)} unsafe token(s) (Stage 1: informational)"))
    return results


def target_of(parts: Path, part: str) -> str:
    lines = [ln.split("\t") for ln in parts.read_text().splitlines() if ln.strip()]
    header = lines[0]
    for cells in lines[1:]:
        if cells[header.index("part_id")] == part:
            return cells[header.index("target_path")]
    raise ValueError(f"part {part!r} not in {parts}")


def git(root: Path, *args: str) -> str:
    r = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)}: {r.stderr.strip()}")
    return r.stdout.strip()


def rollback(root: Path, head: str, paths: list[str]) -> None:
    """Restore every path (target plus `--also` paths) to `head` and delete untracked files under them."""
    for path in paths:
        git(root, "checkout", head, "--", path)
        git(root, "clean", "-fdq", "--", path)


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("part")
    p.add_argument("--crate", required=True, help="path to the Rust crate (Cargo.toml dir)")
    p.add_argument("--stage", type=int, choices=[1, 2], required=True)
    p.add_argument("--parts", default="migration/parts.tsv")
    p.add_argument("--features", action="append", help="feature set to check; repeat for a matrix")
    p.add_argument("--harness-cmd", help="shell command; {features} is replaced; exit 0 = pass")
    p.add_argument("--toolchain", help="rustup toolchain name for cargo, e.g. nightly-2023-04-15")
    p.add_argument("--also", action="append", default=[], help="extra path restored with the target; repeatable")
    p.add_argument("--snapshot", action="store_true", help="record HEAD and baseline, change nothing")
    p.add_argument("--no-rollback", action="store_true", help="report only; keep the working tree as is")
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    parts = Path(args.parts)
    root = parts.resolve().parent.parent  # workspace root = parent of migration/
    crate = Path(args.crate)
    feature_sets: list[str] = args.features if args.features is not None else [""]
    snap = parts.resolve().parent / "gate" / f"{args.part}.stage{args.stage}.json"
    try:
        target = target_of(parts, args.part)
        if args.snapshot:
            results = run_checks(crate, args.stage, feature_sets, args.harness_cmd, args.toolchain, set(), False)
            snap.parent.mkdir(exist_ok=True)
            snap.write_text(
                json.dumps(
                    {
                        "head": git(root, "rev-parse", "HEAD"),
                        "failing": [r.key for r in results if r.status == "fail"],
                        "paths": [target, *args.also],
                    }
                )
            )
            failing = sum(r.status == "fail" for r in results)
            print(f"gate: snapshot of {args.part} recorded ({failing} pre-existing failure(s))", file=sys.stderr)
            return 0
        saved = (
            json.loads(snap.read_text()) if snap.exists() else {"head": git(root, "rev-parse", "HEAD"), "failing": []}
        )
        baseline = set(saved["failing"])
        paths = list(dict.fromkeys([target, *saved.get("paths", []), *args.also]))
        results = run_checks(crate, args.stage, feature_sets, args.harness_cmd, args.toolchain, baseline, True)
    except (ValueError, RuntimeError, OSError) as e:
        print(f"gate: {e}", file=sys.stderr)
        return 1
    for r in results:
        print(f"{r.check}\t{r.features}\t{r.status}\t{r.detail}")
    regressions = [r for r in results if r.status == "fail" and r.key not in baseline]
    if regressions:
        first = regressions[0]
        if not args.no_rollback:
            rollback(root, str(saved["head"]), paths)
        action = "kept working tree" if args.no_rollback else f"restored {', '.join(paths)} to {str(saved['head'])[:8]}"
        print(f"gate: FAIL {args.part}: {first.key}: {first.detail} ({action})", file=sys.stderr)
        return 1
    ok = sum(r.status == "pass" for r in results)
    print(f"gate: PASS {args.part} ({ok} checks passed, stage {args.stage})", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
