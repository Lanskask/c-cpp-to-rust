## Local tickets with a script and a Makefile

For the **local files** tracker, do not hand-write the files. Write the approved breakdown as `spec.json` and let the
script number the tickets, write one file per ticket and add a Makefile:

```bash
python3 -I <this skill>/scripts/tickets.py --dir .scratch/<feature-slug>/issues init --spec .scratch/<feature-slug>/spec.json
make -C .scratch/<feature-slug>/issues status      # state per ticket, READY = todo with every blocker done
make -C .scratch/<feature-slug>/issues tickets     # every ticket with its tasks, [x] done and [ ] open
make -C .scratch/<feature-slug>/issues show NUM=02 # one ticket plus the briefs of its blockers
```

`spec.json` is a list in dependency order (blockers first), one object per ticket:

```json
[
  {"slug": "parse", "title": "Parse the input", "build": "A user can load a file and see its records.",
   "blocked_by": [],
   "tasks": [{"text": "Write the reader", "how": ["module and command to start from"]}, "Add a fixture"],
   "acceptance": ["the fixture loads", "a bad file gives a readable error"]},
  {"slug": "report", "title": "Report", "build": "The records print as a table.", "blocked_by": ["parse"],
   "tasks": ["Format the table"], "acceptance": ["output matches the snapshot"]}
]
```

`init` also copies `tickets.py` next to the tickets, so the Makefile keeps working without the skill. State lives in
the files and is changed only through the script:

- `tickets.py --dir <dir> tick 02 "<text of the task>"` ticks the one open task that contains the text, the moment its
  evidence exists.
- `tickets.py --dir <dir> mark 02 in_progress` starts a ticket (refused while a blocker is not done; it prints the
  briefs of the blockers).
- `tickets.py --dir <dir> mark 02 done --brief "<what was produced, commit, numbers; max 8 lines>"` closes it
  (refused while a task is `[ ]` or a blocker is not done); the brief is written under the `-----------` line.

Status values are `todo`, `in_progress`, `done`; the template's `ready-for-agent` corresponds to `todo` with no open
blocker (`READY` in `status`).
