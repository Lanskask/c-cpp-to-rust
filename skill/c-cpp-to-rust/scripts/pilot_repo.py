#!/usr/bin/env python3
"""Give a Pilot its own git repository, with the upstream C source as a submodule at `src/` (decision 69).

    python3 -I skill/c-cpp-to-rust/scripts/pilot_repo.py new NAME [--remote URL]    # src/ is a clone
    python3 -I skill/c-cpp-to-rust/scripts/pilot_repo.py split NAME [--remote URL]  # pilots/NAME tracked in main

`new` runs `git init` in pilots/NAME (if it is not a repository yet), registers `src/` as a submodule pinned to the
`commit=` line of UPSTREAM (url from the same file) and commits the Pilot as it is. `split` first rewrites the history
of `pilots/NAME` in the main repo into the new repository (`git subtree split`, nothing is lost), then does what `new`
does. With `--remote URL` the repository gets `origin` and the main repo records pilots/NAME as a submodule of that
URL; without it the main repo is left alone and the command to run afterwards is printed. Nothing is pushed.
Run `split` with a clean main working tree: it deletes the tracked pilot files from the main index (one commit).
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

PILOT_IGNORE = """# build output and disposable Judge state (the upstream clone is the submodule `src/`)
target/
Cargo.lock
__pycache__/
*.pyc
migration/gate/
migration/stress/
migration/harness/work/
migration/harness/inputs/
"""


def git(repo: Path, *args: str, check: bool = True) -> str:
    r = subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True)
    if check and r.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} (in {repo}): {r.stderr.strip()}")
    return r.stdout.strip()


def upstream(ws: Path) -> dict[str, str]:
    f = ws / "UPSTREAM"
    if not f.is_file():
        raise RuntimeError(f"{f} missing: it names the upstream url and commit")
    pairs = (ln.split("=", 1) for ln in f.read_text().splitlines() if "=" in ln)
    out = {k.strip(): v.strip() for k, v in pairs}
    if "url" not in out or "commit" not in out:
        raise RuntimeError(f"{f} needs url= and commit= lines")
    return out


def init_repo(ws: Path) -> None:
    """git init + src submodule at the UPSTREAM commit + first commit; idempotent."""
    up = upstream(ws)
    if not (ws / ".git").exists():
        git(ws, "init", "-q", "-b", "main")
    if not (ws / ".gitignore").is_file():
        (ws / ".gitignore").write_text(PILOT_IGNORE)
    src = ws / "src"
    if not (src / ".git").exists():
        raise RuntimeError(f"{src} is not a clone: `git clone {up['url']} {src}` first")
    git(src, "checkout", "-q", up["commit"])
    if "src" not in git(ws, "submodule", "status", check=False):
        # the path is ignored by older main-repo patterns; -f registers it anyway
        git(ws, "submodule", "add", "-f", up["url"], "src")
    git(ws, "add", "-A")
    if git(ws, "status", "--porcelain"):
        git(ws, "-c", "user.name=pilot_repo", "-c", "user.email=pilot_repo@local", "commit", "-qm", "pilot repository")


def split(root: Path, name: str) -> None:
    """Replace pilots/NAME tracked in the main repo by its own history-preserving repository, in place."""
    ws = root / "pilots" / name
    prefix = f"pilots/{name}"
    if git(root, "status", "--porcelain", "--", prefix):
        raise RuntimeError(f"{prefix} has uncommitted changes in the main repo: commit them first")
    if (ws / ".git").exists():
        raise RuntimeError(f"{ws} is already a repository")
    sha = git(root, "subtree", "split", f"--prefix={prefix}")
    git(ws, "init", "-q", "-b", "main")
    git(ws, "fetch", "-q", str(root), sha)
    git(ws, "reset", "-q", sha)  # mixed: index follows the history, the working tree stays as it is
    git(root, "rm", "-r", "-q", "--cached", prefix)


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("cmd", choices=("new", "split"))
    p.add_argument("name")
    p.add_argument("--remote", help="URL of the Pilot's repository: set as origin and registered in the main repo")
    p.add_argument("--root", default=".", help="main repo root (default: cwd)")
    a = p.parse_args(argv)
    root = Path(a.root).resolve()
    ws = root / "pilots" / a.name
    try:
        if a.cmd == "split":
            split(root, a.name)
        init_repo(ws)
        if a.remote:
            if "origin" not in git(ws, "remote"):
                git(ws, "remote", "add", "origin", a.remote)
            git(root, "submodule", "add", "-f", a.remote, f"pilots/{a.name}")
            print(f"pilot_repo: pilots/{a.name} registered in the main repo; commit .gitmodules and the gitlink")
        else:
            print(
                f"pilot_repo: pilots/{a.name} is its own repository (no remote). Create the remote, then:\n"
                f"  git -C pilots/{a.name} remote add origin URL && git -C pilots/{a.name} push -u origin main\n"
                f"  python3 -I skill/c-cpp-to-rust/scripts/pilot_repo.py new {a.name} --remote URL"
            )
    except RuntimeError as e:
        print(f"pilot_repo: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
