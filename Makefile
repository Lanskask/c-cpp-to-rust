# Everyday commands. See README.md.
SCRIPTS := skill/c-to-rust/scripts

.PHONY: setup test lint typecheck fmt check spend-now

setup:            ## create the uv environment with dev tools
	uv sync

test:             ## run pytest for the Skill scripts
	uv run pytest -q

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

spend-now:        ## print a UTC timestamp to mark the start of a phase
	@python3 -I $(SCRIPTS)/spend.py now

# Log spend for a phase:  make spend PHASE=E1 SINCE=2026-10-09T20:00:00+00:00 NOTE="c2rust on jsmn"
spend:
	python3 -I $(SCRIPTS)/spend.py log --phase "$(PHASE)" --since "$(SINCE)" --note "$(NOTE)"
