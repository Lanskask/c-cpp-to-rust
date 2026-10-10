#!/usr/bin/env python3
"""E1: does c2rust 0.22.1 turn header-only jsmn into a Rust crate that compiles and passes jsmn's own tests?

Question: c2rust needs a .c file and a compile DB; jsmn is one header (`jsmn.h`) whose behaviour
depends on two macros (JSMN_STRICT, JSMN_PARENT_LINKS; upstream tests all four combinations).
For each combination we translate upstream `test/tests.c` (which includes jsmn.h) as a binary,
build it with cargo, and run it. The library alone is also translated from a one-line wrapper `jsmn.c`.

Usage: python3 -I experiments/e1_c2rust_jsmn.py   (from the repo root; writes pilots/jsmn/c2rust/)

Answer (run 2026-10-10, c2rust 0.22.1, LLVM 20.1.8, jsmn 25647e6): **yes**.
- Library (`jsmn.c` wrapper): 361 Rust lines, 6 `unsafe extern "C" fn`; builds on stable 1.94.
- `tests.c` in all 4 configs: transpiles, builds, and prints `PASSED: 16 FAILED: 0`, identical to the
  original C tests compiled with clang. Takes ~9 s for all five translations.
- Needed three workarounds, all now in this script:
  1. the compile DB must be named exactly `compile_commands.json` (any other name: "Running without
     flags", so `-I` is lost);
  2. the output dir name becomes the package name; `crate` panics (`"r#crate" is not a valid Ident`);
  3. the emitted `rust-toolchain.toml` pins `nightly-2023-04-15`, which rustup only honours from the
     cwd; with `--manifest-path` pass `cargo +<pin>`. The tests need nightly for `#![feature(c_variadic)]`
     (testutil.h has variadic helpers).
- Caveat: c2rust works after the preprocessor, so each macro config is a separate translation. And the
  translated test binary contains its own copy of jsmn (tests.c includes jsmn.h); it does not yet test the
  library crate. Rewiring it to call the crate is E2 work.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "pilots" / "jsmn" / "src"
OUT = ROOT / "pilots" / "jsmn" / "c2rust"
C2RUST = Path.home() / ".cargo" / "bin" / "c2rust"
CONFIGS: dict[str, list[str]] = {
    "default": [],
    "strict": ["-DJSMN_STRICT=1"],
    "links": ["-DJSMN_PARENT_LINKS=1"],
    "strict_links": ["-DJSMN_STRICT=1", "-DJSMN_PARENT_LINKS=1"],
}


def run(cmd: list[str], log: Path) -> int:
    r = subprocess.run(cmd, capture_output=True, text=True)
    log.write_text(f"$ {' '.join(cmd)}\nexit {r.returncode}\n--- stdout\n{r.stdout}\n--- stderr\n{r.stderr}")
    return r.returncode


def compile_db(work: Path, source: Path, defines: list[str]) -> Path:
    db = work / "compile_commands.json"  # c2rust only accepts this exact file name
    entry = {
        "directory": str(work),
        "file": str(source),
        "arguments": ["clang", "-c", "-I", str(SRC), *defines, str(source), "-o", f"{source.stem}.o"],
    }
    db.write_text(json.dumps([entry], indent=2))
    return db


def translate(name: str, source: Path, defines: list[str], binary: str | None) -> dict[str, object]:
    work = OUT / name / source.stem
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir(parents=True)
    db = compile_db(work, source, defines)
    crate = work / f"{source.stem}_c2rust"  # dir name becomes the package name; "crate" is a keyword
    cmd = [str(C2RUST), "transpile", "-e", "-o", str(crate), str(db)]
    if binary:
        cmd[2:2] = ["-b", binary]
    res: dict[str, object] = {"config": name, "source": source.name}
    res["transpile"] = run(cmd, work / "transpile.log")
    if res["transpile"] != 0:
        return res
    manifest = str(crate / "Cargo.toml")
    # rustup reads rust-toolchain.toml from the cwd, not from --manifest-path, so pass the pin explicitly.
    pin = crate / "rust-toolchain.toml"
    cargo = ["cargo"]
    if pin.exists():
        res["toolchain"] = tomllib.loads(pin.read_text())["toolchain"]["channel"]
        cargo.append(f"+{res['toolchain']}")
    res["build"] = run([*cargo, "build", "-q", "--manifest-path", manifest], work / "build.log")
    if binary and res["build"] == 0:
        res["run"] = run([*cargo, "run", "-q", "--manifest-path", manifest, "--bin", binary], work / "run.log")
    rs = list(crate.rglob("*.rs"))
    res["rust_lines"] = sum(len(p.read_text().splitlines()) for p in rs)
    res["unsafe_blocks_or_fns"] = sum(p.read_text().count("unsafe ") for p in rs)
    return res


def main() -> int:
    if not C2RUST.exists() or not SRC.is_dir():
        print("e1: need c2rust and pilots/jsmn/src", file=sys.stderr)
        return 1
    OUT.mkdir(parents=True, exist_ok=True)
    wrapper = OUT / "jsmn.c"
    wrapper.write_text('#include "jsmn.h"\n')
    results = [translate("default", wrapper, [], None)]
    results += [translate(n, SRC / "test" / "tests.c", d, "tests") for n, d in CONFIGS.items()]
    for r in results:
        print(json.dumps(r))
    ok = all(r.get("build") == 0 and r.get("run", 0) == 0 for r in results)
    print(f"e1: {'all configs build and pass' if ok else 'failures, see logs in pilots/jsmn/c2rust'}", file=sys.stderr)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
