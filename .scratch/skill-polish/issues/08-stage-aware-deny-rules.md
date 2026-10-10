# 08: Deny-rules follow the ticket state

**What to build:** A Translation session can write the Rulebook, Judge files and `parts*.tsv` during Mapping and Judge
building, and cannot during fan-out, without restarting: a guard script wired as a PreToolUse hook reads
`tickets.py status` and refuses the stage-dependent paths only while tickets 04 or 06 are in_progress.

**Blocked by:** 01

**Status:** done (2026-10-10; hook firing in a real session still to be observed)

- [x] `assets/settings.json` keeps only the always-protected paths statically
- [x] `guard.py` refuses RULEBOOK.md, parts*.tsv, harness/**, rust/tests/*.rs, rust/examples/** only while ticket 04 or
  06 is in_progress
- [x] SKILL.md: proof step is 'mark 04 in_progress, then an Edit of RULEBOOK.md is refused'; a Judge-gap change means
  mark 04 todo, commit, mark in_progress
- [x] the guard also covers Bash writes to those paths, or the gap is documented
- [x] tests for allowed, refused and unknown states
