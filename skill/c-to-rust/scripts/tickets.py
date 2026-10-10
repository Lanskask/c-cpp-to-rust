#!/usr/bin/env python3
"""The ticket set of one translation run: init, status, mark. State lives in `<ws>/migration/tickets/*.md`.

A run on a Pilot is seven tickets in a chain (tracer-bullet phases, each demoable on its own):
  01 Mapping (Stage 0) -> 02 Stage 1 Judge -> 03 Stress test -> 04 Stage 1 Faithful port ->
  05 Stage 2 Judge -> 06 Stage 2 Idiomatic pass -> 07 Report.
`init` writes the files (refuses to overwrite unless `--force`); `status` prints each ticket's state and whether
it is unblocked, and `status --tasks` also lists every checkbox task under its ticket (`[x]` done, `[ ]` open);
`mark NN <todo|in_progress|done>` changes the `**Status:**` line and refuses `done`/`in_progress` while a blocker
is not done. The governor reads `status` at the start of every session, next to `parts.tsv`.
Exit code 0 on success, 1 on failure.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

STATES = ("todo", "in_progress", "done")


@dataclass(frozen=True)
class Spec:
    num: str
    slug: str
    title: str
    build: str
    blocked_by: tuple[str, ...]
    acceptance: tuple[str, ...]


CHAIN = (
    Spec(
        "01",
        "mapping",
        "Mapping (Stage 0)",
        "The Source project is inventoried, its dependencies mapped, Parts tiered, and a RULEBOOK drafted whose "
        "Deviations table the user can approve.",
        (),
        (
            "inventory.tsv, edges.tsv, order.txt, parts.tsv exist",
            "every config-macro has a RULEBOOK §3 row",
            "every UB or platform behaviour has a §8 Deviation row",
            "the user approved or vetoed every Deviation",
        ),
    ),
    Spec(
        "02",
        "stage1-judge",
        "Stage 1 Judge",
        "The acceptance checks for Same behaviour exist and are shown able to fail (judge.md §1-§4, §6).",
        ("01",),
        (
            "original tests link to the Rust crate (C only)",
            "driver and deterministic corpus per feature set",
            "a planted mutant makes the Judge fail in every feature set",
            "numbers recorded in RUN-NOTES.md",
        ),
    ),
    Spec(
        "03",
        "stress-test",
        "Rulebook stress test",
        "The hardest Parts are translated twice with the Stage 1 prompt and the differences become rules.",
        ("01", "02"),
        (
            "each difference is a rule proposal in RUN-NOTES.md",
            "accepted proposals applied by Opus",
            "a same-model rerun agrees, or the remaining differences are classified",
            "status line says stress-tested",
        ),
    ),
    Spec(
        "04",
        "stage1-faithful-port",
        "Stage 1 Faithful port",
        "Every Part is done or stubbed, each accepted by the Gate and committed.",
        ("03",),
        ("workqueue.py verify passes", "gate pass recorded for every done Part", "SPEND.md has the batch rows"),
    ),
    Spec(
        "05",
        "stage2-judge",
        "Stage 2 Judge",
        "The adapter, Rust driver and translated tests pass on the Faithful port and fail on the mutant "
        "(judge.md §5), in their own commit.",
        ("04",),
        ("passes on the Faithful port in every feature set", "mutant goes red", "parts-stage2.tsv created"),
    ),
    Spec(
        "06",
        "stage2-idiomatic-pass",
        "Stage 2 Idiomatic pass",
        "Every Part is idiomatic Rust that passes the Gate: zero unsafe, clippy and rustfmt clean, no unjustified "
        "unwrap, same Corpus output.",
        ("05",),
        ("all Stage 2 rows done", "Gate passes in every feature set with the full corpus", "TODO(port) count reported"),
    ),
    Spec(
        "07",
        "report",
        "Report",
        "One page: Parts per stage, Deviations with statuses, open TODO(port), tokens by model, interventions "
        "the user had to make, one recommended next step.",
        ("06",),
        (
            "SPEND-MODELS.tsv rows for the run",
            "every intervention not in the allowed list is a Skill bug in WORK-LOG",
            "next step recommended",
        ),
    ),
)


def render(spec: Spec, pilot: str) -> str:
    blocked = ", ".join(spec.blocked_by) or "None (can start immediately)"
    checks = "\n".join(f"- [ ] {a}" for a in spec.acceptance)
    return (
        f"# {spec.num}: {spec.title} ({pilot})\n\n**What to build:** {spec.build}\n\n"
        f"**Blocked by:** {blocked}\n\n**Status:** todo\n\n{checks}\n"
    )


def ticket_dir(ws: Path) -> Path:
    return ws / "migration" / "tickets"


def read_state(path: Path) -> str:
    m = re.search(r"^\*\*Status:\*\* (\w+)", path.read_text(), re.M)
    if not m or m.group(1) not in STATES:
        raise ValueError(f"{path.name}: no valid **Status:** line")
    return m.group(1)


def blockers(path: Path) -> list[str]:
    m = re.search(r"^\*\*Blocked by:\*\* (.*)$", path.read_text(), re.M)
    return re.findall(r"\b\d{2}\b", m.group(1)) if m else []


def load(ws: Path) -> dict[str, Path]:
    files = sorted(ticket_dir(ws).glob("[0-9][0-9]-*.md"))
    if not files:
        raise ValueError(f"no tickets in {ticket_dir(ws)} (run: tickets.py init --ws {ws})")
    return {f.name[:2]: f for f in files}


def cmd_init(ws: Path, force: bool) -> int:
    out = ticket_dir(ws)
    if out.exists() and any(out.glob("*.md")) and not force:
        print(f"tickets: {out} exists (use --force to overwrite)", file=sys.stderr)
        return 1
    out.mkdir(parents=True, exist_ok=True)
    for spec in CHAIN:
        (out / f"{spec.num}-{spec.slug}.md").write_text(render(spec, ws.name))
    print(f"tickets: wrote {len(CHAIN)} tickets to {out}", file=sys.stderr)
    return 0


def title_and_tasks(path: Path) -> tuple[str, list[str]]:
    text = path.read_text()
    m = re.search(r"^# \d{2}: (.*)$", text, re.M)
    tasks = [f"  - [{c.lower()}] {t}" for c, t in re.findall(r"^- \[([ xX])\] (.*)$", text, re.M)]
    return (m.group(1) if m else path.stem[3:]), tasks


def cmd_status(ws: Path, with_tasks: bool = False) -> int:
    tickets = load(ws)
    state = {n: read_state(p) for n, p in tickets.items()}
    for n, p in tickets.items():
        if with_tasks:
            title, tasks = title_and_tasks(p)
            ready = state[n] == "todo" and all(state.get(b) == "done" for b in blockers(p))
            print(f"Ticket_{n} {state[n]}{' READY' if ready else ''} {title}")
            print("\n".join(tasks))
            continue
        free = all(state.get(b) == "done" for b in blockers(p))
        mark = "READY" if state[n] == "todo" and free else ""
        print(f"{n}\t{state[n]}\t{mark}\t{p.stem[3:]}")
    done = sum(s == "done" for s in state.values())
    print(f"tickets: {done}/{len(state)} done", file=sys.stderr)
    return 0


def cmd_mark(ws: Path, num: str, new: str) -> int:
    tickets = load(ws)
    if num not in tickets:
        raise ValueError(f"no ticket {num}")
    if new != "todo":
        open_blockers = [b for b in blockers(tickets[num]) if read_state(tickets[b]) != "done"]
        if open_blockers:
            raise ValueError(f"ticket {num} is blocked by {', '.join(open_blockers)}")
    path = tickets[num]
    path.write_text(re.sub(r"^(\*\*Status:\*\* )\w+", rf"\g<1>{new}", path.read_text(), count=1, flags=re.M))
    print(f"tickets: {num} -> {new}", file=sys.stderr)
    return 0


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--ws", required=True, type=Path, help="Pilot workspace, e.g. pilots/cjson")
    sub = p.add_subparsers(dest="cmd", required=True)
    i = sub.add_parser("init")
    i.add_argument("--force", action="store_true")
    st = sub.add_parser("status")
    st.add_argument("--tasks", action="store_true", help="also list each ticket's checkbox tasks with [x] / [ ]")
    m = sub.add_parser("mark")
    m.add_argument("num")
    m.add_argument("state", choices=STATES)
    args = p.parse_args(argv)
    try:
        if args.cmd == "init":
            return cmd_init(args.ws, args.force)
        if args.cmd == "status":
            return cmd_status(args.ws, args.tasks)
        return cmd_mark(args.ws, args.num, args.state)
    except (ValueError, OSError) as e:
        print(f"tickets: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
