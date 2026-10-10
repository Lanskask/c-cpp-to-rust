"""jsmn harness/run.py: --crate selects the crate under test, --stage 2 needs examples/dump.rs."""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

import pytest

RUN = Path(__file__).resolve().parent.parent / "pilots" / "jsmn" / "migration" / "harness" / "run.py"


@pytest.mark.skipif(shutil.which("cargo") is None, reason="cargo not installed")
def test_stage2_uses_given_crate_and_fails_without_dump_example(tmp_path: Path) -> None:
    crate = tmp_path / "scratch"
    (crate / "src").mkdir(parents=True)
    (crate / "Cargo.toml").write_text('[package]\nname="s"\nversion="0.0.0"\nedition="2021"\n[workspace]\n')
    (crate / "src" / "lib.rs").write_text("")
    r = subprocess.run(
        [sys.executable, "-I", str(RUN), "--crate", str(crate), "--stage", "2"], capture_output=True, text=True
    )
    assert r.returncode == 1
    assert "translated tests [default]" in r.stdout  # reached stage 2 with the scratch crate
    assert "dump" in r.stderr  # the missing example is what failed
