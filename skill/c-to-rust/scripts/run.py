#!/usr/bin/env python3
"""Headless driver for one stage of a Pilot: queue -> snapshot -> implementer -> gate -> review -> commit -> mark.

    python3 -I skill/c-to-rust/scripts/run.py pilots/<name> --stage 1 [--max-attempts 5] [--budget-output-tokens N]
                                                               [--go] [--model sonnet] [--reviewer-model sonnet]

Mechanical steps call the Skill's scripts (workqueue.py, gate.py via the Pilot's `make pilot-<name>` targets,
spend.py). The model steps are `claude -p` runs (implementer, reviewer). **Without `--go` nothing is spent**: the
driver checks the preconditions, prints the commands it would run and the budget, and exits 0.

The driver stops (exit 3, state stays in parts.tsv and tickets, rerun the same command to resume) when:
a precondition is missing (tickets before the fan-out ticket not `done`, rulebook not approved, a §8 row still
`proposed`), a Part is `stubbed` after the attempt limit, the output-token budget passes 70%, a review reports a
GAP or RULE the governor must apply, or `workqueue.py mark done` refuses (excused checks). It never edits the
Rulebook or the Judge. Exit 0: every Part of the stage is `done`. Exit 1: a script failed unexpectedly.
Decision 66: an implementer run through `claude -p` is not seen by guard.py (no `agent_id`); the hash check of
gate.py (decision 63) is what catches a changed Judge file.
"""

from __future__ import annotations

import argparse
import json
import re
import shlex
import subprocess
import sys
import time
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path

SKILL = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

import gate  # noqa: E402
import workqueue as wq  # noqa: E402

STOP = 3
RULEBOOK_SECTIONS = {1: (0, 2, 3, 4, 5, 7, 8, 9, 11), 2: (1, 2, 4, 6, 7, 8, 9, 10, 11)}
FANOUT_TICKET = {1: "04", 2: "06"}
IMPLEMENTER_TOOLS = "Read,Edit,Write,Grep,Glob,Bash(cargo check:*)"
REVIEWER_TOOLS = "Read,Grep,Glob,Bash(cargo check:*),Bash(cargo test:*),Bash(git diff:*)"


@dataclass
class Config:
    ws: Path  # pilots/<name>
    root: Path  # repo root (workspace root of the gate: parent of migration/ is `ws`)
    stage: int
    max_attempts: int
    budget: int
    go: bool
    model: str
    reviewer_model: str
    claude: str
    settings: Path
    max_turns: int
    gate_cmd: str
    snapshot_cmd: str
    deviations: str = "ask"  # ask | policy

    @property
    def name(self) -> str:
        return self.ws.name

    @property
    def parts(self) -> Path:
        return self.ws / "migration" / ("parts-stage2.tsv" if self.stage == 2 else "parts.tsv")


# ---------------------------------------------------------------- preconditions


def ticket_status(ws: Path) -> dict[str, str]:
    out: dict[str, str] = {}
    for f in sorted((ws / "migration" / "tickets").glob("[0-9][0-9]-*.md")):
        m = re.search(r"^\*\*Status:\*\* (\w+)", f.read_text(), re.M)
        out[f.name[:2]] = m.group(1) if m else "?"
    return out


def preconditions(cfg: Config) -> list[str]:
    problems: list[str] = []
    fan = FANOUT_TICKET[cfg.stage]
    tickets = ticket_status(cfg.ws)
    if not tickets:
        problems.append(f"{cfg.ws}/migration/tickets/ has no tickets (`tickets.py init`)")
    for num, status in tickets.items():
        if num < fan and status != "done":
            problems.append(f"ticket {num} is {status}, must be done before the fan-out ticket {fan}")
    if tickets.get(fan) == "done":
        problems.append(f"ticket {fan} (fan-out) is already done")
    rulebook = cfg.ws / "migration" / "RULEBOOK.md"
    text = rulebook.read_text() if rulebook.is_file() else ""
    status_line = re.search(r"^Status: (.*)$", text, re.M)
    if not status_line or not re.match(r"(deviations-approved|stress-tested)", status_line.group(1)):
        problems.append(f"RULEBOOK.md status is {status_line.group(1) if status_line else 'missing'}")
    for row_id, status in re.findall(r"^\| (D\d+) \|.*\| ([^|]+?) \|\s*$", text, re.M):
        if status.startswith("proposed"):
            problems.append(f"deviation {row_id} is still `proposed`: the user decides it (SKILL.md step 1)")
        elif status.startswith("approved (policy)") and cfg.deviations == "ask":
            problems.append(
                f"deviation {row_id} is `approved (policy)`: approve it (status `approved`) or start with "
                "--deviations policy"
            )
    if not cfg.parts.is_file():
        problems.append(f"{cfg.parts} missing")
    return problems


