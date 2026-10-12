"""gate.py: error grouping, unsafe count, baseline-aware regression detection, path-scoped rollback."""

from __future__ import annotations

import subprocess
from pathlib import Path

import gate
import pytest

HEADER = "part_id\tkind\tsource_paths\ttarget_path\ttier\tdeps\tstate\tattempts\tmodel\tlast_error\tcommit\tupdated\n"


def test_summarize_groups_by_error_code() -> None:
    out = (
        "error[E0308]: mismatched types\n --> x\nerror[E0308]: mismatched types\n"
        "error[E0425]: cannot find\nerror: aborting due to 3 errors\n"
    )
    s = gate.summarize_rustc(out)
    assert s.startswith("E0308 x2, E0425 x1") and "first: mismatched types" in s


def test_count_unsafe_ignores_comments(tmp_path: Path) -> None:
    (tmp_path / "src").mkdir()
    (tmp_path / "src" / "lib.rs").write_text("// unsafe in comment\nunsafe fn a() {}\nfn b() { unsafe { } }\n")
    assert gate.count_unsafe(tmp_path) == 2


def make_workspace(tmp_path: Path) -> tuple[Path, Path]:
    def git(*a: str) -> None:
        subprocess.run(
            ["git", "-C", str(tmp_path), "-c", "user.name=t", "-c", "user.email=t@t", *a],
            check=True,
            capture_output=True,
        )

    git("init", "-q")
    crate = tmp_path / "rust"
    (crate / "src").mkdir(parents=True)
    (crate / "Cargo.toml").write_text('[package]\nname="t"\nversion="0.0.0"\nedition="2021"\n[workspace]\n')
    (crate / "src" / "lib.rs").write_text("pub fn f() -> i32 { 1 }\n")
    (tmp_path / "migration").mkdir()
    (tmp_path / "migration" / "parts.tsv").write_text(
        HEADER + "p\tfile\ta.c\trust/src/lib.rs\tmechanical\t\ttodo\t0\t\t\t\t\n"
    )
    git("add", ".")
    git("commit", "-q", "-m", "base")
    return crate, tmp_path / "migration" / "parts.tsv"


def test_regression_rolls_back_and_good_change_passes(tmp_path: Path) -> None:
    crate, parts = make_workspace(tmp_path)
    args = ["p", "--crate", str(crate), "--stage", "1", "--parts", str(parts)]
    assert gate.main([*args, "--snapshot"]) == 0
    (crate / "src" / "lib.rs").write_text('pub fn f() -> i32 { "no" }\n')
    assert gate.main(args) == 1
    assert (crate / "src" / "lib.rs").read_text() == "pub fn f() -> i32 { 1 }\n"  # restored
    (crate / "src" / "lib.rs").write_text("pub fn f() -> i32 { 2 }\n")
    assert gate.main(args) == 0
    assert "2" in (crate / "src" / "lib.rs").read_text()


def test_preexisting_failure_is_not_a_regression(tmp_path: Path) -> None:
    crate, parts = make_workspace(tmp_path)
    (crate / "src" / "lib.rs").write_text('pub fn f() -> i32 { "no" }\n')  # broken before the Part is touched
    args = ["p", "--crate", str(crate), "--stage", "1", "--parts", str(parts)]
    assert gate.main([*args, "--snapshot"]) == 0
    assert gate.main(args) == 0  # same failures as baseline: no regression


