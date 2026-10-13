"""Whole-run driver: `run.py <ws> --all` works every ticket from an empty workspace to the report (decision 74).

Called from run.py (`--all`); not a separate command. Per ticket:

  none yet      `tickets.py init --lang auto|c|cpp` (creates `<ws>/migration/`, its git repository, tickets, Makefile)
  01 mapping    mechanical `make parts` (inventory, depmap, tier) when there is no parts.tsv, then a governor run
  02 03 05 07   a governor run: headless `claude -p` that works the ticket's tasks (01, 02, 05: the planner model,
                Opus; 03, 07: the implementer model, Sonnet), ticks them and marks the ticket done with a brief
  04 06         the Part fan-out of run.py (implementer, gate, reviewer per Part), then a short governor run to close
                the ticket

The driver re-reads the ticket files after every step, so a rerun resumes. It stops (exit 3) when a governor run
leaves its ticket open `--max-attempts` times, when a governor writes `migration/QUESTIONS.md` (a decision only the
user can make), when a deviation row is `proposed` (or `approved (policy)` under `--deviations ask`), on a stubbed
Part, or at 70% of `--budget-output-tokens` counted over the whole run. Without `--go` it prints the plan only.
"""

from __future__ import annotations

import subprocess
import sys
from dataclasses import replace
from datetime import UTC, datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import layout  # noqa: E402
import run as drv  # noqa: E402
import tickets as tk  # noqa: E402

PLANNER_TICKETS = ("01", "02", "05")  # rulebook, deviations, Judge, section 6: the planner model (Opus)
FANOUT = {"04": 1, "06": 2}
GOVERNOR_TOOLS = "Read,Edit,Write,Grep,Glob,Bash(python3 -I:*),Bash(make:*),Bash(cargo:*),Bash(git:*),Bash(ls:*)"
CPP_SUFFIXES = {".cpp", ".cc", ".cxx", ".hpp", ".hh", ".hxx"}

PROMPT = """You are the governor of a C/C++ to Rust migration, running headless: nobody answers questions.
Workspace: {ws}
Skill: read {skill}/SKILL.md first; its scripts are in {scripts} (run them as `python3 -I {scripts}/<name>.py`).
Everything you create goes under {ws}/migration/ (the original source is read-only). Run builds and the Judge
through `make -C {ws}/migration <target>` (`make help`) or the Skill's scripts, never ad-hoc compiler calls.

Your job: ticket {num} only. Work its tasks in order; after each task has its evidence run
`python3 -I {scripts}/tickets.py --ws {ws} tick {num} "<task text>"`; when every task is ticked run
`python3 -I {scripts}/tickets.py --ws {ws} mark {num} done --brief "<what was produced, commit, numbers>"`.
Commit with `make -C {ws}/migration commit MSG="..."`.
Deviation policy: {policy}
If a decision is only the user's to make, write the question to {ws}/migration/QUESTIONS.md and stop without
marking the ticket done.
{extra}
TICKET_NUM={num}
{previous}
--- ticket file ---
{ticket}
"""

POLICY = {
    "policy": "rows whose Rust behaviour reproduces the original (SKILL.md, policy approval) get `approved (policy)`; "
    "anything else stays `proposed`.",
    "ask": "never set a Deviation row to approved: leave every row `proposed` for the user.",
}
CLOSE_EXTRA = (
    "The driver has already run the Part fan-out of this ticket and every Part is done or stubbed: verify with "
    "`make -C {ws}/migration verify STAGE={stage}`, tick the ticket's tasks that the evidence supports, write the "
    "brief and mark it done. Do not translate Parts yourself."
)


def detect_lang(ws: Path) -> str:
    for p in ws.rglob("*"):
        if "migration" in p.relative_to(ws).parts or ".git" in p.parts:
            continue
        if p.suffix in CPP_SUFFIXES:
            return "cpp"
    return "c"


def blockers_done(ws: Path, num: str, states: dict[str, str]) -> bool:
    files = tk.load(ws)
    return all(states.get(b) == "done" for b in tk.blockers(files[num]))


def ready_ticket(ws: Path) -> str | None:
    states = drv.ticket_status(ws)
    for num, state in states.items():
        if state != "done" and blockers_done(ws, num, states):
            return num
    return None


def step_plan(cfg: drv.Config, num: str) -> str:
    if num in FANOUT:
        fan = f"Part fan-out of stage {FANOUT[num]} (implementer {cfg.model}, reviewer {cfg.reviewer_model})"
        return f"{fan}, then close-out ({cfg.model})"
    model = cfg.planner_model if num in PLANNER_TICKETS else cfg.model
    pre = "make parts, then " if num == "01" else ""
    return f"{pre}governor run ({model})"


def plan(cfg: drv.Config) -> list[str]:
    ws = cfg.ws
    if not (ws / "migration" / "tickets").is_dir():
        lang = cfg.lang or detect_lang(ws)
        return [f"tickets.py --ws {ws} init --lang {lang}   (creates {ws}/migration, its git repository)"]
    states = drv.ticket_status(ws)
    cur = ready_ticket(ws)
    names = {n: f.name[3:-3] for n, f in tk.load(ws).items()}
    return [
        f"{'-> ' if n == cur else '   '}ticket {n} {names[n]:<22} {st:<12} {step_plan(cfg, n)}"
        for n, st in states.items()
    ]


