use anyhow::{Context, Result};
use std::{fs, path::Path};
use toml_edit::{DocumentMut, Formatted, Item, Value};

use crate::domain::VersionField;

/// Parse a Cargo.toml file and return (name, version, has_workspace_inheritance)
pub fn parse(path: &Path) -> Result<(Option<String>, VersionField)> {
    let content = fs::read_to_string(path). //-
        with_context(|| format!("Failed to read {}", path.display()))?;

    let doc = content
        .parse::<DocumentMut>()
        .with_context(|| format!("Failed to parse {}", path.display()))?;

    let package = get_package_section(&doc);

    let name = package
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    let version = if uses_workspace_inheritance(&doc, "package", "version") {
        VersionField::Inherited
    } else {
        match package.and_then(|p| p.get("version")) {
            None => VersionField::Absent,
            Some(v) => v
                .as_str()
                .map(|s| VersionField::Concrete(s.to_string()))
                .ok_or_else(|| anyhow::anyhow!("Version field must be a string"))?,
        }
    };

    Ok((name, version))
}

/// Update the version in a Cargo.toml file, preserving formatting
///
/// This function will update the version in the Cargo.toml file at the given path.
/// It will preserve the existing formatting of the version field, including comments.
///
/// # Arguments
/// * `path` - The path to the Cargo.toml file to update.
pub fn update_version(path: &Path, new_version: &VersionField) -> Result<()> {
    let new_version = match new_version {
        VersionField::Concrete(version) => version,
        _ => return Ok(()),
    };

    let content = fs::read_to_string(path). //-
        with_context(|| format!("Failed to read {}", path.display()))?;

    let mut doc = content
        .parse::<DocumentMut>()
        .with_context(|| format!("Failed to parse {}", path.display()))?;

    let package = get_package_section_mut(&mut doc).ok_or_else(|| {
        anyhow::anyhow!(
            "No workspace or package section found in {}",
            path.display()
        )
    })?;

    // Get the existing decor (comments) from the version field
    let decor = package
        .get("version")
        .and_then(|v| v.as_value())
        .map(|v| v.decor().clone());

    // Create new value with the same decor
    let mut new_value = Value::String(Formatted::new(new_version.to_string()));
    if let Some(d) = decor {
        if let Some(prefix_str) = d.prefix().and_then(|p| p.as_str()) {
            new_value.decor_mut().set_prefix(prefix_str.to_string());
        }
        if let Some(suffix_str) = d.suffix().and_then(|s| s.as_str()) {
            new_value.decor_mut().set_suffix(suffix_str.to_string());
        }
    }

    package["version"] = Item::Value(new_value);

    fs::write(path, doc.to_string())
        .with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

/// Get the package section from either workspace.package or package
fn get_package_section(doc: &DocumentMut) -> Option<&Item> {
    if doc.get("workspace").is_some() {
        doc.get("workspace").and_then(|w| w.get("package"))
    } else {
        doc.get("package")
    }
}

/// Get a mutable reference to the package section from either workspace.package or package
fn get_package_section_mut(doc: &mut DocumentMut) -> Option<&mut Item> {
    if doc.get("workspace").is_some() {
        doc.get_mut("workspace").and_then(|w| w.get_mut("package"))
    } else if doc.get("package").is_some() {
        doc.get_mut("package")
    } else {
        None
    }
}

/// Check if a field uses workspace inheritance (field = { workspace = true })
///
/// Handles both regular tables and inline tables since Cargo can use either:
/// - `version = { workspace = true }` (inline table)
/// - `[package.version] workspace = true` (regular table)
fn uses_workspace_inheritance(doc: &DocumentMut, section: &str, field: &str) -> bool {
    doc.get("workspace").is_none()
        && doc
            .get(section)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp_toml(contents: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", contents).unwrap();
        file
    }

    #[test]
    fn test_parse_basic_package() {
        let toml = r#"
            [package]
            name = "my-package"
            version = "1.2.3"
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Concrete("1.2.3".to_string()));
    }

    #[test]
    fn test_parse_workspace_inheritance_one() {
        let toml = r#"
            [package]
            name = "my-package"
            version = { workspace = true }
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Inherited);
    }

    #[test]
    fn test_parse_workspace_inheritance_two() {
        let toml = r#"
            [package]
            name = "my-package"
            version.workspace = true
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Inherited);
    }

    #[test]
    fn test_update_version_basic() {
        let toml = r#"
            [package]
            name = "my-package"
            version = "1.2.3"
        "#;
        let file = write_temp_toml(toml);
        let new_version = VersionField::Concrete("2.0.0".to_string());
        update_version(file.path(), &new_version).unwrap();
        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("version = \"2.0.0\""));
    }

    #[test]
    fn test_update_version_workspace_inheritance() {
        let toml = r#"
            [package]
            name = "my-package"
            version = { workspace = true }
        "#;
        let file = write_temp_toml(toml);
        let new_version = VersionField::Concrete("2.0.0".to_string());
        update_version(file.path(), &new_version).unwrap();
        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("version = \"2.0.0\""));
    }

    // Workspace package tests
    #[test]
    fn test_parse_workspace_package() {
        let toml = r#"
            [workspace.package]
            name = "workspace-package"
            version = "1.0.0"
            
            [workspace]
            members = ["crate1", "crate2"]
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("workspace-package".to_string()));
        assert_eq!(version, VersionField::Concrete("1.0.0".to_string()));
    }

    #[test]
    fn test_update_workspace_package_version() {
        let toml = r#"
            [workspace.package]
            name = "workspace-package"
            version = "1.0.0"
            
            [workspace]
            members = ["crate1"]
        "#;
        let file = write_temp_toml(toml);
        let new_version = VersionField::Concrete("2.0.0".to_string());
        update_version(file.path(), &new_version).unwrap();
        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("version = \"2.0.0\""));
    }

    // Edge cases and missing fields
    #[test]
    fn test_parse_package_missing_name() {
        let toml = r#"
            [package]
            version = "1.2.3"
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, None);
        assert_eq!(version, VersionField::Concrete("1.2.3".to_string()));
    }

    #[test]
    fn test_parse_package_missing_version() {
        let toml = r#"
            [package]
            name = "my-package"
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Absent);
    }

    // Error cases
    #[test]
    fn test_parse_no_package_or_workspace() {
        let toml = r#"
            [dependencies]
            serde = "1.0"
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, None);
        assert_eq!(version, VersionField::Absent);
    }

    #[test]
    fn test_update_version_no_package_or_workspace() {
        let toml = r#"
            [dependencies]
            serde = "1.0"
        "#;
        let file = write_temp_toml(toml);
        let new_version = VersionField::Concrete("2.0.0".to_string());
        let result = update_version(file.path(), &new_version);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No workspace or package section found"));
    }

    #[test]
    fn test_parse_invalid_toml() {
        let toml = r#"
            [package
            name = "invalid"
        "#;
        let file = write_temp_toml(toml);
        let result = parse(file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse"));
    }

    // Workspace inheritance edge cases
    #[test]
    fn test_workspace_inheritance_false() {
        let toml = r#"
            [package]
            name = "my-package"
            version = "1.0.0"
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Concrete("1.0.0".to_string()));
    }

    #[test]
    fn test_workspace_inheritance_with_other_fields() {
        let toml = r#"
            [package]
            name = "my-package"
            version = { workspace = true, optional = true }
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Inherited);
    }

    // Formatting preservation test
    #[test]
    fn test_update_version_preserves_formatting() {
        let toml = r#"
# This is a comment
[package]
name = "my-package"
version = "1.2.3"  # inline comment
description = "A test package"
        "#;
        let file = write_temp_toml(toml);
        let new_version = VersionField::Concrete("2.0.0".to_string());
        update_version(file.path(), &new_version).unwrap();
        let content = fs::read_to_string(file.path()).unwrap();

        // Check that version was updated
        assert!(content.contains("version = \"2.0.0\""));
        // Check that comments are preserved
        assert!(content.contains("# This is a comment"));
        assert!(content.contains("# inline comment"));
        // Check that other fields are preserved
        assert!(content.contains("description = \"A test package\""));
    }

    // File I/O error cases (harder to test, but worth mentioning)
    #[test]
    fn test_parse_nonexistent_file() {
        let result = parse(Path::new("/nonexistent/path/Cargo.toml"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read"));
    }

    #[test]
    fn test_parse_version_with_comments() {
        let toml = r#"
            [package]
            # This is a comment
            name = "my-package"
            # Version comment
            version = "1.2.3" # Inline comment
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Concrete("1.2.3".to_string()));
    }

    #[test]
    fn test_parse_version_with_whitespace() {
        let toml = r#"
            [package]
            name = "my-package"
            version = "1.2.3"  
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Concrete("1.2.3".to_string()));
    }

    #[test]
    fn test_parse_workspace_inheritance_invalid_value() {
        // Test that using a string "true" instead of boolean true for workspace inheritance
        // is rejected, as per Cargo.toml schema
        let toml = r#"
            [package]
            name = "my-package"
            version = { workspace = "true" }
        "#;
        let file = write_temp_toml(toml);
        let result = parse(file.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Version field must be a string"));
    }

    #[test]
    fn test_parse_workspace_inheritance_with_additional_fields() {
        let toml = r#"
            [package]
            name = "my-package"
            version = { workspace = true, other = "value" }
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Inherited);
    }

    #[test]
    fn test_parse_workspace_only_no_package() {
        let toml = r#"
            [workspace]
            members = ["crate1"]
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, None);
        assert_eq!(version, VersionField::Absent);
    }

    #[test]
    fn test_parse_both_workspace_and_package() {
        let toml = r#"
            [workspace.package]
            name = "workspace-package"
            version = "1.0.0"
            
            [package]
            name = "my-package"
            version = "2.0.0"
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("workspace-package".to_string()));
        assert_eq!(version, VersionField::Concrete("1.0.0".to_string()));
    }

    #[test]
    fn test_update_version_preserves_inline_table() {
        let toml = r#"
            [package]
            name = "my-package"
            version = { workspace = true, other = "value" }
        "#;
        let file = write_temp_toml(toml);
        let new_version = VersionField::Concrete("2.0.0".to_string());
        update_version(file.path(), &new_version).unwrap();
        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("version = \"2.0.0\""));
    }

    #[test]
    fn test_parse_version_invalid_semver() {
        let toml = r#"
            [package]
            name = "my-package"
            version = "not-a-version"
        "#;
        let file = write_temp_toml(toml);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Concrete("not-a-version".to_string()));
    }
}
