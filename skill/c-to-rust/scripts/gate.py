#!/usr/bin/env python3
"""Baseline-aware gate for one Part: check, test, harness, then (Stage 2) unsafe count, clippy, rustfmt.

Flow:  gate.py PART --crate DIR --stage N --snapshot    # before changing the Part: record HEAD + baseline
       ... edit the Part ...
       gate.py PART --crate DIR --stage N               # run checks; on regression restore the Part's path

Checks run once per feature set (`--features ""` `--features strict` ...; default: one run, no features), in
order, stopping at the first regression. A check that already failed in the baseline is not a
regression, except at Stage 2 the Judge checks `test` and `harness`, which always count.
A change to the Rulebook, `migration/harness/**` (not `inputs/`) or the crate's `Cargo.toml`, `tests/**`,
`examples/**` between snapshot and gate fails the attempt (hashes in the snapshot, decision 63) unless
`--accept-judge-change` is passed; those files are not rolled back.
`--strict-judge` makes them always count at Stage 1 too; without it a PASS lists the checks the baseline
excused (`1 of 3 checks passed; EXCUSED ...`), because a stub baseline fails the Judge and the Part still passes.
`--harness-cmd` is a shell command with `{features}` replaced by the feature list; exit 0 = pass.
Rollback is path-scoped (decision 17): `git checkout <HEAD> -- <path>` plus `git clean` of that path, for the
Part's target_path and every `--also PATH` (relative to the workspace root, e.g. rust/tests/adapter/mod.rs).
Cargo is called with `--manifest-path` (decision 25); pass `--toolchain nightly-2023-04-15` if needed.
A `--snapshot` appends `part, stage, head, time` to `migration/attempts.tsv`: one row is one implementer run, the
count `workqueue.py` uses for attempts. Snapshot only at the start of a run, never to "refresh" a baseline.
A PASS appends `part, stage, head, features, time, excused` to `migration/gate-passed.tsv`; `workqueue.py verify`
requires such a row for every `done` Part (decision 7: done = gate passed and commit exists).
Output: TSV lines `check<TAB>features<TAB>pass|fail|skip<TAB>detail`, summary on stderr. Exit 0 pass, 1 fail.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shlex
import subprocess
import sys
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path

# At Stage 2 the Judge checks are never excused by the baseline: a snapshot taken while the Judge already
# fails would otherwise let a still-failing attempt print PASS (found in E3 Stage 2 attempt 2). Stage 1 keeps
# the baseline for them, because a half-translated crate may not compile while other Parts are `todo`.
JUDGE_CHECKS = {"test", "harness"}

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

    def regresses(self, baseline: set[str], stage: int = 1) -> bool:
        """A failed check counts unless the baseline already had it; at Stage 2 Judge checks always count."""
        return self.status == "fail" and (self.key not in baseline or (stage == 2 and self.check in JUDGE_CHECKS))


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


UNWRAP = re.compile(r"\.unwrap\(\)|\.expect\(")
ALLOW = re.compile(r"#!?\[allow\(([^)]*)\)\]")


TEST_RESULT = re.compile(r"^test result: \w+\. (\d+) passed", re.M)


def count_passed(output: str) -> int:
    """Sum of `N passed` over every `test result:` line (lib, each tests/*.rs binary, doctests)."""
    return sum(int(n) for n in TEST_RESULT.findall(output))


def find_unwraps(crate: Path) -> list[str]:
    """`file:line` of every `.unwrap()`/`.expect(` in src/ not justified by `// invariant:` (same or previous line)."""
    found: list[str] = []
    for p in sorted((crate / "src").rglob("*.rs")):
        prev = ""
        for no, line in enumerate(p.read_text().splitlines(), 1):
            code = line.split("//", 1)[0]
            if UNWRAP.search(code) and "// invariant:" not in line and "// invariant:" not in prev:
                found.append(f"{p.name}:{no}")
            prev = line
    return found


def find_blanket_allows(crate: Path, permitted: list[str]) -> list[str]:
    """`file:line lint` of every `#[allow(lint)]` in src/ whose lint is not in `permitted`."""
    found: list[str] = []
    for p in sorted((crate / "src").rglob("*.rs")):
        for no, line in enumerate(p.read_text().splitlines(), 1):
            for m in ALLOW.finditer(line.split("//", 1)[0]):
                found += [
                    f"{p.name}:{no} {lint}"
                    for lint in (x.strip() for x in m.group(1).split(","))
                    if lint not in permitted
                ]
    return found


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
    permitted_lints: list[str] | None = None,
    min_tests: int = 0,
    strict_judge: bool = False,
) -> list[Result]:
    results: list[Result] = []
    judge_stage = 2 if strict_judge else stage  # `regresses` never excuses Judge checks at stage 2

    def record(res: Result) -> bool:
        results.append(res)
        return res.regresses(baseline, judge_stage) and stop_on_regression

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
            if name == "test" and code == 0 and count_passed(out) < min_tests:
                status, detail = "fail", f"{count_passed(out)} test(s) passed, need at least {min_tests}"
            if record(Result(name, feats, status, detail)):
                return results
    if stage == 2:
        n = count_unsafe(crate)
        record(Result("unsafe", "", "pass" if n == 0 else "fail", f"{n} unsafe token(s)" if n else ""))
        unwraps = find_unwraps(crate)
        record(
            Result(
                "unwrap",
                "",
                "fail" if unwraps else "pass",
                ", ".join(unwraps[:5]) + f" ({len(unwraps)} unjustified)" if unwraps else "",
            )
        )
        allows = find_blanket_allows(crate, permitted_lints or [])
        record(Result("allow", "", "fail" if allows else "pass", "; ".join(allows[:5]) if allows else ""))
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


def judge_hashes(root: Path, crate: Path) -> dict[str, str]:
    """SHA-256 of the files an implementer must not change: the Rulebook, the harness, the crate's tests/examples/Cargo.

    `migration/harness/inputs/` (generated corpus) and build output are left out. The gate compares these with the
    snapshot (decision 63): a changed file fails the attempt unless the governor passes `--accept-judge-change`.
    """
    files: list[Path] = [root / "migration" / "RULEBOOK.md", crate / "Cargo.toml"]
    for base, pattern in (
        (root / "migration" / "harness", "**/*"),
        (crate / "tests", "**/*"),
        (crate / "examples", "**/*"),
    ):
        files += [f for f in sorted(base.glob(pattern)) if f.is_file()]
    skip = ("/inputs/", "/target/", "/__pycache__/")
    out: dict[str, str] = {}
    for f in files:
        key = (
            f.resolve().relative_to(root.resolve()).as_posix() if f.resolve().is_relative_to(root.resolve()) else str(f)
        )
        if f.is_file() and not any(part in f"/{key}" for part in skip):
            out[key] = hashlib.sha256(f.read_bytes()).hexdigest()
    return out


