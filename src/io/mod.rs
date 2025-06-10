pub mod cargo;
// pub mod node;    // Future: Node.js/npm support  
// pub mod python;  // Future: Python/pip support

use anyhow::Result;
use crate::domain::{Workspace, WorkspaceMember};
use std::path::PathBuf;

/// Load the current workspace from the file system
/// 
/// This function detects the project type and delegates to the appropriate
/// ecosystem-specific loader (currently only Cargo, but designed for Node/Python/etc.)
pub fn load_workspace() -> Result<Workspace> {
    // For now, we only support Cargo projects
    // Future: detect project type (package.json, pyproject.toml, etc.) and delegate
    cargo::load_cargo_workspace()
}

/// Save workspace changes back to the file system
/// 
/// This function delegates to the appropriate ecosystem-specific saver
/// based on the WorkspaceMember types.
pub fn save_workspace(workspace: &Workspace) -> Result<()> {
    for member in &workspace.members {
        match member {
            WorkspaceMember::Cargo { path, version, .. } => {
                cargo::update_cargo_toml_version(path, version)?;
            }
            // Future ecosystem support:
            // WorkspaceMember::Node { path, version, .. } => {
            //     node::update_package_json_version(path, version)?;
            // }
            // WorkspaceMember::Python { path, version, .. } => {
            //     python::update_pyproject_version(path, version)?;
            // }
        }
    }
    Ok(())
}

/// Find the project root by walking up the directory tree
/// 
/// Currently looks for Cargo.toml, but could be extended to look for
/// package.json, pyproject.toml, etc.
pub fn find_project_root() -> Result<PathBuf> {
    cargo::find_project_root()
} 