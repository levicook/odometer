# Odometer Implementation Spec & Checklist

A Rust workspace version management tool that keeps all crate versions synchronized.

## Core Functionality

### Commands

#### Core Commands
- [ ] `odo roll major` - increment workspace root major version by 1
- [ ] `odo roll minor` - increment workspace root minor version by 1  
- [ ] `odo roll patch` - increment workspace root patch version by 1
- [ ] `odo roll patch -1` - decrement workspace root patch version by 1
- [ ] `odo roll patch 10` - increment workspace root patch version by 10
- [ ] `odo set x.y.z` - set workspace root version to specific version
- [ ] `odo sync x.y.z` - set ALL workspace members to same version (lockstep)
- [ ] `odo show` - display current versions for all workspace members
- [ ] `odo lint` - check for missing/malformed version fields only

#### Package Selection (Cargo-style semantics)
- [ ] `odo roll patch -p crate-name` - bump specific package only
- [ ] `odo roll patch --package crate-name` - long form of `-p`
- [ ] `odo roll patch -p crate-a -p crate-b` - bump multiple specific packages
- [ ] `odo roll patch --workspace` - independently bump ALL workspace members
- [ ] `odo roll patch -w` - short form of `--workspace`
- [ ] `odo set x.y.z --workspace` - set version for all workspace members independently
- [ ] `odo set x.y.z -p crate-name` - set version for specific package

#### Advanced Package Selection
- [ ] `odo roll patch --workspace --exclude crate-name` - all except specified
- [ ] `odo roll patch --all` - alias for `--workspace` (cargo compatibility)
- [ ] `odo lint -p crate-name` - check specific package for valid version field
- [ ] `odo show -p crate-name` - show version for specific package

### Version Detection & Parsing
- [ ] Parse semantic versions from `Cargo.toml` files
- [ ] Handle workspace inheritance (`version.workspace = true`)
- [ ] Support both workspace root and member crate versions
- [ ] Validate semver format (x.y.z, handle pre-release/build metadata)

