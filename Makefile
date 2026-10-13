# Everyday commands. See README.md.
SCRIPTS := skill/c-to-rust/scripts

.PHONY: setup test idiom lint typecheck fmt check install-to-tickets spend-now pilot-jsmn pilot-jsmn-snapshot

setup:            ## create the uv environment with dev tools
	uv sync

test:             ## run pytest for the Skill scripts
	uv run pytest -q

idiom:            ## verify the idiom-table rows: compile C and Rust snippets, compare (slow, ~4 min)
	uv run pytest -q tests/idiom --override-ini addopts=""

lint:             ## ruff lint + format check, then type check
	uvx ruff check .
	uvx ruff format --check .
	$(MAKE) typecheck

typecheck:        ## strict type check with ty
	uvx ty check

fmt:              ## auto-format Python
	uvx ruff format .
	uvx ruff check --fix .

check: test lint  ## everything CI would run

install-to-tickets:  ## copy skill/to-tickets (script + SKILL.md section) into the mattpocock to-tickets skill; rerun after a plugin update
	python3 -I skill/to-tickets/install.py

install-skill:    ## install skill/c-to-rust as the global Skill ~/.claude/skills/cpp-to-rust (rerun after a change)
	python3 -I skill/install_global.py

PILOT ?= cjson
tickets:          ## ticket status of a run: make tickets [PILOT=cjson] [TASKS=1 lists every task with [x]/[ ]]
	@python3 -I $(SCRIPTS)/tickets.py --ws pilots/$(PILOT) status $(if $(TASKS),--tasks)

spend-now:        ## print a UTC timestamp to mark the start of a phase
	@python3 -I $(SCRIPTS)/spend.py now

# Log spend for a phase:  make spend PHASE=E1 SINCE=2026-10-09T20:00:00+00:00 NOTE="c2rust on jsmn"
spend:
	python3 -I $(SCRIPTS)/spend.py log --phase "$(PHASE)" --since "$(SINCE)" --note "$(NOTE)"

# Judge for one Pilot: gate the Part over every upstream-tested feature set.
# Usage: make pilot-jsmn [STAGE=1] [PART=jsmn] [MIN_TESTS=16]   (MIN_TESTS: translated tests, Stage 2)
STAGE ?= 1
MIN_TESTS ?= 0
PART ?= jsmn
pilot-jsmn:
	python3 -I $(SCRIPTS)/gate.py $(PART) --crate pilots/jsmn/rust --stage $(STAGE) --min-tests $(MIN_TESTS) --also rust/tests/adapter \
	  --parts pilots/jsmn/migration/parts.tsv \
	  --features "" --features strict --features parent-links --features strict,parent-links \
	  --harness-cmd "python3 -I pilots/jsmn/migration/harness/run.py --stage $(STAGE) {features}"

TJ_PARTS = $(if $(filter 2,$(STAGE)),parts-stage2.tsv,parts.tsv)
# tinyjson (C++ route, one Part, feature matrix "" only). Usage: make pilot-tinyjson [STAGE=1] [MIN_TESTS=0]
pilot-tinyjson:
	python3 -I $(SCRIPTS)/gate.py src/tinyjson --crate pilots/tinyjson/rust --stage $(STAGE) --min-tests $(MIN_TESTS) \
	  --also rust/tests/adapter --parts pilots/tinyjson/migration/$(TJ_PARTS) --features "" \
	  --harness-cmd "python3 -I pilots/tinyjson/migration/harness/run.py --stage $(STAGE) --crate pilots/tinyjson/rust"

pilot-tinyjson-snapshot:  ## record HEAD + baseline before editing the tinyjson Part
	python3 -I $(SCRIPTS)/gate.py src/tinyjson --crate pilots/tinyjson/rust --stage $(STAGE) \
	  --parts pilots/tinyjson/migration/$(TJ_PARTS) --snapshot --also rust/tests/adapter --features "" \
	  --harness-cmd "python3 -I pilots/tinyjson/migration/harness/run.py --stage $(STAGE) --crate pilots/tinyjson/rust"

pilot-tinyjson-test:  ## full cargo test + harness output for the tinyjson Part; STAGE=2 SHIM=1 judges the section 6 API of the shim over the Faithful port, STAGE=2 the real crate
	-cargo test --manifest-path pilots/tinyjson/rust/Cargo.toml --test upstream 2>&1 | grep -v '^warning' | tail -80
	-python3 -I pilots/tinyjson/migration/harness/run.py --stage $(STAGE) $(if $(SHIM),--shim) --crate pilots/tinyjson/rust 2>&1 | tail -80

pilot-tinyjson-fmt:  ## cargo fmt + clippy output for the tinyjson Part
	-cargo fmt --manifest-path pilots/tinyjson/rust/Cargo.toml
	-cargo clippy --manifest-path pilots/tinyjson/rust/Cargo.toml -- -D warnings 2>&1 | grep -v '^warning: unused' | head -80

pilot-jsmn-snapshot:  ## record HEAD + baseline before editing the Part
	python3 -I $(SCRIPTS)/gate.py $(PART) --crate pilots/jsmn/rust --stage $(STAGE) \
	  --parts pilots/jsmn/migration/parts.tsv --snapshot --also rust/tests/adapter \
	  --features "" --features strict --features parent-links --features strict,parent-links \
	  --harness-cmd "python3 -I pilots/jsmn/migration/harness/run.py --stage $(STAGE) {features}"