# ---------------------------------------------------------------- prompt slots


def prompt_template(path: Path) -> str:
    m = re.search(r"```text\n(.*?)\n```", path.read_text(), re.S)
    if not m:
        raise ValueError(f"{path}: no ```text block")
    return m.group(1)


def rulebook_sections(text: str, wanted: tuple[int, ...]) -> str:
    chunks = re.split(r"^(?=## \d+\.)", text, flags=re.M)
    keep = [chunks[0]]
    for c in chunks[1:]:
        m = re.match(r"## (\d+)\.", c)
        if m and int(m.group(1)) in wanted:
            keep.append(c)
    return "".join(keep).strip()


def numbered(path: Path) -> str:
    return "\n".join(f"{n:5d}  {ln}" for n, ln in enumerate(path.read_text(errors="replace").splitlines(), 1))


def rust_items(text: str) -> tuple[list[str], list[str]]:
    """(type items, fn signatures) of a Rust file: type items are whole `struct`/`enum`/`const`/`type` blocks."""
    items: list[str] = []
    sigs: list[str] = []
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        ln = lines[i]
        if re.match(r"\s*(pub(\([^)]*\))? )?(struct|enum|const|static|type|union) ", ln):
            block = [ln]
            depth = ln.count("{") - ln.count("}")
            while (depth > 0 or (not block[-1].rstrip().endswith((";", "}")) and "{" not in block[0])) and i + 1 < len(
                lines
            ):
                i += 1
                block.append(lines[i])
                depth += lines[i].count("{") - lines[i].count("}")
            items.append("\n".join(block))
        elif re.match(r"\s*(pub(\([^)]*\))? )?(unsafe )?(extern \"C\" )?fn ", ln):
            sig = ln.rstrip()
            j = i
            while "{" not in sig and not sig.endswith(";") and j + 1 < len(lines):
                j += 1
                sig += " " + lines[j].strip()
            sigs.append(sig.split("{")[0].strip())
        i += 1
    return items, sigs


def translated_context(cfg: Config, part: dict[str, str], rows: list[wq.Row]) -> tuple[str, str]:
    """Types and callee signatures from the target files of already done Parts (not the Part itself)."""
    types: list[str] = []
    sigs: list[str] = []
    seen: set[Path] = set()
    for r in rows:
        if r["state"] != "done" or r["part_id"] == part["part_id"]:
            continue
        f = cfg.ws / r["target_path"]
        if f in seen or not f.is_file():
            continue
        seen.add(f)
        t, s = rust_items(f.read_text(errors="replace"))
        types += t
        sigs += s
    return "\n\n".join(types) or "none", "\n".join(sigs) or "none"


def tm_hits(cfg: Config, error: str) -> str:
    if not error:
        return "none"
    r = subprocess.run(
        [
            sys.executable,
            "-I",
            str(SKILL / "scripts" / "tm.py"),
            "--db",
            str(cfg.ws / "migration" / "tm.sqlite"),
            "search",
            error,
        ],
        capture_output=True,
        text=True,
    )
    return "\n".join(r.stdout.splitlines()[:3]) if r.returncode == 0 and r.stdout.strip() else "none"


