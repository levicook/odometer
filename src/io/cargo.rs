use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, value};
use anyhow::{Context, Result};

use crate::domain::{Workspace, WorkspaceMember};

// ==============================================================================
// CARGO.TOML PARSING & EXTRACTION
// ==============================================================================

#[derive(Debug, Clone)]
pub struct CargoToml {
    pub name: Option<String>,
    pub version: Option<String>,
    pub has_workspace_section: bool,
    pub has_package_section: bool,
    pub uses_workspace_version: bool,
}

impl CargoToml {
    /// Parse Cargo.toml content into structured metadata
    pub fn parse(toml_content: &str) -> Result<Self> {
        let doc = toml_content.parse::<DocumentMut>()
            .context("Failed to parse TOML content")?;
        
        Ok(Self {
            name: cargo_name(&doc),
            version: cargo_version(&doc),
            has_workspace_section: has_workspace_section(&doc),
            has_package_section: has_package_section(&doc),
            uses_workspace_version: extract_uses_workspace_version(&doc),
        })
    }
    
    /// Update version in TOML content, returning new content string
    pub fn update_version(toml_content: &str, new_version: &str) -> Result<String> {
        let mut doc = toml_content.parse::<DocumentMut>()
            .context("Failed to parse TOML content")?;
        
        if let Some(package) = doc.get_mut("package") {
            if let Some(package_table) = package.as_table_mut() {
                package_table["version"] = value(new_version);
            }
        }
        
        Ok(doc.to_string())
    }
}

// Pure extraction functions operating on parsed documents
fn cargo_version(doc: &DocumentMut) -> Option<String> {
    doc.get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn cargo_name(doc: &DocumentMut) -> Option<String> {
    doc.get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string())
}

fn has_workspace_section(doc: &DocumentMut) -> bool {
    doc.get("workspace").is_some()
}

fn has_package_section(doc: &DocumentMut) -> bool {
    doc.get("package").is_some()
}

fn extract_uses_workspace_version(doc: &DocumentMut) -> bool {
    uses_workspace_inheritance(doc, "package", "version")
}

/// Check if a field uses workspace inheritance (field = { workspace = true })
/// 
/// Handles both regular tables and inline tables since Cargo can use either:
/// - `version = { workspace = true }` (inline table)
/// - `[package.version] workspace = true` (regular table)
fn uses_workspace_inheritance(doc: &DocumentMut, section: &str, field: &str) -> bool {
    doc.get(section)
        .and_then(|s| s.get(field))
        .and_then(|value| {
            // Check both regular tables and inline tables for workspace = true
            if let Some(table) = value.as_table() {
                table.get("workspace").and_then(|w| w.as_bool())
            } else if let Some(inline_table) = value.as_inline_table() {
                inline_table.get("workspace").and_then(|w| w.as_bool())
            } else {
                None
            }
        })
        .unwrap_or(false)
}

// ==============================================================================
// CARGO WORKSPACE OPERATIONS
// ==============================================================================

/// Load a Cargo workspace from the file system
pub fn load_cargo_workspace() -> Result<Workspace> {
    let project_root = find_project_root()?;
    let root_manifest = project_root.join("Cargo.toml");
    
    let mut members = Vec::new();
    
    // Check if this is a workspace or single crate
    if is_workspace_root(&root_manifest)? {
        // Multi-crate workspace: discover all members
        members.extend(discover_workspace_members(&project_root)?);
        
        // Add workspace root if it has a [package] section too
        if has_package_section_file(&root_manifest)? {
            let name = read_package_name(&root_manifest)?
                .unwrap_or_else(|| "workspace".to_string());
            let version = read_cargo_toml_version(&root_manifest)?
                .context("Workspace root package must have a version")?;
            members.insert(0, WorkspaceMember::Cargo {
                name,
                version,
                path: root_manifest,
                has_workspace_inheritance: false,
            });
        }
    } else {
        // Single crate project
        let name = read_package_name(&root_manifest)?
            .context("Package must have a name")?;
        let version = read_cargo_toml_version(&root_manifest)?
            .context("Package must have a version")?;
        members.push(WorkspaceMember::Cargo {
            name,
            version,
            path: root_manifest,
            has_workspace_inheritance: false,
        });
    }
    
    Ok(Workspace { members })
}

