pub mod cli;
pub mod domain;
pub mod io;

use clap::Parser;
use cli::{Cli, Commands, OutputFormat};

pub fn run() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Roll { bump_type } => {
            let (bump, selection, ignore_options, format) = bump_type.into();
            handle_roll(bump, selection, format, &ignore_options)
        }
        Commands::Set {
            version,
            package_selection,
            ignore_options,
            format,
        } => handle_set(version, package_selection.into(), format, &ignore_options),
        Commands::Sync {
            version,
            ignore_options,
            format,
        } => handle_sync(version, format, &ignore_options),
        Commands::Show {
            package_selection,
            ignore_options,
        } => handle_show(package_selection.into(), &ignore_options),
        Commands::Lint {
            package_selection,
            ignore_options,
        } => handle_lint(package_selection.into(), &ignore_options),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn display_operation_result(result: &domain::OperationResult, format: &OutputFormat) {
    match format {
        OutputFormat::Simple => {
            for change in &result.changes {
                println!(
                    "{}: {} → {}",
                    change.package, change.old_version, change.new_version
                );
            }
        }
        OutputFormat::Json => match serde_json::to_string_pretty(result) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Error serializing to JSON: {}", e),
        },
    }
}

fn handle_roll(
    bump: domain::VersionBump,
    selection: domain::PackageSelection,
    format: OutputFormat,
    ignore_options: &cli::IgnoreOptions,
) -> anyhow::Result<()> {
    let mut workspace = io::load_workspace(ignore_options)?;
    let result = workspace.roll_version(bump, &selection)?;
    io::save_workspace(&workspace)?;

    display_operation_result(&result, &format);
    Ok(())
}

fn handle_set(
    version: String,
    selection: domain::PackageSelection,
    format: OutputFormat,
    ignore_options: &cli::IgnoreOptions,
) -> anyhow::Result<()> {
    let mut workspace = io::load_workspace(ignore_options)?;
    let result = workspace.set_version(&version, &selection)?;
    io::save_workspace(&workspace)?;

    display_operation_result(&result, &format);
    Ok(())
}

fn handle_sync(
    version: String,
    format: OutputFormat,
    ignore_options: &cli::IgnoreOptions,
) -> anyhow::Result<()> {
    let mut workspace = io::load_workspace(ignore_options)?;
    let result = workspace.sync_version(&version)?;
    io::save_workspace(&workspace)?;

    display_operation_result(&result, &format);
    Ok(())
}

fn handle_show(
    selection: domain::PackageSelection,
    ignore_options: &cli::IgnoreOptions,
) -> anyhow::Result<()> {
    let workspace = io::load_workspace(ignore_options)?;

    // If no specific selection is made, show all members
    let effective_selection = match selection {
        domain::PackageSelection::Default => domain::PackageSelection::Workspace,
        _ => selection,
    };

    let output = workspace.show(&effective_selection)?;
    print!("{}", output);
    Ok(())
}

fn handle_lint(
    selection: domain::PackageSelection,
    ignore_options: &cli::IgnoreOptions,
) -> anyhow::Result<()> {
    let workspace = io::load_workspace(ignore_options)?;

    // If no specific selection is made, lint all members
    let effective_selection = match selection {
        domain::PackageSelection::Default => domain::PackageSelection::Workspace,
        _ => selection,
    };

    let errors = workspace.lint(&effective_selection)?;

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