def test_also_paths_are_restored_with_the_target(tmp_path: Path) -> None:
    crate, parts = make_workspace(tmp_path)
    adapter = crate / "tests" / "adapter"
    adapter.mkdir(parents=True)
    (adapter / "mod.rs").write_text("// adapter v1\n")
    subprocess.run(["git", "-C", str(tmp_path), "add", "."], check=True)
    subprocess.run(
        ["git", "-C", str(tmp_path), "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "adapter"],
        check=True,
    )
    args = ["p", "--crate", str(crate), "--stage", "1", "--parts", str(parts), "--also", "rust/tests/adapter"]
    assert gate.main([*args, "--snapshot"]) == 0
    (adapter / "mod.rs").write_text("// adapter edited by the failed attempt\n")
    (adapter / "extra.rs").write_text("// stray new file\n")
    (crate / "src" / "lib.rs").write_text('pub fn f() -> i32 { "no" }\n')
    assert gate.main(args) == 1
    assert (adapter / "mod.rs").read_text() == "// adapter v1\n"
    assert not (adapter / "extra.rs").exists()
    assert (crate / "src" / "lib.rs").read_text() == "pub fn f() -> i32 { 1 }\n"


def test_snapshots_are_per_stage(tmp_path: Path) -> None:
    crate, parts = make_workspace(tmp_path)
    base = ["p", "--crate", str(crate), "--parts", str(parts), "--snapshot"]
    assert gate.main([*base, "--stage", "1"]) == 0
    assert gate.main([*base, "--stage", "2"]) == 0
    names = sorted(f.name for f in (tmp_path / "migration" / "gate").iterdir())
    assert names == ["p.stage1.json", "p.stage2.json"]


def test_find_unwraps_requires_invariant_comment(tmp_path: Path) -> None:
    (tmp_path / "src").mkdir()
    (tmp_path / "src" / "lib.rs").write_text(
        "fn a(x: Option<u8>) -> u8 { x.unwrap() }\n"
        "// invariant: x is Some\nfn b(x: Option<u8>) -> u8 { x.unwrap() }\n"
        'fn c(x: Option<u8>) -> u8 { x.expect("y") } // invariant: checked above\n'
        "// x.unwrap() in a comment\n"
    )
    assert gate.find_unwraps(tmp_path) == ["lib.rs:1"]


def test_find_blanket_allows_respects_permitted_list(tmp_path: Path) -> None:
    (tmp_path / "src").mkdir()
    (tmp_path / "src" / "lib.rs").write_text(
        "#![allow(dead_code)]\n#[allow(clippy::too_many_arguments, unused)]\nfn f() {}\n"
    )
    assert gate.find_blanket_allows(tmp_path, []) == [
        "lib.rs:1 dead_code",
        "lib.rs:2 clippy::too_many_arguments",
        "lib.rs:2 unused",
    ]
    assert gate.find_blanket_allows(tmp_path, ["dead_code", "unused"]) == ["lib.rs:2 clippy::too_many_arguments"]


def test_stage2_unwrap_check_fails_and_is_baselinable(tmp_path: Path) -> None:
    crate, parts = make_workspace(tmp_path)
    (crate / "src" / "lib.rs").write_text("pub fn f(x: Option<i32>) -> i32 {\n    x.unwrap()\n}\n")
    out = gate.run_checks(crate, 2, [""], None, None, set(), False)
    unwrap = next(r for r in out if r.check == "unwrap")
    assert unwrap.status == "fail" and "lib.rs:2" in unwrap.detail
    assert unwrap.key == "unwrap[]"  # no feature suffix: baselined by that key like any other check


def test_count_passed_sums_every_test_binary() -> None:
    out = "test result: ok. 3 passed; 0 failed\ntest result: ok. 16 passed; 0 failed\ntest result: ok. 0 passed\n"
    assert gate.count_passed(out) == 19


def test_min_tests_rejects_vacuous_test_pass(tmp_path: Path) -> None:
    crate, _ = make_workspace(tmp_path)
    (crate / "src" / "lib.rs").write_text("#[test]\nfn one() {}\n")
    ok = gate.run_checks(crate, 1, [""], None, None, set(), False, None, 1)
    assert next(r for r in ok if r.check == "test").status == "pass"
    bad = gate.run_checks(crate, 1, [""], None, None, set(), False, None, 2)
    t = next(r for r in bad if r.check == "test")
    assert t.status == "fail" and "1 test(s) passed, need at least 2" in t.detail
    (crate / "src" / "lib.rs").write_text("pub fn f() {}\n")  # no tests at all: cargo passes, the gate must not
    none = gate.run_checks(crate, 1, [""], None, None, set(), False, None, 1)
    assert next(r for r in none if r.check == "test").status == "fail"


def test_stage2_judge_checks_are_never_baselined(tmp_path: Path) -> None:
    crate, parts = make_workspace(tmp_path)
    for stage, expected in (("1", 0), ("2", 1)):
        args = ["p", "--crate", str(crate), "--stage", stage, "--parts", str(parts), "--harness-cmd", "false"]
        assert gate.main([*args, "--snapshot", "--no-rollback"]) == 0
        # `false` as harness fails in the baseline; Stage 1 excuses it, Stage 2 must not
        assert gate.main([*args, "--no-rollback"]) == expected, stage
    assert not gate.Result("check", "", "fail").regresses({"check[]"}, 2)
    assert gate.Result("harness", "", "fail").regresses({"harness[]"}, 2)
    assert not gate.Result("harness", "", "fail").regresses({"harness[]"}, 1)


def test_snapshot_appends_one_attempt_row_each_time(tmp_path: Path) -> None:
    crate, parts = make_workspace(tmp_path)
    args = ["p", "--crate", str(crate), "--stage", "1", "--parts", str(parts), "--snapshot"]
    assert gate.main(args) == 0
    assert gate.main(args) == 0
    rows = (tmp_path / "migration" / "attempts.tsv").read_text().splitlines()
    assert rows[0] == "part\tstage\thead\ttime" and len(rows) == 3
    assert rows[1].split("\t")[:2] == ["p", "1"]


def test_pass_message_lists_excused_checks_and_strict_judge_refuses(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    crate, parts = make_workspace(tmp_path)
    args = ["p", "--crate", str(crate), "--stage", "1", "--parts", str(parts), "--harness-cmd", "false"]
    assert gate.main([*args, "--snapshot"]) == 0
    capsys.readouterr()
    assert gate.main(args) == 0
    err = capsys.readouterr().err
    assert "2 of 3 checks passed" in err and "EXCUSED" in err and "harness[]" in err
    assert gate.main([*args, "--strict-judge", "--no-rollback"]) == 1


def test_snapshot_creates_nested_gate_dir_for_slash_part_ids(tmp_path: Path) -> None:
    crate, parts = make_workspace(tmp_path)
    parts.write_text(HEADER + "src/p\tfile\ta.c\trust/src/lib.rs\tmechanical\t\ttodo\t0\t\t\t\t\n")
    assert gate.main(["src/p", "--crate", str(crate), "--stage", "1", "--parts", str(parts), "--snapshot"]) == 0
    assert (tmp_path / "migration" / "gate" / "src" / "p.stage1.json").is_file()


def test_rollback_leaves_untracked_also_paths_alone(tmp_path: Path) -> None:
    crate, parts = make_workspace(tmp_path)
    judge = crate / "tests" / "judge.rs"
    judge.parent.mkdir()
    judge.write_text("// untracked Judge file\n")
    args = ["p", "--crate", str(crate), "--stage", "1", "--parts", str(parts), "--also", "rust/tests/judge.rs"]
    assert gate.main([*args, "--snapshot"]) == 0
    (crate / "src" / "lib.rs").write_text('pub fn f() -> i32 { "no" }\n')
    assert gate.main(args) == 1
    assert judge.read_text() == "// untracked Judge file\n"
    assert (crate / "src" / "lib.rs").read_text() == "pub fn f() -> i32 { 1 }\n"
