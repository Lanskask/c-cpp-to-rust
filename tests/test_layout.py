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
