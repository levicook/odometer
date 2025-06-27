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

/// Configuration for controlling which files and directories are ignored during workspace discovery
#[derive(Args, Debug, Clone, Default)]
pub struct IgnoreOptions {
    /// Don't respect .gitignore files
    #[arg(long)]
    pub no_ignore_git: bool,

    /// Don't respect .ignore files (ripgrep/ag format)
    #[arg(long)]
    pub no_ignore: bool,

    /// Don't respect global gitignore files
    #[arg(long)]
    pub no_ignore_global: bool,

    /// Don't automatically ignore hidden files and directories
    #[arg(long)]
    pub hidden: bool,

    /// Disable all ignore filtering (show everything)
    #[arg(long)]
    pub no_ignore_all: bool,
}

#[derive(Clone, Debug, ValueEnum, Default)]
pub(crate) enum OutputFormat {
    /// Simple human-readable format (default)
    #[default]
    Simple,
    /// JSON format for scripting
    Json,
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

        #[command(flatten)]
        ignore_options: IgnoreOptions,
    },

    /// Set ALL workspace members to same version (lockstep)
    Sync {
        /// Version to sync all crates to (e.g., "1.2.3")
        version: String,

        /// Output format
        #[arg(long, default_value = "simple")]
        format: OutputFormat,

        #[command(flatten)]
        ignore_options: IgnoreOptions,
    },

    /// Display current versions for workspace members
    Show {
        #[command(flatten)]
        package_selection: PackageSelection,

        #[command(flatten)]
        ignore_options: IgnoreOptions,
    },

    /// Check for missing/malformed version fields
    Lint {
        #[command(flatten)]
        package_selection: PackageSelection,

        #[command(flatten)]
        ignore_options: IgnoreOptions,
    },
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
}

#[derive(Subcommand, Debug)]
pub(crate) enum BumpType {
    /// Increment major version (x.0.0)
    Major {
        /// Amount to increment/decrement (default: 1, negative values decrement)
        #[arg(default_value = "1", allow_negative_numbers = true)]
        amount: i32,

        #[command(flatten)]
        package_selection: PackageSelection,

        #[command(flatten)]
        ignore_options: IgnoreOptions,

        /// Output format
        #[arg(long, default_value = "simple")]
        format: OutputFormat,
    },

    /// Increment minor version (x.y.0)
    Minor {
        /// Amount to increment/decrement (default: 1, negative values decrement)
        #[arg(default_value = "1", allow_negative_numbers = true)]
        amount: i32,

        #[command(flatten)]
        package_selection: PackageSelection,

        #[command(flatten)]
        ignore_options: IgnoreOptions,

        /// Output format
        #[arg(long, default_value = "simple")]
        format: OutputFormat,
    },

    /// Increment patch version (x.y.z)
    Patch {
        /// Amount to increment/decrement (default: 1, negative values decrement)
        #[arg(default_value = "1", allow_negative_numbers = true)]
        amount: i32,

        #[command(flatten)]
        package_selection: PackageSelection,

        #[command(flatten)]
        ignore_options: IgnoreOptions,

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
        IgnoreOptions,
        OutputFormat,
    )
{
    fn from(bump_type: BumpType) -> Self {
        match bump_type {
            BumpType::Major {
                amount,
                package_selection,
                ignore_options,
                format,
            } => (
                crate::domain::VersionBump::Major(amount),
                package_selection.into(),
                ignore_options,
                format,
            ),
            BumpType::Minor {
                amount,
                package_selection,
                ignore_options,
                format,
            } => (
                crate::domain::VersionBump::Minor(amount),
                package_selection.into(),
                ignore_options,
                format,
            ),
            BumpType::Patch {
                amount,
                package_selection,
                ignore_options,
                format,
            } => (
                crate::domain::VersionBump::Patch(amount),
                package_selection.into(),
                ignore_options,
                format,
            ),
        }
    }
}

impl From<PackageSelection> for crate::domain::PackageSelection {
    fn from(selection: PackageSelection) -> Self {
        if !selection.packages.is_empty() {
            crate::domain::PackageSelection::Specific(selection.packages)
        } else if selection.workspace || selection.all {
            crate::domain::PackageSelection::Workspace
        } else {
            crate::domain::PackageSelection::Default
        }
    }
}