def changed_judge_files(before: dict[str, str], after: dict[str, str]) -> list[str]:
    return sorted(k for k in before.keys() | after.keys() if before.get(k) != after.get(k))


def record_pass(root: Path, part: str, stage: int, features: list[str], excused: list[str] | None = None) -> None:
    """Append one row to migration/gate-passed.tsv (tracked, unlike gate/*.json) for `workqueue.py verify`.

    `excused` lists the checks that failed but were excused by the baseline; `workqueue.py mark done` refuses a Part
    whose latest pass has any (decision 65).
    """
    path = root / "migration" / "gate-passed.tsv"
    header = "part\tstage\thead\tfeatures\ttime\texcused\n"
    if not path.is_file():
        path.write_text(header)
    elif not path.read_text().split("\n", 1)[0].endswith("\texcused"):
        first, _, rest = path.read_text().partition("\n")
        path.write_text(first + "\texcused\n" + rest)  # older ledger: old rows read as "nothing excused"
    row = [
        part,
        str(stage),
        git(root, "rev-parse", "HEAD"),
        "|".join(features),
        datetime.now(UTC).isoformat(timespec="seconds"),
        "|".join(excused or []),
    ]
    with path.open("a") as f:
        f.write("\t".join(row) + "\n")


def record_snapshot(root: Path, part: str, stage: int) -> None:
    """Append one row to migration/attempts.tsv: a snapshot is the start of one implementer run (an attempt).

    `workqueue.py mark done` and `verify` take the Part's attempts from here, so the count does not depend on the
    governor remembering `mark in_progress` before every retry.
    """
    path = root / "migration" / "attempts.tsv"
    if not path.is_file():
        path.write_text("part\tstage\thead\ttime\n")
    row = [part, str(stage), git(root, "rev-parse", "HEAD"), datetime.now(UTC).isoformat(timespec="seconds")]
    with path.open("a") as f:
        f.write("\t".join(row) + "\n")


