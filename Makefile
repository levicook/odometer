# Odometer Development Makefile
# 
# Key targets:
#   make install-tools    - Install development dependencies
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
# Development
# =============================================================================

.PHONY: check
check:
	cargo check

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