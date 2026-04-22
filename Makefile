# Pure-library project — no external integration infrastructure.
#
# `test-integ-up` / `test-integ-down` are no-ops: graph-specs-rust runs
# entirely in-process — no Podman, no databases, no network.

.PHONY: test-integ-up test-integ-down graph-specs-check release-prepare

test-integ-up:
	@echo "graph-specs-rust: no integration infra — pure-library project"

test-integ-down:
	@echo "graph-specs-rust: no integration infra — pure-library project"

# Dual-control spec gate — runs the tool against this repo's own source.
# Invoked by the /ship pre-push gate (Study 002-v3 §5.2) and by CI's
# `dogfood` job. Release-mode build, so the binary reflects current HEAD.
graph-specs-check:
	cargo build -p application --release
	./target/release/graph-specs check --specs specs/ --code .

release-prepare:  ## Bump version + changelog (auto|patch|minor|major)
	scripts/release-prepare.sh $(or $(BUMP),auto)
