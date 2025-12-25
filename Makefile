.PHONY: help build build-release install test check clean run fmt clippy doc dev setup

# Default target
.DEFAULT_GOAL := help

## help: Show this help message
help:
	@echo "Available targets:"
	@sed -n 's/^##//p' ${MAKEFILE_LIST} | column -t -s ':' | sed -e 's/^/ /'

## build: Build the project in debug mode
build:
	cargo build

## build-release: Build optimized release binary
build-release:
	cargo build --release

## install: Install the binary to ~/.cargo/bin
install:
	cargo install --path .

## test: Run all tests
test:
	cargo test

## test-verbose: Run all tests with verbose output
test-verbose:
	cargo test -- --nocapture

## check: Check if the code compiles (fast, no build)
check:
	cargo check

## clean: Remove build artifacts
clean:
	cargo clean

## run: Run the application (default: sync command)
run:
	cargo run

## run-status: Show current Harvest timer status
run-status:
	cargo run -- status

## run-stop: Stop the currently running timer
run-stop:
	cargo run -- stop

## run-generate: Run AI generate command (requires summary argument)
run-generate:
	@echo "Usage: make run-generate SUMMARY='your work summary here'"
	@echo "Example: make run-generate SUMMARY='Fixed bugs, reviewed PRs, team meeting'"

## fmt: Format code using rustfmt
fmt:
	cargo fmt

## fmt-check: Check code formatting without modifying files
fmt-check:
	cargo fmt -- --check

## clippy: Run clippy linter for code quality checks
clippy:
	cargo clippy -- -D warnings

## clippy-fix: Run clippy and automatically apply fixes
clippy-fix:
	cargo clippy --fix

## doc: Generate and open project documentation
doc:
	cargo doc --open

## dev: Run in development mode with hot reload (requires cargo-watch)
dev:
	@which cargo-watch > /dev/null || (echo "cargo-watch not installed. Run: cargo install cargo-watch" && exit 1)
	cargo watch -x check -x test -x run

## setup: Setup development environment
setup:
	@echo "Installing development dependencies..."
	rustup component add rustfmt clippy
	@echo "Development environment ready!"

## config-init: Initialize configuration file
config-init:
	cargo run -- config init

## config-show: Display current configuration
config-show:
	cargo run -- config show

## config-validate: Validate configuration file
config-validate:
	cargo run -- config validate

## systemd-install: Install systemd user timer
systemd-install:
	mkdir -p ~/.config/systemd/user
	cp systemd/harjira.service ~/.config/systemd/user/
	cp systemd/harjira.timer ~/.config/systemd/user/
	systemctl --user daemon-reload
	systemctl --user enable harjira.timer
	systemctl --user start harjira.timer
	@echo "Systemd timer installed and started"
	@echo "Check status: systemctl --user status harjira.timer"

## systemd-uninstall: Uninstall systemd user timer
systemd-uninstall:
	systemctl --user stop harjira.timer || true
	systemctl --user disable harjira.timer || true
	rm -f ~/.config/systemd/user/harjira.service
	rm -f ~/.config/systemd/user/harjira.timer
	systemctl --user daemon-reload
	@echo "Systemd timer uninstalled"

## systemd-status: Show systemd timer status
systemd-status:
	systemctl --user status harjira.timer

## systemd-logs: Show systemd service logs
systemd-logs:
	journalctl --user -u harjira.service -f

## dry-run: Run sync command in dry-run mode
dry-run:
	cargo run -- --dry-run sync

## dry-run-generate: Run generate command in dry-run mode
dry-run-generate:
	@echo "Usage: make dry-run-generate SUMMARY='your work summary here'"
	@if [ -z "$(SUMMARY)" ]; then \
		echo "Error: SUMMARY variable is required"; \
		echo "Example: make dry-run-generate SUMMARY='Fixed bugs, reviewed PRs'"; \
		exit 1; \
	fi
	cargo run -- --dry-run generate "$(SUMMARY)"

## bench: Run benchmarks (if any)
bench:
	cargo bench

## release: Build release binary and show location
release:
	cargo build --release
	@echo ""
	@echo "Release binary built at:"
	@echo "  $$(pwd)/target/release/harjira"
	@echo ""
	@echo "Install with: make install"
