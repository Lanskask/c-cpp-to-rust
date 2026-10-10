# 11: Guard survives a moved cwd; Pilot scripts have a home

**What to build:** the hook resolves the repo from `$CLAUDE_PROJECT_DIR`, so a plain `cd` inside a Pilot neither
breaks every tool call nor bypasses the guard; Pilot analysis scripts go to `<ws>/experiments/`.

**Blocked by:** 08

**Status:** done (2026-10-10; seen in the first cJSON Translation session)

- [x] hook command uses an absolute path built from `$CLAUDE_PROJECT_DIR`
- [x] guard tests cover a cwd inside the Pilot (Edit with `../`, Bash with `../`)
- [x] SKILL.md rules: no cwd inside `<ws>`, scripts to `<ws>/experiments/`
- [ ] hook seen working after a `cd` in a real Translation session
