# Odometer Development Makefile
# 
# Key targets:
#   make install-tools      - Install development dependencies
#   make ci-local           - Local CI with all checks
#   make release-validation - Complete release validation
#   make fixtures           - Generate all test fixtures  
#   make test               - Run unit tests (no fixtures)
#   make test-integration   - Run integration tests with fixtures
#   make test-all           - Run all tests

# =============================================================================
# Development Setup
# =============================================================================

.PHONY: install-tools
install-tools:
	@echo "Installing development tools..."
	@which cargo >/dev/null || (echo "❌ cargo not found. Install Rust: https://rustup.rs/" && exit 1)
	@echo "✅ cargo found"
	@cargo --version
	@which npm >/dev/null || (echo "❌ npm not found. Install Node.js: https://nodejs.org/" && exit 1)
	@echo "✅ npm found"
	@npm --version
	@cargo install cargo-workspaces --force
	@echo "✅ Development tools ready"

# =============================================================================
# CI Targets
# =============================================================================

.PHONY: ci
ci:
	@echo "🚀 Running comprehensive CI validation..."
	$(MAKE) fmt-check
	$(MAKE) check
	$(MAKE) test-all
	@echo "All binaries clippy:"
	cargo clippy --all-targets --all-features -- -D warnings
	@echo "Publish dry run:"
	cargo publish --dry-run --allow-dirty
	@echo "✅ CI passed"

.PHONY: ci-local
ci-local: ci
	@echo "✅ Local CI passed"

# Release validation - comprehensive checks before publishing
.PHONY: release-validation
release-validation:
	@echo "🚀 Running release validation..."
	@echo "Verifying tag matches Cargo.toml version..."
	@if [ -n "$$TAG_VERSION" ] && [ -n "$$(grep '^version = ' Cargo.toml | sed 's/version = \"(.*)\"/\\1/')" ]; then \
		CARGO_VERSION=$$(grep '^version = ' Cargo.toml | sed 's/version = \"(.*)\"/\\1/'); \
		if [ "$$TAG_VERSION" != "$$CARGO_VERSION" ]; then \
			echo "❌ Tag version $$TAG_VERSION doesn't match Cargo.toml version $$CARGO_VERSION"; \
			exit 1; \
		fi; \
		echo "✅ Tag version matches Cargo.toml version: $$TAG_VERSION"; \
	fi
	$(MAKE) ci
	@echo "✅ Release validation passed"

# Publish to crates.io (requires CARGO_REGISTRY_TOKEN)
.PHONY: publish
publish:
	@echo "📦 Publishing to crates.io..."
	cargo publish --token $$CARGO_REGISTRY_TOKEN
	@echo "✅ Published to crates.io"

# Dogfooding - Use our own tool for releases! 🎯
.PHONY: release-patch release-minor release-major
release-patch:
	@./scripts/release.sh patch

release-minor:
	@./scripts/release.sh minor

release-major:
	@./scripts/release.sh major

# Install odo locally for dogfooding
.PHONY: install-local
install-local:
	@echo "📦 Installing odo locally..."
	cargo install --path . --force
	@echo "✅ odo installed! Try: odo show"

# =============================================================================
# Test Fixtures (Git-ignored, generated on demand)
# =============================================================================

FIXTURES_DIR = tests/fixtures

.PHONY: fixtures.clean
fixtures.clean:
	@rm -rf $(FIXTURES_DIR)

.PHONY: fixtures.dir
fixtures.dir:
	@mkdir -p $(FIXTURES_DIR)

.PHONY: fixtures
fixtures: fixtures.node fixtures.rust

.PHONY: fixtures.node
fixtures.node:
	$(MAKE) fixtures.node.basic-node-workspace

.PHONY: fixtures.rust
fixtures.rust:
	$(MAKE) fixtures.rust.basic-rust-workspace

.PHONY: fixtures.rust.basic-rust-workspace
fixtures.rust.basic-rust-workspace: fixtures.dir
	@rm -fr $(FIXTURES_DIR)/basic-rust-workspace && mkdir -p $(FIXTURES_DIR)/basic-rust-workspace
	@cargo new --vcs none --frozen --bin $(FIXTURES_DIR)/basic-rust-workspace/bin1
	@cargo new --vcs none --frozen --bin $(FIXTURES_DIR)/basic-rust-workspace/bin2
	@cargo new --vcs none --frozen --lib $(FIXTURES_DIR)/basic-rust-workspace/lib1
	@cargo new --vcs none --frozen --lib $(FIXTURES_DIR)/basic-rust-workspace/lib2
	@cargo workspaces init $(FIXTURES_DIR)/basic-rust-workspace