def rollback(root: Path, head: str, paths: list[str]) -> None:
    """Restore every path (target plus `--also` paths) to `head` and delete untracked files under them.

    A path that does not exist at `head` (an untracked Judge file passed with `--also`) is left alone: deleting it
    would destroy work that no commit holds.
    """
    for path in paths:
        probe = subprocess.run(["git", "-C", str(root), "cat-file", "-e", f"{head}:./{path}"], capture_output=True)
        if probe.returncode != 0:
            print(f"gate: {path} is not in {head[:8]}: left as is (commit it to have it restored)", file=sys.stderr)
            continue
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
    p.add_argument("--min-tests", type=int, default=0, help="fail `cargo test` if fewer tests pass (per feature set)")
    p.add_argument("--allow-lint", action="append", default=[], help="lint a Stage 2 `#[allow]` may name; repeatable")
    p.add_argument("--toolchain", help="rustup toolchain name for cargo, e.g. nightly-2023-04-15")
    p.add_argument("--also", action="append", default=[], help="extra path restored with the target; repeatable")
    p.add_argument(
        "--strict-judge",
        action="store_true",
        help="Stage 1: never excuse `test`/`harness` failures the baseline already had (once the Judge compiles)",
    )
    p.add_argument(
        "--accept-judge-change",
        action="store_true",
        help="the Rulebook/Judge files changed since the snapshot on purpose (governor edit); do not fail the attempt",
    )
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
            snap.parent.mkdir(parents=True, exist_ok=True)
            snap.write_text(
                json.dumps(
                    {
                        "head": git(root, "rev-parse", "HEAD"),
                        "failing": [r.key for r in results if r.status == "fail"],
                        "paths": [target, *args.also],
                        "judge": judge_hashes(root, crate),
                    }
                )
            )
            record_snapshot(root, args.part, args.stage)
            failing = sum(r.status == "fail" for r in results)
            print(f"gate: snapshot of {args.part} recorded ({failing} pre-existing failure(s))", file=sys.stderr)
            return 0
        saved = (
            json.loads(snap.read_text()) if snap.exists() else {"head": git(root, "rev-parse", "HEAD"), "failing": []}
        )
        baseline = set(saved["failing"])
        paths = list(dict.fromkeys([target, *saved.get("paths", []), *args.also]))
        results = run_checks(
            crate,
            args.stage,
            feature_sets,
            args.harness_cmd,
            args.toolchain,
            baseline,
            True,
            args.allow_lint,
            args.min_tests,
            args.strict_judge,
        )
    except (ValueError, RuntimeError, OSError) as e:
        print(f"gate: {e}", file=sys.stderr)
        return 1
    for r in results:
        print(f"{r.check}\t{r.features}\t{r.status}\t{r.detail}")
    regressions = [r for r in results if r.regresses(baseline, 2 if args.strict_judge else args.stage)]
    if regressions:
        first = regressions[0]
        if not args.no_rollback:
            rollback(root, str(saved["head"]), paths)
        action = "kept working tree" if args.no_rollback else f"restored {', '.join(paths)} to {str(saved['head'])[:8]}"
        print(f"gate: FAIL {args.part}: {first.key}: {first.detail} ({action})", file=sys.stderr)
        return 1
    judge_raw = saved.get("judge")
    recorded: dict[str, str] = {str(k): str(v) for k, v in judge_raw.items()} if isinstance(judge_raw, dict) else {}
    if recorded and not args.accept_judge_change:
        changed = [
            c
            for c in changed_judge_files(recorded, judge_hashes(root, crate))
            if not any(c == q or c.startswith(q.rstrip("/") + "/") for q in paths)  # `--also` paths are the attempt's
        ]
        if changed:
            if not args.no_rollback:
                rollback(root, str(saved["head"]), [target])  # never the Judge files: the governor may own the edit
            print(
                f"gate: FAIL {args.part}: Rulebook/Judge files changed since the snapshot: {', '.join(changed[:8])}"
                f"{' ...' if len(changed) > 8 else ''}. Review them; if the change is yours, re-run with"
                " --accept-judge-change (the target was restored; the changed files were not)",
                file=sys.stderr,
            )
            return 1
    ok = sum(r.status == "pass" for r in results)
    ran = sum(r.status != "skip" for r in results)
    excused = [r.key for r in results if r.status == "fail"]
    record_pass(root, args.part, args.stage, feature_sets, excused)
    note = f"; EXCUSED by the baseline, still failing: {', '.join(excused)}" if excused else ""
    print(f"gate: PASS {args.part} ({ok} of {ran} checks passed, stage {args.stage}{note})", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