/// Find the project root using workspace-first strategy
/// 
/// Algorithm:
/// 1. Walk up from current dir looking for Cargo.toml files
/// 2. For each Cargo.toml found, check if it defines a workspace
/// 3. If workspace found, that's our project root
/// 4. If we exhaust all ancestors without finding a workspace,
///    use the first (lowest) Cargo.toml we found
pub fn find_project_root() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()
        .context("Failed to get current directory")?;
    
    let mut first_cargo_toml = None;
    
    for ancestor in current_dir.ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if cargo_toml.exists() {
            // Remember the first Cargo.toml we find (closest to current dir)
            if first_cargo_toml.is_none() {
                first_cargo_toml = Some(ancestor.to_path_buf());
            }
            
            // Check if this Cargo.toml defines a workspace
            if is_workspace_root(&cargo_toml)? {
                return Ok(ancestor.to_path_buf());
            }
        }
    }
    
    // No workspace found, use the first Cargo.toml we encountered
    first_cargo_toml
        .ok_or_else(|| anyhow::anyhow!("No Cargo.toml found. Make sure you're in a Rust project."))
}

/// Check if a Cargo.toml file defines a workspace
fn is_workspace_root(cargo_toml_path: &Path) -> Result<bool> {
    let content = fs::read_to_string(cargo_toml_path)
        .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;
    
    let cargo_toml = CargoToml::parse(&content)
        .with_context(|| format!("Failed to parse TOML in {}", cargo_toml_path.display()))?;
    
    Ok(cargo_toml.has_workspace_section)
}

/// Discover all workspace members by walking the filesystem
/// 
/// This approach is more robust than parsing [workspace] members because:
/// - It finds all Cargo.toml files regardless of workspace config  
/// - No need to handle complex glob patterns
/// - Similar to how `git` finds all files under the repo root
pub fn discover_workspace_members(workspace_root: &Path) -> Result<Vec<WorkspaceMember>> {
    let mut members = Vec::new();
    
    // Walk the directory tree to find all Cargo.toml files
    visit_cargo_tomls(workspace_root, &mut |cargo_toml_path| {
        // Skip the workspace root manifest (it's handled separately)
        if cargo_toml_path == workspace_root.join("Cargo.toml") {
            return Ok(());
        }
        
        // Check if this is a valid package (has [package] section)
        if let Ok(member) = load_workspace_member(cargo_toml_path) {
            members.push(member);
        }
        
        Ok(())
    })?;
    
    Ok(members)
}

/// Recursively walk directory tree and call visitor for each Cargo.toml found
fn visit_cargo_tomls<F>(dir: &Path, visitor: &mut F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    if !dir.is_dir() {
        return Ok(());
    }
    
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            // Skip target directories and hidden directories to avoid noise
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') || name == "target" {
                    continue;
                }
            }
            // Recursively visit subdirectories
            visit_cargo_tomls(&path, visitor)?;
        } else if path.file_name() == Some(std::ffi::OsStr::new("Cargo.toml")) {
            // Found a Cargo.toml, call the visitor
            visitor(&path)?;
        }
    }
    
    Ok(())
}

/// Load a single workspace member from its Cargo.toml
fn load_workspace_member(member_manifest: &Path) -> Result<WorkspaceMember> {
    let content = fs::read_to_string(member_manifest)
        .with_context(|| format!("Failed to read {}", member_manifest.display()))?;
    
    let cargo_toml = CargoToml::parse(&content)
        .with_context(|| format!("Failed to parse {}", member_manifest.display()))?;
    
    let name = cargo_toml.name
        .context("Workspace member must have a name")?;
    
    let version = if cargo_toml.uses_workspace_version {
        // If using workspace inheritance, we'll need to resolve the workspace version
        // For now, use a placeholder - this will be resolved later
        "workspace".to_string()
    } else {
        cargo_toml.version
            .context("Workspace member must have a version or use workspace inheritance")?
    };
    
    Ok(WorkspaceMember::Cargo {
        name,
        version,
        path: member_manifest.to_path_buf(),
        has_workspace_inheritance: cargo_toml.uses_workspace_version,
    })
}