.PHONY: fixtures.node.basic-node-workspace
.PHONY: fixtures.node.basic-node-workspace
fixtures.node.basic-node-workspace: fixtures.dir
	@rm -fr $(FIXTURES_DIR)/basic-node-workspace && mkdir -p $(FIXTURES_DIR)/basic-node-workspace
	@cd $(FIXTURES_DIR)/basic-node-workspace && npm init -y
	@mkdir -p $(FIXTURES_DIR)/basic-node-workspace/bin1 $(FIXTURES_DIR)/basic-node-workspace/bin2 $(FIXTURES_DIR)/basic-node-workspace/lib1 $(FIXTURES_DIR)/basic-node-workspace/lib2
	@cd $(FIXTURES_DIR)/basic-node-workspace/bin1 && npm init -y
	@cd $(FIXTURES_DIR)/basic-node-workspace/bin2 && npm init -y
	@cd $(FIXTURES_DIR)/basic-node-workspace/lib1 && npm init -y
	@cd $(FIXTURES_DIR)/basic-node-workspace/lib2 && npm init -y
	@cd $(FIXTURES_DIR)/basic-node-workspace && jq '.workspaces = ["bin1", "bin2", "lib1", "lib2"]' package.json > package.json.tmp && mv package.json.tmp package.json

# =============================================================================
# Testing
# =============================================================================

.PHONY: test
test: build
	@echo "Running unit tests (no fixtures)..."
	cargo test --lib

.PHONY: test-integration
test-integration: build fixtures.clean fixtures.dir
	@echo "Running integration tests with fresh fixtures..."
	cargo test --features fixture-tests

.PHONY: test-all  
test-all: build test test-integration

# =============================================================================
# Code Quality
# =============================================================================

.PHONY: fmt
fmt:
	@echo "🎨 Formatting code..."
	cargo fmt --all
	@echo "✅ Code formatted"

.PHONY: fmt-check
fmt-check:
	@echo "🎨 Checking code formatting..."
	cargo fmt --all -- --check
	@echo "✅ Code formatting OK"

# =============================================================================
# Development
# =============================================================================

.PHONY: check
check:
	@echo "🔍 Checking workspace..."
	cargo check --workspace
	@echo "✅ Workspace check passed"

.PHONY: build
build:
	@echo "Building odo binary..."
	cargo build --bin odo

.PHONY: clean
clean: fixtures.clean
	cargo clean

# Default target
.PHONY: help
help:
	@echo "Odometer Development Commands:"
	@echo ""
	@echo "Setup:"
	@echo "  make install-tools     Install development dependencies"
	@echo ""
	@echo "Testing:"
	@echo "  make test             Run unit tests (fast)"
	@echo "  make test-integration Run integration walkthrough with fixtures"
	@echo "  make test-all         Run all tests"
	@echo ""
	@echo "CI & Quality:"
	@echo "  make ci               Comprehensive CI validation (recommended)"
	@echo "  make ci-local         Local CI (same as 'ci')"
	@echo "  make fmt              Format code"
	@echo "  make fmt-check        Check code formatting"
	@echo ""
	@echo "Release:"
	@echo "  make release-validation  Complete release validation"
	@echo "  make publish            Publish to crates.io"
	@echo ""
	@echo "Dogfooding (using our own odo tool!):"
	@echo "  make install-local      Install odo locally"
	@echo "  make release-patch      Release patch version (bug fixes)"
	@echo "  make release-minor      Release minor version (new features)"
	@echo "  make release-major      Release major version (breaking changes)"
	@echo ""
	@echo "Fixtures:"
	@echo "  make fixtures         Generate all test fixtures"
	@echo "  make fixtures.clean   Remove all fixtures"
	@echo ""
	@echo "Development:"
	@echo "  make check            Check code without building"
	@echo "  make build            Build project"
	@echo "  make clean            Clean build artifacts and fixtures"

.DEFAULT_GOAL := help 