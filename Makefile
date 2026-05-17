.PHONY: help run test check lint ci fmt search clean run-console

help:
	@echo "Local Operator build targets:"
	@echo "  make run            - Run the Rust API server (cargo run)"
	@echo "  make test           - Run backend test suite"
	@echo "  make check          - Run cargo check"
	@echo "  make lint           - Run formatting check (cargo fmt -- --check)"
	@echo "  make fmt            - Format backend sources"
	@echo "  make ci             - Run check + format-check + test"
	@echo "  make search q=term  - Search source files (rg preferred, grep fallback)"
	@echo "  make run-console    - Run the React operator console"

run:
	cargo run

test:
	cargo test

check:
	cargo check

lint:
	cargo fmt -- --check

fmt:
	cargo fmt

ci: check lint test

run-console:
	cd operator-console && npm run dev

search:
	@if [ -z "$(q)" ]; then \
		echo "Usage: make search q=term"; \
		exit 1; \
	fi
	@if command -v rg >/dev/null 2>&1; then \
		rg -n "$(q)" src operator-console; \
	else \
		grep -RIn "$(q)" src operator-console | head -n 200; \
	fi

clean:
	cargo clean