/// Update the version in a specific Cargo.toml file
pub fn update_cargo_toml_version(cargo_toml_path: &Path, new_version: &str) -> Result<()> {
    let content = fs::read_to_string(cargo_toml_path)
        .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;
    
    let updated_content = CargoToml::update_version(&content, new_version)
        .with_context(|| format!("Failed to update version in {}", cargo_toml_path.display()))?;
    
    fs::write(cargo_toml_path, updated_content)
        .with_context(|| format!("Failed to write {}", cargo_toml_path.display()))?;
    
    Ok(())
}

/// Read the current version from a Cargo.toml file
pub fn read_cargo_toml_version(cargo_toml_path: &Path) -> Result<Option<String>> {
    let content = fs::read_to_string(cargo_toml_path)
        .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;
    
    let cargo_toml = CargoToml::parse(&content)
        .with_context(|| format!("Failed to parse TOML in {}", cargo_toml_path.display()))?;
    
    Ok(cargo_toml.version)
}

/// Check if a package uses workspace inheritance for its version
pub fn uses_workspace_version(cargo_toml_path: &Path) -> Result<bool> {
    let content = fs::read_to_string(cargo_toml_path)
        .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;
    
    let cargo_toml = CargoToml::parse(&content)
        .with_context(|| format!("Failed to parse TOML in {}", cargo_toml_path.display()))?;
    
    Ok(cargo_toml.uses_workspace_version)
}

/// Check if a Cargo.toml has a [package] section
fn has_package_section_file(cargo_toml_path: &Path) -> Result<bool> {
    let content = fs::read_to_string(cargo_toml_path)
        .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;
    
    let cargo_toml = CargoToml::parse(&content)
        .with_context(|| format!("Failed to parse TOML in {}", cargo_toml_path.display()))?;
    
    Ok(cargo_toml.has_package_section)
}

