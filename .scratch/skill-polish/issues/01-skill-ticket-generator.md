# 01: The Skill creates the ticket set for a translation run

**What to build:** Start a run on any Pilot and the Skill itself writes `<ws>/migration/tickets/` (Mapping, Stage 1
Judge, stress test, Stage 1 Faithful port, Stage 2 Judge, Stage 2 Idiomatic pass, report), with blocking edges,
acceptance checklists and a `mark`/`status` command; SKILL.md says when to create and update them.

**Blocked by:** None (can start immediately)

**Status:** done (2026-10-10)

- [x] `tickets.py init` writes seven ticket files for a Pilot and refuses to overwrite without `--force`
- [x] `tickets.py status` lists each ticket's state and which are unblocked
- [x] `tickets.py mark NN done` refuses when a blocker is not done
- [x] SKILL.md names the step; a test covers init, status and mark