def implementer_prompt(cfg: Config, part: dict[str, str], rows: list[wq.Row], previous: str) -> str:
    ref = "stage2-prompt.md" if cfg.stage == 2 else "stage1-prompt.md"
    rulebook = (cfg.ws / "migration" / "RULEBOOK.md").read_text()
    idiom = (SKILL / "references" / "idiom-table-c.md").read_text()
    if "idiom-table-cpp" in rulebook:
        idiom += "\n\n" + (SKILL / "references" / "idiom-table-cpp.md").read_text()
    structs, sigs = translated_context(cfg, part, rows)
    sources = [s for s in part["source_paths"].split(",") if s]
    source = "\n\n".join(f"// {s}\n{numbered(cfg.ws / s)}" for s in sources)
    faithful = ""
    if cfg.stage == 2:
        t = cfg.ws / part["target_path"]
        faithful = t.read_text() if t.is_file() else ""
    adapter = cfg.ws / "rust" / "tests" / "adapter" / "mod.rs"
    slots = {
        "part_id": part["part_id"],
        "tier": part["tier"],
        "target_path": part["target_path"],
        "source": source,
        "rulebook": rulebook_sections(rulebook, RULEBOOK_SECTIONS[cfg.stage]),
        "idiom_rows": idiom,
        "c2rust": "none",
        "structs": structs,
        "callee_signatures": sigs,
        "previous": previous or "none (attempt 1)",
        "tm_hits": tm_hits(cfg, previous),
        "faithful": faithful or "none",
        "adapter": adapter.read_text() if adapter.is_file() else "none",
        "judge": "see the RULEBOOK §10 and migration/harness/",
    }
    return fill(prompt_template(SKILL / "references" / ref), slots)


def fill(template: str, slots: dict[str, str]) -> str:
    return re.sub(r"\{([a-z_0-9]+)\}", lambda m: slots.get(m.group(1), m.group(0)), template)


def review_prompt(cfg: Config, part: dict[str, str], head: str, gate_output: str) -> str:
    diff = subprocess.run(
        ["git", "-C", str(cfg.root), "diff", head, "--", str(cfg.ws / part["target_path"])],
        capture_output=True,
        text=True,
    ).stdout
    rulebook = (cfg.ws / "migration" / "RULEBOOK.md").read_text()
    sources = [s for s in part["source_paths"].split(",") if s]
    slots = {
        "part_id": part["part_id"],
        "stage": str(cfg.stage),
        "diff": diff,
        "rulebook": rulebook,
        "idiom_rows": (SKILL / "references" / "idiom-table-c.md").read_text(),
        "source": "\n\n".join(f"// {s}\n{numbered(cfg.ws / s)}" for s in sources),
        "judge": "see the RULEBOOK §10 and migration/harness/",
        "gate_output": gate_output,
    }
    return fill(prompt_template(SKILL / "references" / "review-prompt.md"), slots)


# ---------------------------------------------------------------- model and script steps


def claude_cmd(cfg: Config, model: str, tools: str) -> list[str]:
    return [
        cfg.claude,
        "-p",
        "--model",
        model,
        "--settings",
        str(cfg.settings),
        "--permission-mode",
        "acceptEdits",
        "--allowed-tools",
        tools,
        "--max-turns",
        str(cfg.max_turns),
        "--output-format",
        "json",
    ]


def run_claude(cfg: Config, model: str, tools: str, prompt: str) -> tuple[int, str]:
    r = subprocess.run(claude_cmd(cfg, model, tools), input=prompt, capture_output=True, text=True, cwd=cfg.root)
    text = r.stdout
    try:
        data = json.loads(r.stdout)
        text = str(data.get("result", r.stdout)) if isinstance(data, dict) else r.stdout
    except json.JSONDecodeError:
        pass
    return r.returncode, text + (("\n" + r.stderr) if r.returncode else "")


def sh(cfg: Config, template: str, part: str) -> tuple[int, str]:
    cmd = template.format(root=cfg.root, ws=cfg.ws, name=cfg.name, stage=cfg.stage, part=part)
    r = subprocess.run(cmd, shell=True, capture_output=True, text=True, cwd=cfg.root)
    return r.returncode, r.stdout + r.stderr


