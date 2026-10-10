"""gate.py: error grouping, unsafe count, baseline-aware regression detection, path-scoped rollback."""

from __future__ import annotations

import subprocess
from pathlib import Path

import gate

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
