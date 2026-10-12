# 01: Mapping (Stage 0) (tinyjson)

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

- Inventory (AST engine): 92 symbols, 187 edges, 0 cycles, 1 Part src/tinyjson (judgment).
- facts.tsv 65 rows. RULEBOOK by Opus: CPP1-CPP36 (§2.1, proposed), §8 D1-D16.
- User approved all Deviations 2026-10-11; status line deviations-approved.
- D3 kept bug (U+0000, driver at -O0). D5 emulate the libstdc++ chunk rule (measure in 02).
- Skill bugs 1-5 in RUN-NOTES, requests sent to the Skill session.
- Commits ad9510e, 1fb7cd4, ca63f10, eaf43a7.
