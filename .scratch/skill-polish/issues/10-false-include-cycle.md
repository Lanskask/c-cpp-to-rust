# 10: Check the cJSON ↔ cJSON_Utils cycle

**What to build:** `depmap.py` reports a cycle between cJSON and cJSON_Utils; find out whether the edge is real (include
or call) or an artefact of name-based matching, and fix the script if it is false.

**Blocked by:** 09

**Status:** done (2026-10-10)

- [x] the edge that closes the cycle is named with file and line
- [x] if false: `depmap.py` no longer reports it and a test covers the case
- [x] if real: the cycle handling (translate together) is shown to work on it
