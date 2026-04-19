# Pure-library project — no external integration infrastructure.
#
# `test-integ-up` / `test-integ-down` exist solely to satisfy the
# agent-zero ship preflight contract (Makefile must declare integ
# targets). They are no-ops because graph-specs-rust runs entirely
# in-process — no Podman, no databases, no network.

.PHONY: test-integ-up test-integ-down

test-integ-up:
	@echo "graph-specs-rust: no integration infra — pure-library project"

test-integ-down:
	@echo "graph-specs-rust: no integration infra — pure-library project"
