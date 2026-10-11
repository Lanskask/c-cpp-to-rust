#!/usr/bin/env python3
"""The ticket set of one translation run: init, status, mark. State lives in `<ws>/migration/tickets/*.md`.

A run on a Pilot is seven tickets in a chain (tracer-bullet phases, each demoable on its own):
  01 Mapping (Stage 0) -> 02 Stage 1 Judge -> 03 Stress test -> 04 Stage 1 Faithful port ->
  05 Stage 2 Judge -> 06 Stage 2 Idiomatic pass -> 07 Report.
`init` writes the files and `<ws>/migration/Makefile` (`status`, `tickets`); it refuses to overwrite unless `--force`.
`makefile` rewrites only the Makefile. `status` prints each ticket's state and whether it is unblocked; `status
--tasks` also lists every checkbox task (`[x]` done, `[ ]` open). `tick NN TEXT` ticks the one open task of ticket NN
whose text contains TEXT. `mark NN <todo|in_progress|done>` changes the `**Status:**` line and refuses
`done`/`in_progress` while a blocker is not done, and `done` while a task is still `[ ]` (`--tick-all` ticks them
after the governor has checked each). `done` also needs `--brief TEXT` (at most 8 lines of 120 columns: what was
produced, commit, numbers from runs): it is written under the `-----------` line of the ticket. `show NN` prints
ticket NN and the briefs of its blockers only (`mark NN in_progress` prints the same). A ticket is: What to build,
Blocked by (number and name), Status, `## Tasks` (what to do, each with its how-steps), `## Acceptance` (what proves
it), then after `-----------` the brief. Tickets written before this layout (no `## Tasks`, no brief) still work.
The governor reads `status` at the start of every session. Exit code 0 on success, 1 on failure.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

STATES = ("todo", "in_progress", "done")
OPEN_TASK = re.compile(r"^- \[ \] (.*)$", re.M)


BRIEF_RULE = "-----------"
BRIEF_LINE = re.compile(rf"^{BRIEF_RULE}$", re.M)
BRIEF_MAX_LINES = 8
BRIEF_MAX_WIDTH = 120


@dataclass(frozen=True)
class Task:
    """One thing to do: a checkbox line and the how-steps printed under it (commands, files, references)."""

    text: str
    how: tuple[str, ...] = ()


@dataclass(frozen=True)
class Spec:
    num: str
    slug: str
    title: str
    build: str
    blocked_by: tuple[str, ...]
    tasks: tuple[Task, ...]
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
            Task(
                "Inventory the Source and map its dependencies",
                (
                    "`make spend-now`, note the timestamp for the SPEND row",
                    "`inventory.py <ws>/src --lang c|cpp --out <ws>/migration/inventory.tsv` (add `--exclude GLOB` for "
                    "non-library directories; tests and examples are never Parts)",
                    "`depmap.py <ws>/migration/inventory.tsv --source-root <ws>/src --out-dir <ws>/migration`",
                ),
            ),
            Task(
                "Tier the Parts",
                ("`tier.py <ws>/migration/inventory.tsv <ws>/migration/order.txt --out <ws>/migration/parts.tsv`",),
            ),
            Task(
                "Draft RULEBOOK.md (Opus)",
                (
                    "start from `references/rulebook-template.md` and `references/idiom-table-c.md`",
                    "every `config-macro:` tag gets a §3 row, every UB or platform behaviour a §8 Deviation row "
                    "with status `proposed`",
                ),
            ),
            Task(
                "Get the Deviations decided by the user",
                (
                    "show the §8 table and stop; the user marks each row `approved` or `vetoed`",
                    "set the rulebook status line to `deviations-approved`",
                ),
            ),
        ),
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
            Task(
                "Write the driver and the corpus generator",
                (
                    "one canonical dump of every observable, in every API mode (`references/judge.md` §2)",
                    "`make_inputs.py` with a fixed seed writes the git-ignored corpus; drop non-deterministic inputs "
                    "(§3)",
                ),
            ),
            Task(
                "Write run.py for every feature set",
                (
                    "build the staticlib, link the original tests (C only), build the driver twice, run "
                    "`harness.py --orig --rust --inputs --out` (§4)",
                    "outputs go under `migration/harness/work/`",
                    "the crate does not exist yet: validate on a scratch crate in `<ws>/experiments/scratch-crate/` "
                    "(§4a)",
                ),
            ),
            Task(
                "Prove the Judge can fail",
                (
                    "plant a one-character mutant in the scratch crate, run, restore (§6)",
                    "write `<ws>/experiments/e<N>_<pilot>_judge_sensitivity.py`, docstring: which piece caught it",
                    "time the first run and record the numbers in RUN-NOTES.md",
                ),
            ),
        ),
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
            Task(
                "Translate the hardest Parts twice",
                (
                    "pick the 3 hardest `judgment` Parts of `parts.tsv` (fewer if fewer exist)",
                    "each twice with `references/stage1-prompt.md`, into scratch copies, never the accepted target",
                ),
            ),
            Task(
                "Turn every disagreement into a rule proposal",
                (
                    "diff the two translations of each Part; each difference two agents decided differently is a "
                    "missing rule",
                    "write them under `## Rule proposals` in RUN-NOTES.md",
                ),
            ),
            Task(
                "Apply the accepted proposals (Opus)",
                ("set the rulebook status line to `stress-tested (<date>)`; if the user skips the ticket, record why",),
            ),
        ),
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
        (
            Task(
                "Protect the Rulebook and the Judge, then start the fan-out",
                (
                    "session started with `claude --settings skill/c-to-rust/assets/settings.json` (SKILL.md step 3.1)",
                    "`tickets.py mark 04 in_progress`, then an `Edit` of RULEBOOK.md must be refused; if not, stop",
                    "write `rust/src/lib.rs` (`pub mod <stem>;` per Part) and the cargo features of rulebook §3",
                ),
            ),
            Task(
                "Port every Part in batches",
                (
                    "`workqueue.py --parts <ws>/migration/parts.tsv next --batch N`",
                    "per Part: `gate.py --snapshot` -> implementer (`references/stage1-prompt.md`) -> `gate.py` -> "
                    "review (`references/review-prompt.md`) -> commit -> `workqueue.py mark`",
                    "up to 5 attempts, then `stubbed` (real signatures, `todo!()` bodies, RUN-NOTES entry)",
                ),
            ),
            Task(
                "Log every batch",
                ("`make spend PHASE=<phase> SINCE=<timestamp> NOTE=...` and one WORK-LOG entry per batch",),
            ),
        ),
        ("workqueue.py verify passes", "gate pass recorded for every done Part", "SPEND.md has the batch rows"),
    ),
    Spec(
        "05",
        "stage2-judge",
        "Stage 2 Judge",
        "The adapter, Rust driver and translated tests pass on the Faithful port and fail on the mutant "
        "(judge.md §5), in their own commit.",
        ("04",),
        (
            Task(
                "Check that rulebook §6 (Stage 2 API) is filled",
                ("if empty, stop and ask the user for it (Opus writes it)",),
            ),
            Task(
                "Write the adapter, the Rust driver and the translated tests",
                (
                    "`rust/tests/adapter/mod.rs` (the only file that calls the crate), `rust/examples/dump.rs`, "
                    "`rust/tests/upstream.rs` (`references/judge.md` §5)",
                    "`run.py --stage 2` runs `cargo test`, then `harness.py` with the C driver against the Rust one",
                    "list allocation failure, NULL members and unterminated input as covered or not testable "
                    "(`references/judge.md` §5a)",
                ),
            ),
            Task(
                "Validate on the Faithful port and on the mutant",
                (
                    "green in every feature set on the Faithful port; red with the planted mutant, then restore",
                    "one commit for the adapter, driver and tests, before the Stage 2 implementer starts",
                    "copy `parts.tsv` to `parts-stage2.tsv` (all `todo`, attempts 0, no commit)",
                ),
            ),
        ),
        ("passes on the Faithful port in every feature set", "mutant goes red", "parts-stage2.tsv created"),
    ),
    Spec(
        "06",
        "stage2-idiomatic-pass",
        "Stage 2 Idiomatic pass",
        "Every Part is idiomatic Rust that passes the Gate: zero unsafe, clippy and rustfmt clean, no unjustified "
        "unwrap, same Corpus output.",
        ("05",),
        (
            Task(
                "Start the fan-out with the guard active",
                ("`tickets.py mark 06 in_progress`, then an `Edit` of RULEBOOK.md must be refused (SKILL.md 3.1)",),
            ),
            Task(
                "Rewrite every Part in batches",
                (
                    "`workqueue.py --parts <ws>/migration/parts-stage2.tsv next --batch N`",
                    "per Part: snapshot -> implementer (`references/stage2-prompt.md`) -> `gate.py --stage 2` -> "
                    "review -> commit -> mark; the Rust stays as close to the C behaviour as the rulebook says",
                    "a behaviour change is a Deviation row, not a silent edit",
                ),
            ),
            Task(
                "Log every batch and count the open ports",
                (
                    "`make spend ...` and a WORK-LOG entry per batch",
                    '`grep -rn "TODO(port)" <ws>/rust/src` for the report',
                ),
            ),
        ),
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
            Task(
                "Collect the numbers",
                (
                    "`workqueue.py status` for both stages; the §8 Deviations table with statuses",
                    "tokens for this Pilot from `SPEND.md` / `SPEND-MODELS.tsv`; open `TODO(port)` count",
                ),
            ),
            Task(
                "List the interventions",
                ("every time the user had to step in; each one not on the allowed list is a Skill bug in WORK-LOG",),
            ),
            Task("Write the report and recommend one next step"),
        ),
        (
            "SPEND-MODELS.tsv rows for the run",
            "every intervention not in the allowed list is a Skill bug in WORK-LOG",
            "next step recommended",
        ),
    ),
)


def render(spec: Spec, pilot: str) -> str:
    names = {c.num: c.title for c in CHAIN}
    blocked = ", ".join(f"{b} {names[b]}" for b in spec.blocked_by) or "None (can start immediately)"
    tasks = "\n".join(f"- [ ] {t.text}" + "".join(f"\n  - {h}" for h in t.how) for t in spec.tasks)
    checks = "\n".join(f"- [ ] {a}" for a in spec.acceptance)
    return (
        f"# {spec.num}: {spec.title} ({pilot})\n\n**What to build:** {spec.build}\n\n"
        f"**Blocked by:** {blocked}\n\n**Status:** todo\n\n"
        f"## Tasks\n\n{tasks}\n\n## Acceptance\n\n{checks}\n"
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
    if not m:
        return []
    return [n.group(1) for item in m.group(1).split(",") if (n := re.match(r"\s*(\d{2})\b", item))]


def split_brief(text: str) -> tuple[str, str | None]:
    """The ticket body above the `-----------` line, and the brief below it (None when there is none yet)."""
    m = BRIEF_LINE.search(text)
    if not m:
        return text, None
    return text[: m.start()], text[m.end() :].strip() or None


def format_brief(raw: str) -> str:
    lines = [ln.strip() for ln in raw.strip().splitlines() if ln.strip()]
    if not lines:
        raise ValueError("--brief is empty: say what was produced, the commit and the numbers (max 8 short lines)")
    bullets = [ln if ln.startswith("- ") else f"- {ln}" for ln in lines]
    if len(bullets) > BRIEF_MAX_LINES or any(len(b) > BRIEF_MAX_WIDTH for b in bullets):
        raise ValueError(f"--brief is too long: at most {BRIEF_MAX_LINES} lines of {BRIEF_MAX_WIDTH} columns")
    return "## Done\n\n" + "\n".join(bullets)


def display_title(ws: Path, path: Path) -> str:
    title, _ = title_and_tasks(path)
    return title.removesuffix(f" ({ws.name})")


def print_blocker_briefs(ws: Path, tickets: dict[str, Path], num: str) -> None:
    for b in blockers(tickets[num]):
        if b not in tickets:
            continue
        _, brief = split_brief(tickets[b].read_text())
        body = brief.removeprefix("## Done").strip() if brief else "(no brief yet)"
        print(f"== brief of {b} {display_title(ws, tickets[b])}\n{body}")


def load(ws: Path) -> dict[str, Path]:
    files = sorted(ticket_dir(ws).glob("[0-9][0-9]-*.md"))
    if not files:
        raise ValueError(f"no tickets in {ticket_dir(ws)} (run: tickets.py init --ws {ws})")
    return {f.name[:2]: f for f in files}


def write_makefile(ws: Path) -> Path:
    """`<ws>/migration/Makefile` with `status` and `tickets`, scripts addressed by absolute path."""
    template = Path(__file__).resolve().parent.parent / "assets" / "migration.Makefile"
    target = ws / "migration" / "Makefile"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(template.read_text())
    return target


def cmd_tick(ws: Path, num: str, text: str) -> int:
    tickets = load(ws)
    if num not in tickets:
        raise ValueError(f"no ticket {num}")
    body = tickets[num].read_text()
    hits = [t for t in OPEN_TASK.findall(split_brief(body)[0]) if text.lower() in t.lower()]
    if len(hits) != 1:
        open_tasks = OPEN_TASK.findall(body)
        raise ValueError(
            f"ticket {num}: {len(hits)} open tasks match {text!r} (need exactly 1); open: {open_tasks or 'none'}"
        )
    tickets[num].write_text(body.replace(f"- [ ] {hits[0]}", f"- [x] {hits[0]}", 1))
    print(f"tickets: {num} ticked: {hits[0]}", file=sys.stderr)
    return 0


def cmd_init(ws: Path, force: bool) -> int:
    out = ticket_dir(ws)
    if out.exists() and any(out.glob("*.md")) and not force:
        print(f"tickets: {out} exists (use --force to overwrite)", file=sys.stderr)
        return 1
    out.mkdir(parents=True, exist_ok=True)
    for spec in CHAIN:
        (out / f"{spec.num}-{spec.slug}.md").write_text(render(spec, ws.name))
    mk = write_makefile(ws)
    print(f"tickets: wrote {len(CHAIN)} tickets to {out} and {mk}", file=sys.stderr)
    return 0


def title_and_tasks(path: Path) -> tuple[str, list[str]]:
    text = split_brief(path.read_text())[0]
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


def cmd_mark(ws: Path, num: str, new: str, tick_all: bool = False, brief: str | None = None) -> int:
    tickets = load(ws)
    if num not in tickets:
        raise ValueError(f"no ticket {num}")
    path = tickets[num]
    text = path.read_text()
    if new == "done":
        open_tasks = OPEN_TASK.findall(split_brief(text)[0])
        if open_tasks and not tick_all:
            raise ValueError(
                f"ticket {num} has {len(open_tasks)} open task(s); tick each `- [x]` once it holds "
                f"(a skipped one: `- [x] <task> (skipped: <reason>)`), or pass --tick-all after checking them:\n  "
                + "\n  ".join(open_tasks)
            )
        if brief is None:
            raise ValueError(
                f"ticket {num}: `done` needs --brief TEXT (what was produced, commit, numbers; max 8 lines)"
            )
        formatted = format_brief(brief)
    if new != "todo":
        open_blockers = [b for b in blockers(path) if read_state(tickets[b]) != "done"]
        if open_blockers:
            raise ValueError(f"ticket {num} is blocked by {', '.join(open_blockers)}")
    if new == "done":
        body = split_brief(text)[0]
        if tick_all:
            body = re.sub(r"^- \[ \] ", "- [x] ", body, flags=re.M)
        text = f"{body.rstrip()}\n\n{BRIEF_RULE}\n{formatted}\n"
    path.write_text(re.sub(r"^(\*\*Status:\*\* )\w+", rf"\g<1>{new}", text, count=1, flags=re.M))
    print(f"tickets: {num} -> {new}", file=sys.stderr)
    if new == "in_progress":
        print_blocker_briefs(ws, tickets, num)
    return 0


def cmd_show(ws: Path, num: str) -> int:
    tickets = load(ws)
    if num not in tickets:
        raise ValueError(f"no ticket {num}")
    print(tickets[num].read_text().rstrip())
    print_blocker_briefs(ws, tickets, num)
    return 0


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--ws", required=True, type=Path, help="Pilot workspace, e.g. pilots/cjson")
    sub = p.add_subparsers(dest="cmd", required=True)
    i = sub.add_parser("init")
    i.add_argument("--force", action="store_true")
    st = sub.add_parser("status")
    st.add_argument("--tasks", action="store_true", help="also list each ticket's checkbox tasks with [x] / [ ]")
    sub.add_parser("makefile", help="(re)write <ws>/migration/Makefile with the status and tickets targets")
    t = sub.add_parser("tick", help="tick the one open task of ticket NN whose text contains TEXT")
    t.add_argument("num")
    t.add_argument("text")
    m = sub.add_parser("mark")
    m.add_argument("num")
    m.add_argument("state", choices=STATES)
    m.add_argument("--tick-all", action="store_true", help="with `done`: tick every open task (only after checking)")
    m.add_argument("--brief", help="with `done`: what was produced, commit, numbers; max 8 lines of 120 columns")
    sh = sub.add_parser("show", help="print ticket NN and the briefs of its blockers")
    sh.add_argument("num")
    args = p.parse_args(argv)
    try:
        if args.cmd == "init":
            return cmd_init(args.ws, args.force)
        if args.cmd == "makefile":
            print(f"tickets: wrote {write_makefile(args.ws)}", file=sys.stderr)
            return 0
        if args.cmd == "tick":
            return cmd_tick(args.ws, args.num, args.text)
        if args.cmd == "status":
            return cmd_status(args.ws, args.tasks)
        if args.cmd == "show":
            return cmd_show(args.ws, args.num)
        return cmd_mark(args.ws, args.num, args.state, args.tick_all, args.brief)
    except (ValueError, OSError) as e:
        print(f"tickets: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