/// Read the package name from a Cargo.toml file
fn read_package_name(cargo_toml_path: &Path) -> Result<Option<String>> {
    let content = fs::read_to_string(cargo_toml_path)
        .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;
    
    let cargo_toml = CargoToml::parse(&content)
        .with_context(|| format!("Failed to parse TOML in {}", cargo_toml_path.display()))?;
    
    Ok(cargo_toml.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cargo_toml_parse_basic_package() {
        let toml = r#"
[package]
name = "my-crate"
version = "1.2.3"
edition = "2021"
"#;
        
        let cargo_toml = CargoToml::parse(toml).unwrap();
        
        assert_eq!(cargo_toml.name, Some("my-crate".to_string()));
        assert_eq!(cargo_toml.version, Some("1.2.3".to_string()));
        assert!(!cargo_toml.has_workspace_section);
        assert!(cargo_toml.has_package_section);
        assert!(!cargo_toml.uses_workspace_version);
    }

    #[test]
    fn test_cargo_toml_parse_workspace_root() {
        let toml = r#"
[workspace]
members = ["crates/*", "tools/cli"]

[package]
name = "workspace-root"
version = "0.1.0"
"#;
        
        let cargo_toml = CargoToml::parse(toml).unwrap();
        
        assert_eq!(cargo_toml.name, Some("workspace-root".to_string()));
        assert_eq!(cargo_toml.version, Some("0.1.0".to_string()));
        assert!(cargo_toml.has_workspace_section);
        assert!(cargo_toml.has_package_section);
        assert!(!cargo_toml.uses_workspace_version);
    }

    #[test]
    fn test_cargo_toml_parse_workspace_only() {
        let toml = r#"
[workspace]
members = ["packages/*"]
resolver = "2"
"#;
        
        let cargo_toml = CargoToml::parse(toml).unwrap();
        
        assert_eq!(cargo_toml.name, None);
        assert_eq!(cargo_toml.version, None);
        assert!(cargo_toml.has_workspace_section);
        assert!(!cargo_toml.has_package_section);
        assert!(!cargo_toml.uses_workspace_version);
    }

    #[test]
    fn test_cargo_toml_parse_workspace_inheritance() {
        let toml = r#"
[package]
name = "member-crate"
version = { workspace = true }
edition = { workspace = true }
"#;
        
        let cargo_toml = CargoToml::parse(toml).unwrap();
        
        assert_eq!(cargo_toml.name, Some("member-crate".to_string()));
        assert_eq!(cargo_toml.version, None); // workspace inheritance means no direct version
        assert!(!cargo_toml.has_workspace_section);
        assert!(cargo_toml.has_package_section);
        assert!(cargo_toml.uses_workspace_version);
    }

    #[test]
    fn test_cargo_toml_parse_mixed_inheritance() {
        let toml = r#"
[package]
name = "mixed-crate"
version = "1.0.0"
edition = { workspace = true }
"#;
        
        let cargo_toml = CargoToml::parse(toml).unwrap();
        
        assert_eq!(cargo_toml.name, Some("mixed-crate".to_string()));
        assert_eq!(cargo_toml.version, Some("1.0.0".to_string()));
        assert!(!cargo_toml.has_workspace_section);
        assert!(cargo_toml.has_package_section);
        assert!(!cargo_toml.uses_workspace_version); // version is NOT inherited
    }

    #[test]
    fn test_cargo_toml_parse_empty() {
        let toml = r#"
# Just a comment
"#;
        
        let cargo_toml = CargoToml::parse(toml).unwrap();
        
        assert_eq!(cargo_toml.name, None);
        assert_eq!(cargo_toml.version, None);
        assert!(!cargo_toml.has_workspace_section);
        assert!(!cargo_toml.has_package_section);
        assert!(!cargo_toml.uses_workspace_version);
    }

    #[test]
    fn test_cargo_toml_parse_invalid_toml() {
        let toml = r#"
[package
name = "broken
"#;
        
        let result = CargoToml::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_cargo_toml_update_version_basic() {
        let toml = r#"[package]
name = "test"
version = "1.0.0"
"#;
        
        let updated = CargoToml::update_version(toml, "2.0.0").unwrap();
        
        assert!(updated.contains("version = \"2.0.0\""));
        assert!(updated.contains("name = \"test\""));
        // Should preserve structure
        assert!(updated.contains("[package]"));
    }

    #[test]
    fn test_cargo_toml_update_version_preserves_formatting() {
        let toml = r#"# My awesome crate
[package]
name = "test"
version = "1.0.0"  # Current version
edition = "2021"

[dependencies]
anyhow = "1.0"
"#;
        
        let updated = CargoToml::update_version(toml, "1.1.0").unwrap();
        
        assert!(updated.contains("version = \"1.1.0\""));
        // Should preserve comments and structure
        assert!(updated.contains("# My awesome crate"));
        assert!(updated.contains("[dependencies]"));
        assert!(updated.contains("anyhow = \"1.0\""));
    }

    #[test]
    fn test_cargo_toml_update_version_no_package_section() {
        let toml = r#"[workspace]
members = ["crates/*"]
"#;
        
        let updated = CargoToml::update_version(toml, "2.0.0").unwrap();
        
        // Should not add version if no [package] section exists
        assert!(!updated.contains("version = \"2.0.0\""));
        assert!(updated.contains("[workspace]"));
    }

    #[test]
    fn test_cargo_toml_update_version_workspace_inheritance() {
        let toml = r#"[package]
name = "test"
version = { workspace = true }
"#;
        
        let updated = CargoToml::update_version(toml, "2.0.0").unwrap();
        
        // Should overwrite workspace inheritance with concrete version
        assert!(updated.contains("version = \"2.0.0\""));
        assert!(!updated.contains("workspace = true"));
        }
} 