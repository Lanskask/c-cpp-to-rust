# 09: inventory and depmap skip vendored code; non-library Parts are dropped by a script

**What to build:** `fuzzing/`, `tests/unity` and similar vendored or test-only directories no longer become Parts, and
SKILL.md says how a non-library file (a root `test.c`, examples) is excluded from `parts.tsv`.

**Blocked by:** None (can start immediately)

**Status:** done (2026-10-10)

- [x] `inventory.py` skips `--exclude` globs and a default list (`tests/unity`, `third_party`, `vendor`, `fuzzing`)
- [x] `tier.py` marks root-level test and example files as non-Parts
- [x] cJSON inventory yields only `cJSON` and `cJSON_Utils`
- [x] tests with a fixture tree
