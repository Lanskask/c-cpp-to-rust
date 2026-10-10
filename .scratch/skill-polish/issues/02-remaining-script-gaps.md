# 02: Close the remaining script gaps G6, G13, G17, G19

**What to build:** The deny-rules are verified by a script instead of a manual Edit try, `fmt` runs once per gate,
`tm.py` indexes both stage queues, and `dead.txt` says it under-approximates.

**Blocked by:** None (can start immediately)

**Status:** todo

- [ ] G19: a `deny_check.py` (or hook) exits 1 when an Edit of a protected path would be allowed
- [ ] G6: `fmt` once per gate; `unsafe` ignores strings and block comments
- [ ] G13: `tm.py` reads every `migration/parts*.tsv` and tags documents with the stage
- [ ] G17: `dead.txt` header states the call graph is name-based and an under-approximation
- [ ] each has a test
