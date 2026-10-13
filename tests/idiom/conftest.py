"""Shared lab for the idiom-table tests: compile a C snippet and a Rust snippet, run both, compare stdout.

Every test in `tests/idiom/` verifies one row of `skill/c-cpp-to-rust/references/idiom-table-c.md`: the row's
Stage 1 or Stage 2 mapping must print the same bytes as the C it replaces, and the row's `trap` must be shown
(a naive mapping prints something else, or panics). Test names are `test_<ROWID>_<slug>`; the docstring starts
with the row id and quotes the trap. Skipped when `cc` or `rustc` is missing.
"""

from __future__ import annotations

import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

import pytest

CC = shutil.which("cc")
RUSTC = shutil.which("rustc")
TIMEOUT = 60


@dataclass(frozen=True)
class Run:
    code: int
    out: str
    err: str


class Lab:
    def __init__(self, root: Path) -> None:
        self.root = root
        self.n = 0

    def _path(self, suffix: str) -> Path:
        self.n += 1
        return self.root / f"snippet{self.n}{suffix}"

    def _run(self, exe: Path) -> Run:
        r = subprocess.run([str(exe)], capture_output=True, text=True, timeout=TIMEOUT)
        return Run(r.returncode, r.stdout, r.stderr)

    def c(self, source: str, flags: tuple[str, ...] = ("-std=c11",)) -> Run:
        """Compile and run a C program (needs `int main`). `-fwrapv` is NOT passed: signed overflow stays UB."""
        src = self._path(".c")
        src.write_text(source)
        exe = src.with_suffix("")
        r = subprocess.run([CC or "cc", *flags, "-o", str(exe), str(src), "-lm"], capture_output=True, text=True)
        assert r.returncode == 0, f"C compile failed:\n{r.stderr}"
        return self._run(exe)

    def rust(self, source: str, flags: tuple[str, ...] = ()) -> Run:
        """Compile and run a Rust program (needs `fn main`), debug profile: arithmetic overflow panics."""
        src = self._path(".rs")
        src.write_text(source)
        exe = src.with_suffix("")
        r = subprocess.run(
            [RUSTC or "rustc", "--edition", "2021", "-o", str(exe), str(src), *flags], capture_output=True, text=True
        )
        assert r.returncode == 0, f"Rust compile failed:\n{r.stderr}"
        return self._run(exe)

    def same(self, c_source: str, rust_source: str) -> str:
        """Both programs exit 0 and print the same stdout; returns that stdout."""
        c, r = self.c(c_source), self.rust(rust_source)
        assert c.code == 0, f"C exited {c.code}: {c.err}"
        assert r.code == 0, f"Rust exited {r.code}: {r.err}"
        assert c.out == r.out, f"C printed {c.out!r}, Rust printed {r.out!r}"
        return c.out

    def differs(self, c_source: str, naive_rust: str) -> None:
        """The naive mapping (the row's trap) must NOT reproduce C: different stdout or a panic."""
        c, r = self.c(c_source), self.rust(naive_rust)
        assert c.code == 0, f"C exited {c.code}: {c.err}"
        assert r.code != 0 or r.out != c.out, f"naive mapping unexpectedly matched C: {c.out!r}"


@pytest.fixture
def lab(tmp_path: Path) -> Lab:
    if CC is None or RUSTC is None:
        pytest.skip("cc and rustc are required")
    return Lab(tmp_path)