### Lint Functionality (Simplified)
- [ ] Check for missing `version` fields in workspace members
- [ ] Validate semver format in all version fields  
- [ ] Report malformed versions with helpful error messages
- [ ] **No version consistency checking** (that's what `show` and `sync` are for)

### File Discovery
- [ ] Find workspace root `Cargo.toml`
- [ ] Discover all workspace members from `[workspace.members]`
- [ ] Respect `.gitignore` patterns (skip ignored files by default)
- [ ] Handle glob patterns in workspace members
- [ ] Skip excluded workspace members

## Project Setup

### Repository Structure
- [ ] Create `odometer` repository
- [ ] Initialize Rust project with `cargo init --name odometer`
- [ ] Set up multiple binary targets in `Cargo.toml`:
  ```toml
  [[bin]]
  name = "cargo-odometer"
  path = "src/main.rs"
  
  [[bin]]
  name = "cargo-odo"
  path = "src/main.rs"
  
  [[bin]]
  name = "odometer"
  path = "src/main.rs"
  
  [[bin]]
  name = "odo"
  path = "src/main.rs"
  ```

### Dependencies
- [ ] Add `clap` for CLI argument parsing
- [ ] Add `toml` for parsing Cargo.toml files
- [ ] Add `semver` for version manipulation
- [ ] Add `walkdir` for file system traversal
- [ ] Add `anyhow` for error handling
- [ ] Add `serde` for TOML deserialization

### Usage Examples Section

Add this section after "Core Functionality" for quick reference during implementation:

## Usage Examples

### Basic Operations (Workspace Root)
```bash
# Increment workspace root version
odo roll patch              # 1.0.0 -> 1.0.1
odo roll minor              # 1.0.1 -> 1.1.0  
odo roll major              # 1.1.0 -> 2.0.0

# Custom increments
odo roll patch 5            # 2.0.0 -> 2.0.5
odo roll patch -2           # 2.0.5 -> 2.0.3

# Set specific version
odo set 3.1.4               # Set workspace root to exactly 3.1.4
```

### Lockstep Versioning Workflow
```bash
# Get everything in sync first
odo sync 1.0.0              # Set ALL crates to 1.0.0

# Now operations work predictably across all crates
odo roll patch --workspace  # All: 1.0.0 -> 1.0.1
odo roll minor --workspace  # All: 1.0.1 -> 1.1.0
```

### Independent Versioning Workflow
```bash
# Work on specific packages
odo roll patch -p my-core         # Just my-core gets bumped
odo roll minor --package my-utils # Just my-utils gets bumped
odo roll patch -p crate-a -p crate-b # Two specific crates

# Bulk independent operations
odo roll patch --workspace        # Each crate's patch version increments
# Example: crate-a (1.0.0->1.0.1), crate-b (0.2.5->0.2.6)
```

### Inspection Commands
```bash
# See current state
odo show                    # All workspace versions
odo show -p my-core         # Specific crate version

# Validate version fields
odo lint                    # Check for missing/malformed versions
odo lint -p my-core         # Check specific crate
```

### Real-world Workflows
```bash
# Preparing a coordinated release
odo sync 2.0.0              # Get everything aligned
odo show                    # Verify all at 2.0.0
odo lint                    # Check all version fields valid

# Working on independent features
odo roll minor -p my-app    # App gets new feature
odo roll patch -p my-utils  # Utils gets bugfix
odo show                    # See current state

# Bulk patch release
odo roll patch --workspace  # Everyone gets patch bump
odo lint                    # Validate everything
```

### CLI Structure
- [ ] Set up clap subcommands (`roll`, `lint`, `set`)
- [ ] Handle `roll` command with bump type and optional amount
- [ ] Implement cargo-style package selection:
  - [ ] `-p/--package` for specific packages (can be repeated)
  - [ ] `--workspace/-w` for all workspace members  
  - [ ] `--exclude` to skip packages when using `--workspace`
  - [ ] `--all` alias for `--workspace`
- [ ] Default behavior: operate on workspace root only (safe default)
- [ ] Handle both `cargo odo` and direct `odo` invocation
- [ ] Implement help text and examples showing package selection

### Workspace Discovery
- [ ] Find workspace root (walk up directory tree looking for workspace `Cargo.toml`)
- [ ] Parse workspace configuration
- [ ] Resolve member paths (handle globs, relative paths)
- [ ] Build list of all `Cargo.toml` files to manage

### Version Management
- [ ] Read current versions from all workspace members
- [ ] Apply version changes atomically (specific packages or workspace root)
- [ ] Handle workspace inheritance properly
- [ ] Preserve TOML formatting when possible
- [ ] Implement `sync` command for lockstep versioning
- [ ] Support independent version bumping with `--workspace`

### Error Handling
- [ ] Graceful handling of malformed `Cargo.toml` files
- [ ] Clear error messages for invalid version formats
- [ ] Rollback on partial failures
- [ ] Validate workspace structure
- [ ] Handle missing package specifications gracefully

## Advanced Features

### Configuration
- [ ] Optional `odo.toml` configuration file
- [ ] Exclude patterns for specific crates
- [ ] Custom version constraints/rules
- [ ] Dry-run mode (`--dry-run` flag)

### Comment Tagging (Optional)
- [ ] Add `# odo` comments to managed version lines
- [ ] Detect manually edited versions
- [ ] Warn about unmanaged changes
- [ ] `--force` flag to override warnings

### Git Integration
- [ ] Respect `.gitignore` by default
- [ ] Optional git commit creation after version changes
- [ ] Git tag creation option
- [ ] `--include-ignored` flag to process ignored files

## Testing

### Unit Tests
- [ ] Version parsing and manipulation
- [ ] TOML file reading/writing
- [ ] Workspace discovery logic
- [ ] CLI argument parsing

### Integration Tests
- [ ] Create test workspace fixtures
- [ ] Test full version bump workflows
- [ ] Test error scenarios (malformed files, etc.)
- [ ] Test workspace inheritance scenarios

### Edge Cases
- [ ] Empty workspaces
- [ ] Nested workspaces
- [ ] Mixed workspace/non-workspace crates
- [ ] Pre-release versions
- [ ] Build metadata in versions

## Documentation

### User Documentation
- [ ] README with installation and usage examples
- [ ] Command help text and examples
- [ ] Common workflow documentation
- [ ] Troubleshooting guide

### Developer Documentation
- [ ] Code documentation and examples
- [ ] Architecture overview
- [ ] Contributing guidelines

## Distribution

### crates.io Publishing
- [ ] Set up crate metadata (description, keywords, license)
- [ ] Create crates.io account and API token
- [ ] Test `cargo publish --dry-run`
- [ ] Publish initial version

### Homebrew Formula
- [ ] Create `homebrew-odometer` tap repository
- [ ] Write Homebrew formula
- [ ] Set up GitHub releases with binaries
- [ ] Test homebrew installation

### GitHub Actions CI/CD
- [ ] Set up automated testing
- [ ] Cross-platform binary builds
- [ ] Automated releases on git tags
- [ ] Security scanning

## Quality Assurance

### Code Quality
- [ ] Set up `clippy` linting
- [ ] Format code with `rustfmt`
- [ ] Add pre-commit hooks
- [ ] Code coverage reporting

### User Experience
- [ ] Intuitive error messages
- [ ] Progress indicators for long operations
- [ ] Consistent command-line interface
- [ ] Shell completion scripts

## Future Enhancements

### Nice-to-Have Features
- [ ] Interactive mode for version selection
- [ ] Backup/restore functionality
- [ ] Integration with conventional commits
- [ ] Custom version schemes support
- [ ] Workspace dependency version updating
- [ ] JSON/YAML output formats for scripting

### Performance Optimizations
- [ ] Parallel file processing
- [ ] Incremental change detection
- [ ] Caching for large workspaces

## Release Checklist

### v0.1.0 (MVP)
- [ ] Basic `roll`, `lint`, `set` commands working
- [ ] Workspace discovery and version management
- [ ] Published to crates.io
- [ ] Basic documentation

### v0.2.0
- [ ] Configuration file support
- [ ] Git integration features
- [ ] Homebrew distribution
- [ ] Comprehensive testing

### v1.0.0
- [ ] Stable API
- [ ] Full feature set
- [ ] Production-ready error handling
- [ ] Comprehensive documentation