def governor(cfg: drv.Config, num: str, model: str, extra: str, previous: str) -> tuple[int, str]:
    text = tk.load(cfg.ws)[num].read_text()
    prompt = PROMPT.format(
        ws=cfg.ws,
        skill=drv.SKILL,
        scripts=drv.SKILL / "scripts",
        num=num,
        policy=POLICY[cfg.deviations],
        extra=extra,
        previous=f"Previous attempt: {previous}\n" if previous else "",
        ticket=text,
    )
    c = replace(cfg, max_turns=cfg.governor_turns)
    return drv.timed(drv.run_claude, f"governor ticket {num} ({model})", c, model, GOVERNOR_TOOLS, prompt)


def questions(ws: Path) -> str:
    q = ws / "migration" / "QUESTIONS.md"
    return q.read_text().strip() if q.is_file() else ""


def finish(cfg: drv.Config, done: list[str], reason: str, code: int) -> int:
    lines = [f"tickets finished this run: {', '.join(done) or 'none'}"] + ([f"STOPPED: {reason}"] if reason else [])
    drv.log_work(cfg, lines)
    layout.commit(cfg.ws, "run.py --all: " + (reason or "all tickets done"))
    drv.say("summary")
    print("run.py --all summary:\n  " + "\n  ".join(lines))
    return code


def run_all(cfg: drv.Config) -> int:
    ws = cfg.ws
    print("run.py --all plan:\n  " + "\n  ".join(plan(cfg)))
    if not cfg.go:
        print("run.py: nothing spent; rerun with --go to start")
        return 0
    started = datetime.now(UTC).isoformat(timespec="seconds")
    if not (ws / "migration" / "tickets").is_dir():
        lang = cfg.lang or detect_lang(ws)
        r = subprocess.run(
            [sys.executable, "-I", str(drv.SKILL / "scripts" / "tickets.py"), "--ws", str(ws), "init", "--lang", lang],
            capture_output=True,
            text=True,
        )
        if r.returncode != 0:
            print(f"run.py: tickets init failed: {r.stderr.strip()}", file=sys.stderr)
            return 1
        drv.say(f"initialised {ws}/migration ({lang})")
    done: list[str] = []
    attempts: dict[str, int] = {}
    previous = ""
    while True:
        num = ready_ticket(ws)
        if num is None:
            states = drv.ticket_status(ws)
            if all(s == "done" for s in states.values()):
                return finish(cfg, done, "", 0)
            return finish(cfg, done, f"no ticket is ready: {states}", drv.STOP)
        used = drv.spend_output(cfg, started)
        if cfg.budget and used >= 0.7 * cfg.budget:
            return finish(cfg, done, f"output-token budget passed 70% of {cfg.budget}", drv.STOP)
        if attempts.get(num, 0) >= cfg.max_attempts:
            return finish(cfg, done, f"ticket {num} still open after {cfg.max_attempts} governor runs", drv.STOP)
        attempts[num] = attempts.get(num, 0) + 1
        states = drv.ticket_status(ws)
        drv.say(f"ticket {num} ({sum(s == 'done' for s in states.values())}/{len(states)} done): {step_plan(cfg, num)}")
        if states[num] == "todo":
            subprocess.run(
                [
                    sys.executable,
                    "-I",
                    str(drv.SKILL / "scripts" / "tickets.py"),
                    "--ws",
                    str(ws),
                    "mark",
                    num,
                    "in_progress",
                ],
                capture_output=True,
            )
        model = cfg.planner_model if num in PLANNER_TICKETS else cfg.model
        extra = ""
        if num == "01" and not (ws / "migration" / "parts.tsv").is_file():
            code, out = drv.sh(cfg, f"make -C {ws}/migration parts", num)
            drv.say("  make parts: " + ("ok" if code == 0 else f"failed: {out.strip()[-200:]}"))
        if num in FANOUT:
            sub = replace(cfg, stage=FANOUT[num], go=True, budget=max(cfg.budget - used, 0) if cfg.budget else 0)
            code = drv.run(sub)
            if code != 0:
                return finish(cfg, done, f"Part fan-out of stage {FANOUT[num]} stopped (exit {code}, see above)", code)
            extra = CLOSE_EXTRA.format(ws=ws, stage=FANOUT[num])
            model = cfg.model
        code, out = governor(cfg, num, model, extra, previous)
        previous = "" if code == 0 else f"governor run failed (exit {code}): {out.strip()[-300:]}"
        layout.commit(ws, f"run.py --all: ticket {num} governor step")
        q = questions(ws)
        if q:
            return finish(cfg, done, f"the governor needs a decision (migration/QUESTIONS.md):\n{q}", drv.STOP)
        state = drv.ticket_status(ws).get(num)
        if state == "done":
            done.append(num)
            drv.say(f"ticket {num}: DONE")
        else:
            previous = previous or f"ticket {num} is `{state}`: finish its tasks and mark it done"
            drv.say(f"ticket {num} not done after governor run {attempts[num]}/{cfg.max_attempts}: {previous[-160:]}")


if __name__ == "__main__":
    print(__doc__)
    print("usage: python3 -I run.py <ws> --all [--go] [--deviations policy] [--max-attempts N] (this file is a module)")
    sys.exit(0 if "--help" in sys.argv[1:] or "-h" in sys.argv[1:] else 2)
