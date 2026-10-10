# 03: Rulebook stress test (cjson)

**What to build:** The hardest Parts are translated twice with the Stage 1 prompt and the differences become rules.

**Blocked by:** 01, 02

**Status:** done

**Skipped (user decision, 2026-10-10):** the Stage 1 engine is c2rust, so two independent LLM
translations would only compare polished output. Instead, the polish prompt gets a dry run on `cJSON_Utils` inside
ticket 04. The boxes below stay unchecked: the work was not done.

- [ ] each difference is a rule proposal in RUN-NOTES.md
- [ ] accepted proposals applied by Opus
- [ ] a same-model rerun agrees, or the remaining differences are classified
- [ ] status line says stress-tested
