# Odometer Development Makefile
# 
# Key targets:
#   make install-tools    - Install development dependencies
#   make ci-docker-full   - Complete CI in Docker (matches GitHub Actions)
#   make ci-local         - Local CI with all checks
#   make release-validation - Complete release validation
#   make fixtures         - Generate all test fixtures  
#   make test            - Run unit tests (no fixtures)
#   make test-integration - Run integration tests with fixtures
#   make test-all        - Run all tests

# =============================================================================
# Development Setup
# =============================================================================

.PHONY: install-tools
install-tools:
	@echo "Installing development tools..."
	@which cargo >/dev/null || (echo "❌ cargo not found. Install Rust: https://rustup.rs/" && exit 1)
	@echo "✅ cargo found"
	@cargo --version
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

# Docker-based CI targets (matches CI environment exactly)
.PHONY: ci-docker
ci-docker:
	@echo "🐳 Running CI in Docker container..."
	@docker pull rust:latest > /dev/null 2>&1 || true
	@mkdir -p ~/.cargo
	docker run --rm \
		-v $$(pwd):/workspace \
		-v ~/.cargo:/root/.cargo \
		-w /workspace \
		rust:latest sh -c "rustup component add clippy rustfmt && rm -f Cargo.lock && make ci"



# Release validation - comprehensive checks before publishing
.PHONY: release-validation
release-validation:
	@echo "🚀 Running release validation..."
	@echo "Verifying tag matches Cargo.toml version..."
	@if [ -n "$$TAG_VERSION" ] && [ -n "$$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')" ]; then \
		CARGO_VERSION=$$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/'); \
		if [ "$$TAG_VERSION" != "$$CARGO_VERSION" ]; then \
			echo "❌ Tag version $$TAG_VERSION doesn't match Cargo.toml version $$CARGO_VERSION"; \
			exit 1; \
		fi; \
		echo "✅ Tag version matches Cargo.toml version: $$TAG_VERSION"; \
	fi
	$(MAKE) ci-docker
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

# Clean all fixtures
.PHONY: clean-fixtures
clean-fixtures:
	rm -rf $(FIXTURES_DIR)

# Single crate fixture
$(FIXTURES_DIR)/single-crate/Cargo.toml:
	@echo "Creating single-crate fixture..."
	mkdir -p $(FIXTURES_DIR)
	cd $(FIXTURES_DIR) && cargo new single-crate
	@echo "✅ single-crate fixture ready"

# Simple workspace fixture  
$(FIXTURES_DIR)/workspace-simple/Cargo.toml:
	@echo "Creating workspace-simple base..."
	mkdir -p $(FIXTURES_DIR)
	cd $(FIXTURES_DIR) && cargo new --name workspace-simple workspace-simple

$(FIXTURES_DIR)/workspace-simple/lib1/Cargo.toml: $(FIXTURES_DIR)/workspace-simple/Cargo.toml
	@echo "Adding lib1 to workspace-simple..."
	cd $(FIXTURES_DIR)/workspace-simple && cargo new --lib lib1

$(FIXTURES_DIR)/workspace-simple/lib2/Cargo.toml: $(FIXTURES_DIR)/workspace-simple/Cargo.toml
	@echo "Adding lib2 to workspace-simple..."
	cd $(FIXTURES_DIR)/workspace-simple && cargo new --lib lib2

$(FIXTURES_DIR)/workspace-simple/.configured: $(FIXTURES_DIR)/workspace-simple/lib1/Cargo.toml $(FIXTURES_DIR)/workspace-simple/lib2/Cargo.toml
	@echo "Configuring workspace-simple..."
	echo '' >> $(FIXTURES_DIR)/workspace-simple/Cargo.toml
	echo '[workspace]' >> $(FIXTURES_DIR)/workspace-simple/Cargo.toml  
	echo 'members = ["lib1", "lib2"]' >> $(FIXTURES_DIR)/workspace-simple/Cargo.toml
	touch $(FIXTURES_DIR)/workspace-simple/.configured
	@echo "✅ workspace-simple fixture ready"

# Workspace with inheritance fixture
$(FIXTURES_DIR)/workspace-inheritance/Cargo.toml:
	@echo "Creating workspace-inheritance base..."
	mkdir -p $(FIXTURES_DIR)
	cd $(FIXTURES_DIR) && cargo new --name workspace-root workspace-inheritance

$(FIXTURES_DIR)/workspace-inheritance/member1/Cargo.toml: $(FIXTURES_DIR)/workspace-inheritance/Cargo.toml
	@echo "Adding member1 to workspace-inheritance..."
	cd $(FIXTURES_DIR)/workspace-inheritance && cargo new --lib member1

$(FIXTURES_DIR)/workspace-inheritance/member2/Cargo.toml: $(FIXTURES_DIR)/workspace-inheritance/Cargo.toml
	@echo "Adding member2 to workspace-inheritance..."
	cd $(FIXTURES_DIR)/workspace-inheritance && cargo new --lib member2

$(FIXTURES_DIR)/workspace-inheritance/.configured: $(FIXTURES_DIR)/workspace-inheritance/member1/Cargo.toml $(FIXTURES_DIR)/workspace-inheritance/member2/Cargo.toml
	@echo "Configuring workspace-inheritance..."
	# Add workspace section to root
	echo '' >> $(FIXTURES_DIR)/workspace-inheritance/Cargo.toml
	echo '[workspace]' >> $(FIXTURES_DIR)/workspace-inheritance/Cargo.toml
	echo 'members = ["member1", "member2"]' >> $(FIXTURES_DIR)/workspace-inheritance/Cargo.toml
	# Configure member1 to use workspace inheritance
	perl -i -pe 's/version = "0.1.0"/version = { workspace = true }/' $(FIXTURES_DIR)/workspace-inheritance/member1/Cargo.toml
	# member2 keeps its own version for testing mixed scenarios
	touch $(FIXTURES_DIR)/workspace-inheritance/.configured
	@echo "✅ workspace-inheritance fixture ready"

# High-level fixture targets
.PHONY: single-crate workspace-simple workspace-inheritance fixtures
single-crate: $(FIXTURES_DIR)/single-crate/Cargo.toml
workspace-simple: $(FIXTURES_DIR)/workspace-simple/.configured  
workspace-inheritance: $(FIXTURES_DIR)/workspace-inheritance/.configured
fixtures: single-crate workspace-simple workspace-inheritance

# =============================================================================
# Testing
# =============================================================================

.PHONY: test
test:
	@echo "Running unit tests (no fixtures)..."
	cargo test --lib

.PHONY: test-integration
test-integration: build clean-fixtures fixtures
	@echo "Running integration tests with fresh fixtures..."
	ODO_BINARY=$(shell pwd)/target/debug/odo cargo test --features fixture-tests

.PHONY: test-all  
test-all: test test-integration

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
clean: clean-fixtures
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
	@echo "  make ci-docker        CI in Docker (matches GitHub Actions)"
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
	@echo "  make single-crate     Generate single-crate fixture"
	@echo "  make workspace-simple Generate simple workspace fixture"
	@echo "  make clean-fixtures   Remove all fixtures"
	@echo ""
	@echo "Development:"
	@echo "  make check            Check code without building"
	@echo "  make build            Build project"
	@echo "  make clean            Clean build artifacts and fixtures"

.DEFAULT_GOAL := help 