def workqueue(cfg: Config, *args: str) -> int:
    return wq.main(["--parts", str(cfg.parts), *args])


def parse_review(text: str) -> tuple[list[str], list[str], str]:
    """(BLOCK lines, GAP/RULE lines, verdict) of a review report."""
    blocks = [ln for ln in text.splitlines() if re.match(r"BLOCK\t", ln)]
    others = [ln for ln in text.splitlines() if re.match(r"(GAP|RULE)\t", ln)]
    m = re.findall(r"^VERDICT:\s*(\w+)", text, re.M)
    return blocks, others, m[-1] if m else "missing"


def spend_output(cfg: Config, since: str) -> int:
    r = subprocess.run(
        [sys.executable, "-I", str(SKILL / "scripts" / "spend.py"), "sum", "--since", since],
        capture_output=True,
        text=True,
    )
    try:
        return int(json.loads(r.stdout).get("output", 0))
    except (json.JSONDecodeError, ValueError, AttributeError):
        return 0


def note(cfg: Config, heading: str, lines: list[str]) -> None:
    path = cfg.ws / "migration" / "RUN-NOTES.md"
    with path.open("a") as f:
        f.write(f"\n## {heading}\n" + "\n".join(f"- {ln}" for ln in lines) + "\n")


def git(cfg: Config, *args: str) -> str:
    return subprocess.run(["git", "-C", str(cfg.root), *args], capture_output=True, text=True).stdout.strip()


def log_work(cfg: Config, summary: list[str]) -> None:
    path = cfg.root / "WORK-LOG.md"
    text = path.read_text() if path.is_file() else ""
    nums = [int(n) for n in re.findall(r"^## (\d+)\.", text, re.M)]
    entry = (
        f"\n## {max(nums, default=0) + 1}. run.py {cfg.name} stage {cfg.stage}\n"
        f"- **Date:** {datetime.now(UTC).date()}\n- **Who:** run.py (headless driver)\n"
        + "".join(f"- {ln}\n" for ln in summary)
    )
    path.write_text(text.rstrip("\n") + "\n" + entry)


# ---------------------------------------------------------------- the loop


def retry_candidates(rows: list[wq.Row], cfg: Config) -> list[wq.Row]:
    """A `failed` Part left by an earlier run (workqueue.next_parts skips it) whose dependencies are done."""
    finished = {r["part_id"] for r in rows if r["state"] in ("done", "stubbed")}
    return [
        r
        for r in rows
        if r["state"] == "failed"
        and int(r["attempts"] or "0") < cfg.max_attempts
        and all(d in finished for d in wq.deps_of(r))
    ][:1]


def plan(cfg: Config) -> list[str]:
    return [
        f"implementer: {shlex.join(claude_cmd(cfg, cfg.model, IMPLEMENTER_TOOLS))}  < prompt",
        f"reviewer:    {shlex.join(claude_cmd(cfg, cfg.reviewer_model, REVIEWER_TOOLS))}  < prompt",
        f"snapshot:    {cfg.snapshot_cmd}",
        f"gate:        {cfg.gate_cmd}",
        f"attempts:    at most {cfg.max_attempts} per Part, then the Part is `stubbed` and the driver stops",
        "budget:      "
        + (f"stop at 70% of {cfg.budget} output tokens" if cfg.budget else "none (--budget-output-tokens)"),
    ]


_T0 = time.monotonic()


def say(msg: str) -> None:
    """One progress line, flushed at once: `[mm:ss] msg` (elapsed since the driver started)."""
    m, s = divmod(int(time.monotonic() - _T0), 60)
    print(f"[{m // 60:d}:{m % 60:02d}:{s:02d}] {msg}", flush=True)


def progress(rows: list[wq.Row], pid: str) -> str:
    """`Part 3/12 (25% done)`: position of `pid` among all Parts and the share already done or stubbed."""
    finished = sum(1 for r in rows if r["state"] in ("done", "stubbed"))
    ids = [r["part_id"] for r in rows]
    return f"Part {ids.index(pid) + 1}/{len(rows)} [{finished}/{len(rows)} finished, {100 * finished // len(rows)}%]"


