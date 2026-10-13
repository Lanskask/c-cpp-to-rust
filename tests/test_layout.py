"""layout.py and the new-workspace flow: everything under <ws>/migration, legacy workspaces unchanged (decision 72)."""

from __future__ import annotations

from pathlib import Path

import guard
import layout
import pytest
import tickets
import tier


def test_legacy_is_the_default_without_layout_json(tmp_path: Path) -> None:
    lay = layout.load(tmp_path)
    assert (lay.kind, lay.source_rel, lay.crate_rel) == ("legacy", "src", "rust")


def test_init_in_a_bare_project_folder_creates_only_migration(tmp_path: Path) -> None:
    (tmp_path / "lib.cpp").write_text("int f() { return 1; }\n")
    assert tickets.main(["--ws", str(tmp_path), "init", "--lang", "cpp"]) == 0
    assert sorted(p.name for p in tmp_path.iterdir()) == ["lib.cpp", "migration"]
    lay = layout.load(tmp_path)
    assert (lay.kind, lay.source, lay.crate) == ("inplace", tmp_path.resolve(), tmp_path.resolve() / "migration/rust")
    mk = (tmp_path / "migration" / "Makefile").read_text()
    assert f"WORK_DIR  ?= {tmp_path.resolve()}" in mk and "@" not in mk.split("\n")[2]
    ticket = next((tmp_path / "migration" / "tickets").glob("01-*.md")).read_text()
    assert "<ws>" not in ticket and str(tmp_path.resolve()) in ticket


def test_init_keeps_legacy_when_src_exists(tmp_path: Path) -> None:
    (tmp_path / "src").mkdir()
    assert tickets.main(["--ws", str(tmp_path), "init"]) == 0
    assert layout.load(tmp_path).kind == "legacy"


def test_tier_uses_the_crate_of_the_layout() -> None:
    assert tier.target_paths(["a"], None, "migration/rust") == {"a": "migration/rust/src/lib.rs"}
    assert tier.target_paths(["a", "b"], "a", "migration/rust")["b"] == "migration/rust/src/b.rs"


@pytest.mark.parametrize(
    "rel", ["migration/RULEBOOK.md", "migration/parts.tsv", "migration/harness/run.py", "migration/rust/Cargo.toml"]
)
def test_guard_protects_the_judge_of_a_workspace_at_the_project_root(tmp_path: Path, rel: str) -> None:
    tdir = tmp_path / "migration" / "tickets"
    tdir.mkdir(parents=True)
    (tdir / "04-x.md").write_text("**Status:** in_progress\n")
    assert guard.refusal(tmp_path, rel) is not None
    (tdir / "04-x.md").write_text("**Status:** todo\n")
    assert guard.refusal(tmp_path, rel) is None


def git_log(repo: Path) -> list[str]:
    import subprocess

    out = subprocess.run(["git", "-C", str(repo), "log", "--format=%s"], capture_output=True, text=True).stdout
    return out.splitlines()


def test_migration_is_its_own_repository_and_ticket_marks_are_committed(tmp_path: Path) -> None:
    import subprocess

    (tmp_path / "lib.cpp").write_text("int f() { return 1; }\n")
    for k, v in (("user.name", "t"), ("user.email", "t@t")):
        subprocess.run(["git", "config", "--global", k, v], capture_output=True) if False else None
    env = {"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t", "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@t"}
    import os

    os.environ.update(env)
    assert tickets.main(["--ws", str(tmp_path), "init", "--lang", "cpp"]) == 0
    mig = tmp_path / "migration"
    assert (mig / ".git").is_dir() and not (tmp_path / ".git").exists()
    assert git_log(mig) == ["migration: tickets, Makefile, layout"]
    assert tickets.main(["--ws", str(tmp_path), "mark", "01", "in_progress"]) == 0
    assert git_log(mig)[0] == "ticket 01 -> in_progress"
    assert layout.git_dir(tmp_path) == mig
    assert layout.in_repo(tmp_path, "migration/rust/src/lib.rs") == "rust/src/lib.rs"


def test_test_driver_names_are_not_parts() -> None:
    assert tier.is_test_stem("xmltest") and tier.is_test_stem("test_main") and tier.is_test_stem("unit_tests")
    assert not tier.is_test_stem("tinyxml2") and not tier.is_test_stem("latest")


def test_guard_keeps_subagents_inside_migration(tmp_path: Path) -> None:
    (tmp_path / "migration").mkdir()
    (tmp_path / "migration" / "layout.json").write_text('{"kind": "inplace", "source": ".", "crate": "migration/rust"}')
    assert guard.refusal(tmp_path, "tinyxml2.cpp") is not None  # the original: read-only
    assert guard.refusal(tmp_path, "migration/rust/src/lib.rs") is None
    assert guard.refusal(tmp_path, "/tmp/scratch.rs") is None
    assert guard.check_call(tmp_path, "Bash", {"command": "cargo test > out.log"}) is None
    assert guard.check_call(tmp_path, "Bash", {"command": "sed -i s/a/b/ tinyxml2.cpp"}) is not None
