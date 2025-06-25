mod cargo_toml;
mod package_json;

use crate::domain::{Workspace, WorkspaceMember};
use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::path::Path;

/// Load the current workspace from the file system
///
/// This function discovers members from all supported ecosystems
/// and builds a composite workspace.
pub fn load_workspace() -> Result<Workspace> {
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let members = discover_members(&current_dir, |_| false)?;

    if members.is_empty() {
        anyhow::bail!("No supported package manifests found in current directory");
    }

    Ok(Workspace { members })
}

/// Save workspace changes back to the file system
///
/// This function delegates to the appropriate ecosystem-specific saver
/// based on the WorkspaceMember types.
pub fn save_workspace(workspace: &Workspace) -> Result<()> {
    for member in &workspace.members {
        match member {
            WorkspaceMember::Cargo { path, version, .. } => {
                cargo_toml::update_version(&path.join("Cargo.toml"), version)?;
            }
            WorkspaceMember::Node { path, version, .. } => {
                package_json::update_version(&path.join("package.json"), version)?;
            }
        }
    }
    Ok(())
}

pub fn discover_members<F>(root: &Path, ignore: F) -> Result<Vec<WorkspaceMember>>
where
    F: Fn(&Path) -> bool,
{
    if !root.exists() {
        return Err(anyhow::anyhow!(
            "Root path does not exist: {}",
            root.display()
        ));
    }

    let mut members = Vec::new();

    // Use WalkBuilder to respect .gitignore files
    for result in WalkBuilder::new(root)
        .hidden(false) // Don't ignore hidden files by default
        .git_ignore(true) // Respect .gitignore files
        .git_global(true) // Respect global git ignore
        .git_exclude(true) // Respect .git/info/exclude
        .build()
    {
        let entry = result.with_context(|| "Failed to walk directory tree")?;
        let path = entry.path();

        // Skip directories - we only care about files
        if entry.file_type().is_some_and(|ft| ft.is_dir()) {
            continue;
        }

        // Apply custom ignore filter
        if ignore(path) {
            continue;
        }

        let parent_path = path
            .parent()
            .with_context(|| format!("Invalid path structure: {}", path.display()))?;

        let basename = parent_path
            .file_name()
            .with_context(|| format!("Cannot determine directory name for {}", path.display()))?
            .to_string_lossy()
            .to_string();

        if path.file_name() == Some("Cargo.toml".as_ref()) {
            let (name, version) = cargo_toml::parse(path)?;

            members.push(WorkspaceMember::Cargo {
                name: name.unwrap_or(basename),
                path: parent_path.to_path_buf(),
                version,
            });
        } else if path.file_name() == Some("package.json".as_ref()) {
            let (name, version) = package_json::parse(path)?;

            members.push(WorkspaceMember::Node {
                name: name.unwrap_or(basename),
                path: parent_path.to_path_buf(),
                version,
            });
        }
    }

    members.sort_by(|a, b| a.name().cmp(b.name()));

    Ok(members)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::VersionField;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    fn write_file(path: &Path, contents: &str) {
        let mut file = File::create(path).unwrap();
        write!(file, "{}", contents).unwrap();
    }

    #[test]
    fn test_discover_members_basic() {
        let dir = tempdir().unwrap();
        let rust_dir = dir.path().join("rust");
        let node_dir = dir.path().join("node");
        fs::create_dir(&rust_dir).unwrap();
        fs::create_dir(&node_dir).unwrap();
        write_file(
            &rust_dir.join("Cargo.toml"),
            r#"[package]
name = "rustpkg"
version = "1.0.0"
"#,
        );
        write_file(
            &node_dir.join("package.json"),
            r#"{
  "name": "nodepkg",
  "version": "2.0.0"
}
"#,
        );

        let members = discover_members(dir.path(), |_| false).unwrap();
        assert_eq!(members.len(), 2);
        assert!(members.iter().any(|m| m.name() == "rustpkg"));
        assert!(members.iter().any(|m| m.name() == "nodepkg"));
    }

    #[test]
    fn test_discover_members_with_filter() {
        let dir = tempdir().unwrap();
        let skip_dir = dir.path().join("skipme");
        fs::create_dir(&skip_dir).unwrap();
        write_file(
            &skip_dir.join("Cargo.toml"),
            r#"[package]\nname = \"skip\"\nversion = \"0.1.0\"\n"#,
        );
        let members =
            discover_members(dir.path(), |p| p.to_string_lossy().contains("skipme")).unwrap();
        assert!(members.iter().all(|m| m.name() != "skip"));
    }

    #[test]
    fn test_discover_members_empty_ok() {
        let dir = tempdir().unwrap();
        let members = discover_members(dir.path(), |_| false).unwrap();
        assert!(members.is_empty());
    }

    #[test]
    fn test_discover_members_parse_error() {
        let dir = tempdir().unwrap();
        let bad = dir.path().join("bad");
        fs::create_dir(&bad).unwrap();
        write_file(&bad.join("Cargo.toml"), "not toml");
        let result = discover_members(dir.path(), |_| false);
        assert!(result.is_err());
    }

    #[test]
    fn test_discover_members_missing_names() {
        // Test packages without name fields use directory basename
        let dir = tempdir().unwrap();
        let rust_dir = dir.path().join("my-rust-package");
        fs::create_dir(&rust_dir).unwrap();
        write_file(
            &rust_dir.join("Cargo.toml"),
            "[package]\nversion = \"1.0.0\"",
        );

        let members = discover_members(dir.path(), |_| false).unwrap();
        assert_eq!(members[0].name(), "my-rust-package");
    }

    #[test]
    fn test_discover_members_workspace_inheritance() {
        // Test that workspace inheritance flag is detected correctly
        let dir = tempdir().unwrap();
        let pkg_dir = dir.path().join("pkg");
        fs::create_dir(&pkg_dir).unwrap();
        write_file(
            &pkg_dir.join("Cargo.toml"),
            "[package]\nname = \"pkg\"\nversion = { workspace = true }",
        );

        let members = discover_members(dir.path(), |_| false).unwrap();
        if let WorkspaceMember::Cargo { version, .. } = &members[0] {
            assert!(matches!(version, VersionField::Inherited));
        }
    }

    #[test]
    fn test_discover_members_nested_manifests() {
        // Test deeply nested structure
        let dir = tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();
        write_file(
            &nested.join("Cargo.toml"),
            "[package]\nname = \"nested\"\nversion = \"1.0.0\"",
        );

        let members = discover_members(dir.path(), |_| false).unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name(), "nested");
    }

    #[test]
    fn test_discover_members_mixed_ecosystems() {
        // Test finding both Rust and Node.js packages in the same directory
        let dir = tempdir().unwrap();
        let mixed_dir = dir.path().join("mixed");
        fs::create_dir(&mixed_dir).unwrap();

        // Create a Rust package
        write_file(
            &mixed_dir.join("Cargo.toml"),
            r#"[package]
name = "rust-pkg"
version = "1.0.0"
"#,
        );

        // Create a Node.js package
        write_file(
            &mixed_dir.join("package.json"),
            r#"{
  "name": "node-pkg",
  "version": "2.0.0"
}
"#,
        );

        let members = discover_members(dir.path(), |_| false).unwrap();
        assert_eq!(members.len(), 2);
        assert!(members.iter().any(|m| m.name() == "rust-pkg"));
        assert!(members.iter().any(|m| m.name() == "node-pkg"));
    }

    #[test]
    fn test_discover_members_invalid_path() {
        // Test handling of invalid path structure
        let result = discover_members(Path::new("/nonexistent/path"), |_| false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Root path does not exist"));
    }

    #[test]
    fn test_discover_members_symlinks() {
        // Test handling of symlinked directories
        let dir = tempdir().unwrap();
        let real_dir = dir.path().join("real");
        let symlink_dir = dir.path().join("symlink");
        fs::create_dir(&real_dir).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&real_dir, &symlink_dir).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&real_dir, &symlink_dir).unwrap();

        write_file(
            &real_dir.join("Cargo.toml"),
            r#"[package]
name = "symlinked"
version = "1.0.0"
"#,
        );

        let members = discover_members(dir.path(), |_| false).unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name(), "symlinked");
    }

    #[test]
    fn test_discover_members_duplicate_names() {
        // Test handling of packages with duplicate names
        let dir = tempdir().unwrap();
        let pkg1_dir = dir.path().join("pkg1");
        let pkg2_dir = dir.path().join("pkg2");
        fs::create_dir(&pkg1_dir).unwrap();
        fs::create_dir(&pkg2_dir).unwrap();

        write_file(
            &pkg1_dir.join("Cargo.toml"),
            r#"[package]
name = "duplicate"
version = "1.0.0"
"#,
        );
        write_file(
            &pkg2_dir.join("Cargo.toml"),
            r#"[package]
name = "duplicate"
version = "2.0.0"
"#,
        );

        let members = discover_members(dir.path(), |_| false).unwrap();
        assert_eq!(members.len(), 2);
        assert!(members.iter().all(|m| m.name() == "duplicate"));
    }

    #[test]
    fn test_discover_members_case_sensitivity() {
        // Test case sensitivity in file names
        let dir = tempdir().unwrap();
        let pkg_dir = dir.path().join("pkg");
        fs::create_dir(&pkg_dir).unwrap();
        write_file(
            &pkg_dir.join("CARGO.TOML"), // Note the uppercase
            r#"[package]
name = "case-sensitive"
version = "1.0.0"
"#,
        );

        let members = discover_members(dir.path(), |_| false).unwrap();
        assert_eq!(members.len(), 0); // Should not find uppercase Cargo.toml
    }

    #[test]
    fn test_discover_members_empty_manifest() {
        // Test handling of empty manifest files
        let dir = tempdir().unwrap();
        let pkg_dir = dir.path().join("pkg");
        fs::create_dir(&pkg_dir).unwrap();
        write_file(&pkg_dir.join("Cargo.toml"), "");

        let members = discover_members(dir.path(), |_| false).unwrap();
        assert_eq!(members.len(), 1);
        if let WorkspaceMember::Cargo { name, version, .. } = &members[0] {
            assert_eq!(name, "pkg");
            assert_eq!(version, &VersionField::Absent);
        }
    }

    #[test]
    fn test_discover_members_sorting() {
        // Test that members are properly sorted by name
        let dir = tempdir().unwrap();
        let pkg1_dir = dir.path().join("z-pkg");
        let pkg2_dir = dir.path().join("a-pkg");
        fs::create_dir(&pkg1_dir).unwrap();
        fs::create_dir(&pkg2_dir).unwrap();

        write_file(
            &pkg1_dir.join("Cargo.toml"),
            r#"[package]
name = "z-pkg"
version = "1.0.0"
"#,
        );
        write_file(
            &pkg2_dir.join("Cargo.toml"),
            r#"[package]
name = "a-pkg"
version = "2.0.0"
"#,
        );

        let members = discover_members(dir.path(), |_| false).unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].name(), "a-pkg");
        assert_eq!(members[1].name(), "z-pkg");
    }
}