def timed(fn: object, label: str, *args: object) -> tuple[int, str]:
    """Run a step `fn(*args)` -> (code, output) and print its start and its duration."""
    say(f"  {label} ...")
    t = time.monotonic()
    code, out = fn(*args)  # ty: ignore[call-non-callable]
    say(f"  {label}: exit {code} in {time.monotonic() - t:.0f}s")
    return code, out


def run(cfg: Config) -> int:
    problems = preconditions(cfg)
    if problems:
        print("run.py: cannot start:\n  " + "\n  ".join(problems), file=sys.stderr)
        return STOP
    print("run.py plan:\n  " + "\n  ".join(plan(cfg)))
    if not cfg.go:
        print("run.py: nothing spent; rerun with --go to start")
        return 0
    started = datetime.now(UTC).isoformat(timespec="seconds")
    fan = FANOUT_TICKET[cfg.stage]
    subprocess.run(
        [sys.executable, "-I", str(SKILL / "scripts" / "tickets.py"), "--ws", str(cfg.ws), "mark", fan, "in_progress"],
        capture_output=True,
    )
    done: list[str] = []
    notes: list[str] = []
    needs_governor = False
    while True:
        rows = wq.read_parts(cfg.parts)
        ready = wq.next_parts(rows, 1) or retry_candidates(rows, cfg)
        if not ready:
            break
        part = ready[0]
        pid = part["part_id"]
        say(f"{progress(rows, pid)} {pid} -> {part['target_path']} (tier {part['tier']})")
        previous = part["last_error"] if part["state"] == "failed" else ""
        while True:
            rows = wq.read_parts(cfg.parts)
            row = next(r for r in rows if r["part_id"] == pid)
            say(f"{pid}: attempt {int(row['attempts'] or '0') + 1}/{cfg.max_attempts}")
            if int(row["attempts"] or "0") >= cfg.max_attempts:
                workqueue(cfg, "mark", pid, "failed", "--error", previous or "attempt limit")
                return stop(cfg, done, notes, f"{pid} is stubbed after {cfg.max_attempts} attempts: write the stub")
            if cfg.budget and spend_output(cfg, started) >= 0.7 * cfg.budget:
                return stop(cfg, done, notes, f"output-token budget passed 70% of {cfg.budget}")
            workqueue(cfg, "mark", pid, "in_progress")
            code, out = timed(sh, "snapshot", cfg, cfg.snapshot_cmd, pid)
            if code != 0:
                return stop(cfg, done, notes, f"snapshot failed: {out.strip()[-200:]}", 1)
            head = git(cfg, "rev-parse", "HEAD")
            code, report = timed(
                run_claude,
                f"implementer ({cfg.model})",
                cfg,
                cfg.model,
                IMPLEMENTER_TOOLS,
                implementer_prompt(cfg, part, rows, previous),
            )
            if code != 0:
                say(f"  implementer failed: {report.strip()[-200:]}")
                previous = f"implementer run failed (exit {code}): {report.strip()[-300:]}"
                workqueue(cfg, "mark", pid, "failed", "--error", previous)
                continue
            code, gate_out = timed(
                sh, "gate", cfg, cfg.gate_cmd, pid
            )  # `STATUS: done` in `report` is a claim; the gate decides
            say("  gate: " + (gate_out.strip().splitlines() or ["(no output)"])[-1])
            if code != 0:
                previous = "gate: " + " ".join(gate_out.strip().splitlines()[-1:])
                workqueue(cfg, "mark", pid, "failed", "--error", previous)
                continue
            rcode, review = timed(
                run_claude,
                f"review ({cfg.reviewer_model})",
                cfg,
                cfg.reviewer_model,
                REVIEWER_TOOLS,
                review_prompt(cfg, part, head, gate_out),
            )
            blocks, others, verdict = parse_review(review)
            say(f"  review: verdict {verdict}, {len(blocks)} BLOCK, {len(others)} GAP/RULE")
            for ln in blocks:
                say(f"    {ln[:160]}")
            if others:
                needs_governor = True
                notes += [f"{pid}: {ln}" for ln in others]
                note(cfg, f"Review findings {pid} stage {cfg.stage}", others)
            if rcode != 0 or blocks or verdict not in ("accept",):
                gate.rollback(cfg.root, head, [str(cfg.ws / part["target_path"])])
                previous = ("review BLOCK: " + " | ".join(blocks))[:1500] if blocks else f"review verdict {verdict}"
                workqueue(cfg, "mark", pid, "failed", "--error", previous)
                continue
            target = str(cfg.ws / part["target_path"])
            git(cfg, "add", "--", target)
            subprocess.run(
                ["git", "-C", str(cfg.root), "commit", "-q", "-m", f"{cfg.name}: {pid} stage {cfg.stage} (run.py)"],
                capture_output=True,
            )
            if workqueue(cfg, "mark", pid, "done", "--commit", git(cfg, "rev-parse", "HEAD")) != 0:
                return stop(cfg, done, notes, f"{pid}: mark done refused (see above); governor decides")
            done.append(pid)
            say(f"{pid}: ACCEPTED, committed {git(cfg, 'rev-parse', '--short', 'HEAD')}")
            break
        if needs_governor:
            return stop(
                cfg, done, notes, "review reported GAP/RULE entries: apply them (RUN-NOTES.md) before more Parts"
            )
    return stop(cfg, done, notes, "", 0)


