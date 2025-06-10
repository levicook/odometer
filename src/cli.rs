use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "odometer")]
#[command(about = "A workspace version management tool")]
#[command(long_about = "Keeps package versions synchronized across projects")]
#[command(version)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Clone, Debug, ValueEnum)]
pub(crate) enum OutputFormat {
    /// Simple human-readable format (default)
    Simple,
    /// JSON format for scripting
    Json,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Simple
    }
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Increment version numbers
    Roll {
        #[command(subcommand)]
        bump_type: BumpType,
    },

    /// Set workspace root version to specific version
    Set {
        /// Version to set (e.g., "1.2.3")
        version: String,

        #[command(flatten)]
        package_selection: PackageSelection,

        /// Output format
        #[arg(long, default_value = "simple")]
        format: OutputFormat,
    },

    /// Set ALL workspace members to same version (lockstep)
    Sync {
        /// Version to sync all crates to (e.g., "1.2.3")
        version: String,

        /// Output format
        #[arg(long, default_value = "simple")]
        format: OutputFormat,
    },

    /// Display current versions for workspace members
    Show {
        #[command(flatten)]
        package_selection: PackageSelection,
    },

    /// Check for missing/malformed version fields
    Lint {
        #[command(flatten)]
        package_selection: PackageSelection,
    },
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
    Major {
        #[command(flatten)]
        amount: BumpAmount,

        #[command(flatten)]
        package_selection: PackageSelection,

        /// Output format
        #[arg(long, default_value = "simple")]
        format: OutputFormat,
    },

    /// Increment minor version (x.y.0)
    Minor {
        #[command(flatten)]
        amount: BumpAmount,

        #[command(flatten)]
        package_selection: PackageSelection,

        /// Output format
        #[arg(long, default_value = "simple")]
        format: OutputFormat,
    },

    /// Increment patch version (x.y.z)
    Patch {
        #[command(flatten)]
        amount: BumpAmount,

        #[command(flatten)]
        package_selection: PackageSelection,

        /// Output format
        #[arg(long, default_value = "simple")]
        format: OutputFormat,
    },
}

// CLI to Domain converters
impl From<BumpType>
    for (
        crate::domain::VersionBump,
        crate::domain::PackageSelection,
        OutputFormat,
    )
{
    fn from(bump_type: BumpType) -> Self {
        match bump_type {
            BumpType::Major {
                amount,
                package_selection,
                format,
            } => (
                crate::domain::VersionBump::Major(amount.amount),
                package_selection.into(),
                format,
            ),
            BumpType::Minor {
                amount,
                package_selection,
                format,
            } => (
                crate::domain::VersionBump::Minor(amount.amount),
                package_selection.into(),
                format,
            ),
            BumpType::Patch {
                amount,
                package_selection,
                format,
            } => (
                crate::domain::VersionBump::Patch(amount.amount),
                package_selection.into(),
                format,
            ),
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
