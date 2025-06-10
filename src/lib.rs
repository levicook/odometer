pub mod cli;
pub mod domain;
pub mod io;

use clap::Parser;
use cli::{Cli, Commands};

pub fn run() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Roll {
            bump_type,
            package_selection,
        } => handle_roll(bump_type.into(), package_selection.into()),
        Commands::Set {
            version,
            package_selection,
        } => handle_set(version, package_selection.into()),
        Commands::Sync { version } => handle_sync(version),
        Commands::Show => handle_show(),
        Commands::Lint => handle_lint(),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn handle_roll(
    bump: domain::VersionBump,
    selection: domain::PackageSelection,
) -> anyhow::Result<()> {
    let mut workspace = io::load_workspace()?;
    workspace.roll_version(bump, &selection)?;
    io::save_workspace(&workspace)?;
    Ok(())
}

fn handle_set(version: String, selection: domain::PackageSelection) -> anyhow::Result<()> {
    let mut workspace = io::load_workspace()?;
    workspace.set_version(&version, &selection)?;
    io::save_workspace(&workspace)?;
    Ok(())
}

fn handle_sync(version: String) -> anyhow::Result<()> {
    let mut workspace = io::load_workspace()?;
    workspace.sync_version(&version)?;
    io::save_workspace(&workspace)?;
    Ok(())
}

fn handle_show() -> anyhow::Result<()> {
    let workspace = io::load_workspace()?;
    println!("{}", workspace.show());
    Ok(())
}

fn handle_lint() -> anyhow::Result<()> {
    let workspace = io::load_workspace()?;
    let errors = workspace.lint();

    if errors.is_empty() {
        println!("✅ All workspace versions are valid");
    } else {
        for error in errors {
            eprintln!("❌ {}: {}", error.member, error.message);
        }
        std::process::exit(1);
    }

    Ok(())
}