def stop(cfg: Config, done: list[str], notes: list[str], reason: str, code: int = STOP) -> int:
    summary = [f"Parts accepted this run: {', '.join(done) or 'none'}", f"review findings kept: {len(notes)}"]
    if reason:
        summary.append(f"STOPPED: {reason}")
    log_work(cfg, summary)
    say("summary")
    print("run.py summary:\n  " + "\n  ".join(summary))
    return code


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("pilot", help="pilots/<name>")
    p.add_argument("--stage", type=int, choices=[1, 2], default=1)
    p.add_argument("--max-attempts", type=int, default=wq.MAX_ATTEMPTS)
    p.add_argument("--budget-output-tokens", type=int, default=0)
    p.add_argument("--go", action="store_true", help="spend tokens; without it the plan is printed and nothing runs")
    p.add_argument(
        "--deviations",
        choices=("ask", "policy"),
        default="ask",
        help="ask: every deviation row must be `approved` by you (default); policy: `approved (policy)` rows pass",
    )
    p.add_argument("--model", default="sonnet")
    p.add_argument("--reviewer-model", default="sonnet")
    p.add_argument("--claude", default="claude", help="the claude executable (tests pass a fake)")
    p.add_argument("--settings", default=str(SKILL / "assets" / "settings.json"))
    p.add_argument("--max-turns", type=int, default=60)
    p.add_argument("--root", help="repo root (default: parent of pilots/)")
    p.add_argument("--gate-cmd", help="template; {root} {ws} {name} {stage} {part} (default: make pilot-<name>)")
    p.add_argument("--snapshot-cmd", help="template, as --gate-cmd (default: make pilot-<name>-snapshot)")
    return p


def main(argv: list[str] | None = None) -> int:
    a = build_parser().parse_args(argv)
    ws = Path(a.pilot).resolve()
    root = Path(a.root).resolve() if a.root else ws.parent.parent
    base = "make -C {root} pilot-{name}"
    cfg = Config(
        ws=ws,
        root=root,
        stage=a.stage,
        max_attempts=a.max_attempts,
        budget=a.budget_output_tokens,
        go=a.go,
        model=a.model,
        reviewer_model=a.reviewer_model,
        claude=a.claude,
        settings=Path(a.settings),
        max_turns=a.max_turns,
        gate_cmd=a.gate_cmd or base + " STAGE={stage} PART={part}",
        snapshot_cmd=a.snapshot_cmd or base + "-snapshot STAGE={stage} PART={part}",
        deviations=a.deviations,
    )
    try:
        return run(cfg)
    except (ValueError, OSError, StopIteration) as e:
        print(f"run.py: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
