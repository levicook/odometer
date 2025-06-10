use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "odometer")]
#[command(about = "A workspace version management tool")]
#[command(long_about = "Keeps package versions synchronized across projects")]
#[command(version)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Increment version numbers
    Roll {
        #[command(subcommand)]
        bump_type: BumpType,

        #[command(flatten)]
        package_selection: PackageSelection,
    },

    /// Set workspace root version to specific version
    Set {
        /// Version to set (e.g., "1.2.3")
        version: String,

        #[command(flatten)]
        package_selection: PackageSelection,
    },

    /// Set ALL workspace members to same version (lockstep)
    Sync {
        /// Version to sync all crates to (e.g., "1.2.3")
        version: String,
    },

    /// Display current versions for workspace members
    Show,

    /// Check for missing/malformed version fields
    Lint,
}

#[derive(Args, Debug)]
pub(crate) struct BumpAmount {
    /// Amount to increment/decrement (default: 1, negative values decrement)
    #[arg(default_value = "1")]
    pub(crate) amount: i32,
}

#[derive(Args, Debug)]
pub(crate) struct PackageSelection {
    /// Target specific package(s) - can be used multiple times
    #[arg(short = 'p', long = "package")]
    pub(crate) packages: Vec<String>,

    /// Apply to all workspace members independently
    #[arg(short = 'w', long = "workspace", conflicts_with = "packages")]
    pub(crate) workspace: bool,

    /// Alias for --workspace (cargo compatibility)
    #[arg(long = "all", conflicts_with = "packages")]
    pub(crate) all: bool,

    /// Exclude specific packages when using --workspace
    #[arg(long = "exclude")]
    pub(crate) exclude: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum BumpType {
    /// Increment major version (x.0.0)
    Major(#[command(flatten)] BumpAmount),

    /// Increment minor version (x.y.0)
    Minor(#[command(flatten)] BumpAmount),

    /// Increment patch version (x.y.z)
    Patch(#[command(flatten)] BumpAmount),
}

// CLI to Domain converters
impl From<BumpType> for crate::domain::VersionBump {
    fn from(bump_type: BumpType) -> Self {
        match bump_type {
            BumpType::Major(amount) => crate::domain::VersionBump::Major(amount.amount),
            BumpType::Minor(amount) => crate::domain::VersionBump::Minor(amount.amount),
            BumpType::Patch(amount) => crate::domain::VersionBump::Patch(amount.amount),
        }
    }
}

impl From<PackageSelection> for crate::domain::PackageSelection {
    fn from(selection: PackageSelection) -> Self {
        crate::domain::PackageSelection {
            packages: selection.packages,
            workspace: selection.workspace || selection.all,
            exclude: selection.exclude,
        }
    }
}
