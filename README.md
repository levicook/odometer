# Odometer 🚀

A workspace version management tool that keeps package versions synchronized across projects.

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![Tests](https://img.shields.io/badge/tests-passing-green.svg)](https://github.com/levicook/odometer)

## Overview

Odometer provides intuitive commands to manage versions across project workspaces, with precise control over which packages get updated. Whether you need lockstep versioning for coordinated releases or independent versioning for different packages, odometer has you covered.

**Currently supports:** Rust/Cargo workspaces and Node.js/npm workspaces  
**Planned support:** Python/pip and other package ecosystems

### Key Features

- 🎯 **Precise Package Selection** - Target specific packages, all workspace members, or just the root
- 🔄 **Flexible Version Strategies** - Independent versioning or lockstep synchronization
- 🛡️ **Safe Defaults** - Operations target workspace root only unless explicitly specified
- 📊 **Clear Inspection** - See current versions and validate version fields
- 🏗️ **Workspace Inheritance Support** - Handles `version = { workspace = true }` (Cargo) and `workspace:*` (Node.js)
- ⚡ **Fast & Reliable** - Written in Rust with comprehensive test coverage

## Installation

### From crates.io (Recommended)

```bash
# Install latest stable release
cargo install odometer

# Or install specific version
cargo install odometer@0.3.1
```

### From Source (Development)

```bash
git clone https://github.com/levicook/odometer.git
cd odometer
cargo install --path .
```

### Verify Installation

```bash
odo --version
# Output: odometer 0.3.1
```

### Multiple Binary Names

Odometer provides multiple binary names for convenience:

```bash
odo --help          # Short form (recommended)
odometer --help     # Full name
cargo odo --help    # Cargo subcommand style
cargo odometer --help
```

## Quick Start

```bash
# See current versions
odo show

# Bump workspace root patch version
odo roll patch

# Bump all workspace members independently
odo roll --workspace patch

# Set all crates to same version (lockstep)
odo sync 1.0.0

# Validate all version fields
odo lint
```

## Commands

### `odo show` - Display Versions

Shows current versions for all workspace members:

```bash
$ odo show
workspace-root 1.0.0
lib1 0.5.2
lib2 0.3.1
```

### `odo roll` - Increment Versions

Increment versions with precise control:

```bash
# Bump workspace root only (safe default)
odo roll patch              # 1.0.0 → 1.0.1
odo roll minor              # 1.0.1 → 1.1.0
odo roll major              # 1.1.0 → 2.0.0

# Custom increments
odo roll patch 5            # 2.0.0 → 2.0.5
odo roll patch -2           # 2.0.5 → 2.0.3

# Target all workspace members independently
odo roll --workspace patch  # Each crate's patch version increments

# Target specific packages
odo roll --package lib1 minor
odo roll -p lib1 -p lib2 patch
```

### `odo set` - Set Specific Versions

Set exact versions for packages:

```bash
# Set workspace root to specific version
odo set 2.1.4

# Set specific packages
odo set 1.0.0 --package lib1
odo set 2.0.0 --workspace    # Set all workspace members
```

### `odo sync` - Lockstep Synchronization

Set ALL workspace members to the same version:

```bash
# Synchronize everything to 1.0.0
odo sync 1.0.0

# After sync, all crates have identical versions
$ odo show
workspace-root 1.0.0
lib1 1.0.0
lib2 1.0.0
```

### `odo lint` - Validate Versions

Check for missing or malformed version fields:

```bash
$ odo lint
✅ All workspace versions are valid

# Or with errors:
$ odo lint
❌ lib1: Invalid version 'not-a-version': unexpected character 'n' while parsing major version number
```

## Package Selection

Odometer uses cargo-style package selection for precise control:

| Flag              | Description                         | Example                                     |
| ----------------- | ----------------------------------- | ------------------------------------------- |
| _(default)_       | Workspace root only                 | `odo roll patch`                            |
| `-p, --package`   | Specific package(s)                 | `odo roll patch -p lib1`                    |
| `-w, --workspace` | All workspace members               | `odo roll --workspace patch`                |
| `--exclude`       | Exclude packages from `--workspace` | `odo roll --workspace --exclude lib1 patch` |

### Example Workflows

#### Coordinated Release Workflow

```bash
# Get everything synchronized first
odo sync 1.0.0
odo show                    # Verify all at 1.0.0

# Now bulk operations work predictably
odo roll --workspace minor  # All: 1.0.0 → 1.1.0
odo lint                    # Validate everything
```

#### Independent Development Workflow

```bash
# Work on different features in different crates
odo roll --package app minor     # App gets new feature: 1.0.0 → 1.1.0
odo roll --package utils patch   # Utils gets bugfix: 0.5.0 → 0.5.1
odo show                         # See current state

# Bulk patch release when ready
odo roll --workspace patch       # Each crate gets patch bump
```

#### Preparing Major Release

```bash
# Review current state
odo show
odo lint

# Synchronize for coordinated release
odo sync 2.0.0
odo show                    # Confirm all at 2.0.0
```

## Workspace Support

Odometer properly handles:

- **Workspace roots** with `[workspace]` sections (Cargo) or `workspaces` field (Node.js)
- **Member packages** in subdirectories
- **Workspace inheritance**:
  - Cargo: `version = { workspace = true }`
  - Node.js: `"version": "workspace:*"` or `"version": "workspace:~"`
- **Mixed scenarios** (some packages inherit, others don't)
- **Single package projects** (no workspace)
- **Mixed ecosystems** (Rust and Node.js packages in the same workspace)

### Node.js Workspace Example

```bash
# Initialize a Node.js workspace
mkdir my-workspace && cd my-workspace
npm init -y
# Add workspaces to package.json
echo '{"workspaces": ["packages/*"]}' > package.json

# Create some packages
mkdir -p packages/pkg1 packages/pkg2
cd packages/pkg1 && npm init -y
cd ../pkg2 && npm init -y

# Now use odometer to manage versions
odo show                    # See all package versions
odo roll --workspace patch  # Bump all packages
odo sync 1.0.0             # Set all to same version
```

### Rust Workspace Example

```bash
# Initialize a Rust workspace
cargo init --lib
# Add workspace configuration to Cargo.toml
echo '[workspace]
members = ["packages/*"]' >> Cargo.toml

# Create some crates
mkdir -p packages/crate1 packages/crate2
cd packages/crate1 && cargo init --lib
cd ../crate2 && cargo init --lib

# Now use odometer to manage versions
odo show                    # See all crate versions
odo roll --workspace patch  # Bump all crates
odo sync 1.0.0             # Set all to same version
```

## Development

### Setup

```bash
git clone https://github.com/levicook/odometer.git
cd odometer
make install-tools
```

### Testing

```bash
make test              # Unit tests (fast)
make test-integration  # Integration tests with fixtures
make test-all          # Run everything
```

### Available Make Targets

```bash
make help              # Show all available commands
make check             # Check code without building
make build             # Build project
make fixtures          # Generate test fixtures
make clean             # Clean build artifacts
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Run the test suite (`make test-all`)
6. Commit your changes (`git commit -am 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

### Architecture

Odometer uses a clean architecture with three main layers:

- **Domain** (`src/domain.rs`) - Pure business logic for version operations
- **IO** (`src/io/`) - File system operations (currently Cargo, designed for Node.js/Python expansion)
- **CLI** (`src/cli.rs`) - Command-line interface and orchestration

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) 🦀
- CLI powered by [clap](https://github.com/clap-rs/clap)
- Version parsing via [semver](https://github.com/dtolnay/semver)
- TOML manipulation using [toml_edit](https://github.com/ordian/toml_edit)

---

_Made with ❤️ for the Rust community_
