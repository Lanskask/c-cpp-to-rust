# 01: Mapping (Stage 0) (cjson)

**What to build:** The Source project is inventoried, its dependencies mapped, Parts tiered, and a RULEBOOK drafted
whose Deviations table the user can approve.

**Blocked by:** None (can start immediately)

**Status:** done

## Tasks

- [x] Inventory the Source and map its dependencies
  - `make spend-now`, note the timestamp for the SPEND row
  - `inventory.py <ws>/src --lang c|cpp --out <ws>/migration/inventory.tsv` (add `--exclude GLOB` for non-library
    directories; tests and examples are never Parts)
  - `depmap.py <ws>/migration/inventory.tsv --source-root <ws>/src --out-dir <ws>/migration`
- [x] Tier the Parts
  - `tier.py <ws>/migration/inventory.tsv <ws>/migration/order.txt --out <ws>/migration/parts.tsv`
- [x] Draft RULEBOOK.md (Opus)
  - start from `references/rulebook-template.md` and `references/idiom-table-c.md`
  - every `config-macro:` tag gets a §3 row, every UB or platform behaviour a §8 Deviation row with status `proposed`
- [x] Get the Deviations decided by the user
  - show the §8 table and stop; the user marks each row `approved` or `vetoed`
  - set the rulebook status line to `deviations-approved`

## Acceptance

- [x] inventory.tsv, edges.tsv, order.txt, parts.tsv exist
- [x] every config-macro has a RULEBOOK §3 row
- [x] every UB or platform behaviour has a §8 Deviation row
- [x] the user approved or vetoed every Deviation

-----------
## Done

- 2 Parts (cJSON, cJSON_Utils), no cycle; inventory, edges, order, parts.tsv regenerated (f9ac270)
- RULEBOOK by Opus: 18 config-macro rows, D1-D20, Q1-Q24 (03ec1a9); Stage 1 engine c2rust (f1ec01c)
- User approved Deviations D1-D20; Q15/Q20 proved by running (ed568b9)
