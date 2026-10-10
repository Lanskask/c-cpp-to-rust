# 03: `spend.py log` refuses an overlapping window

**What to build:** Logging spend can no longer double-count: a `SINCE` earlier than the previous row's end is refused
with a message, and an identical row is not appended twice.

**Blocked by:** None (can start immediately)

**Status:** todo

- [ ] `log` with `--since` before the last row's timestamp exits 1 unless `--force`
- [ ] a byte-identical row is never appended twice
- [ ] `SPEND.md` duplicate `E3-stage2 | 05:30` row is removed with a correction note in WORK-LOG
- [ ] test for both guards
