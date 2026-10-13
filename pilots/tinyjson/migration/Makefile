# Written by `tickets.py init` (or `tickets.py makefile`); do not edit. Run: make -C pilots/<name>/migration status
# SCRIPTS: pilots/<name>/migration -> repo root -> skill/c-to-rust/scripts
MIGRATION := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
WS        := $(abspath $(MIGRATION)/..)
SCRIPTS   ?= $(abspath $(MIGRATION)/../../../skill/c-to-rust/scripts)

.PHONY: status tickets

status:   ## where the migration stands: rulebook, tickets, Parts per stage
	@echo "== rulebook: $$(grep -m1 '^Status:' $(MIGRATION)/RULEBOOK.md 2>/dev/null || echo 'no RULEBOOK.md yet')"
	@echo "== tickets"
	@python3 -I $(SCRIPTS)/tickets.py --ws $(WS) status || true
	@echo "== Stage 1 Parts"
	@python3 -I $(SCRIPTS)/workqueue.py --parts $(MIGRATION)/parts.tsv status 2>&1 || true
	@if [ -f $(MIGRATION)/parts-stage2.tsv ]; then echo "== Stage 2 Parts"; \
	  python3 -I $(SCRIPTS)/workqueue.py --parts $(MIGRATION)/parts-stage2.tsv status 2>&1; fi

tickets:  ## every ticket with its tasks: [x] done, [ ] open
	@python3 -I $(SCRIPTS)/tickets.py --ws $(WS) status --tasks
