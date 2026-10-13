# Written by `tickets.py init` (or `tickets.py makefile`); everything the Skill creates lives next to this file.
# Usage: make -C <WORK_DIR>/migration help      (WORK_DIR, SCRIPTS and CRATE can be overridden on the command line)
SCRIPTS   ?= @SCRIPTS@
WORK_DIR  ?= @WORK_DIR@
SOURCE    ?= @SOURCE@
CRATE     ?= @CRATE@
MIGRATION := $(WORK_DIR)/migration
PY        := python3 -I $(SCRIPTS)
STAGE     ?= 1
BATCH     ?= 1
LANG_     ?= @LANG@
PARTS     := $(MIGRATION)/parts$(if $(filter 2,$(STAGE)),-stage2).tsv
# gate.mk is written by ticket 02 (the Judge): GATE_ARGS = --features "" --harness-cmd "..." --also migration/rust/tests/adapter
-include $(MIGRATION)/gate.mk

.PHONY: help commit status tickets next verify gate snapshot inventory depmap tier parts refbuild run run-go spend-now spend

help:     ## list the targets
	@grep -E '^[a-z-]+:.*##' $(firstword $(MAKEFILE_LIST)) | sed 's/:.*## /\t/'

commit:   ## commit everything in migration/ (its own git repository): make commit MSG="what changed"
	@git -C $(MIGRATION) add -A && git -C $(MIGRATION) commit -q -m "$(or $(MSG),migration checkpoint)" && git -C $(MIGRATION) log --oneline -1 || true

status:   ## where the migration stands: rulebook, tickets, Parts per stage
	@echo "== rulebook: $$(grep -m1 '^Status:' $(MIGRATION)/RULEBOOK.md 2>/dev/null || echo 'no RULEBOOK.md yet')"
	@echo "== tickets"
	@$(PY)/tickets.py --ws $(WORK_DIR) status || true
	@echo "== Stage 1 Parts"
	@$(PY)/workqueue.py --parts $(MIGRATION)/parts.tsv status 2>&1 || true
	@if [ -f $(MIGRATION)/parts-stage2.tsv ]; then echo "== Stage 2 Parts"; \
	  $(PY)/workqueue.py --parts $(MIGRATION)/parts-stage2.tsv status 2>&1; fi

tickets:  ## every ticket with its tasks: [x] done, [ ] open
	@$(PY)/tickets.py --ws $(WORK_DIR) status --tasks

next:     ## the next Parts of STAGE that can start: make next [STAGE=1] [BATCH=3]
	@$(PY)/workqueue.py --parts $(PARTS) next --batch $(BATCH)

verify:   ## check that every `done` Part has its commit, gate record and output: make verify [STAGE=1]
	@$(PY)/workqueue.py --parts $(PARTS) verify

gate:     ## run the Judge on one Part: make gate PART=<id> [STAGE=1] [MIN_TESTS=0]
	@test -n "$(PART)" || { echo "usage: make gate PART=<part id> [STAGE=1]"; exit 2; }
	$(PY)/gate.py $(PART) --crate $(CRATE) --stage $(STAGE) --min-tests $(or $(MIN_TESTS),0) --parts $(PARTS) $(GATE_ARGS)

snapshot: ## record HEAD and the failing baseline before editing a Part: make snapshot PART=<id> [STAGE=1]
	@test -n "$(PART)" || { echo "usage: make snapshot PART=<part id> [STAGE=1]"; exit 2; }
	$(PY)/gate.py $(PART) --crate $(CRATE) --stage $(STAGE) --parts $(PARTS) --snapshot $(GATE_ARGS)

inventory: ## symbols and files of the original: make inventory [LANG_=cpp] [INV_ARGS="--exclude GLOB"]
	$(PY)/inventory.py $(SOURCE) --lang $(LANG_) --out $(MIGRATION)/inventory.tsv $(INV_ARGS)

depmap:   ## dependency edges and leaf-first order from inventory.tsv
	$(PY)/depmap.py $(MIGRATION)/inventory.tsv --source-root $(SOURCE) --out-dir $(MIGRATION)

tier:     ## parts.tsv from inventory.tsv and order.txt
	$(PY)/tier.py $(MIGRATION)/inventory.tsv $(MIGRATION)/order.txt --out $(MIGRATION)/parts.tsv

parts: inventory depmap tier  ## inventory + depmap + tier

refbuild: ## reference build of the original (judge.md §8): make refbuild TEST=xmltest.cpp SOURCES="tinyxml2.cpp" [RES=resources]
	@test -n "$(TEST)" || { echo "usage: make refbuild TEST=<file> SOURCES=\"<files>\" [RES=<dir>]"; exit 2; }
	$(PY)/refbuild.py $(WORK_DIR) --test $(TEST) $(foreach s,$(SOURCES),--source $(s)) $(foreach r,$(RES),--resource $(r))

run:      ## dry run of the headless driver: prints the plan, spends nothing: make run [STAGE=1] [RUN_ARGS="..."]
	$(PY)/run.py $(WORK_DIR) --stage $(STAGE) $(RUN_ARGS)

run-go:   ## the same, spending tokens: make run-go RUN_ARGS="--deviations policy --max-attempts 2 --budget-output-tokens 300000"
	$(PY)/run.py $(WORK_DIR) --stage $(STAGE) --go $(RUN_ARGS)

spend-now: ## print a UTC timestamp to mark the start of a phase
	@$(PY)/spend.py now

spend:    ## log spend of a phase: make spend PHASE=E1 SINCE=<timestamp> NOTE="..."
	$(PY)/spend.py log --phase "$(PHASE)" --since "$(SINCE)" --note "$(NOTE)"
