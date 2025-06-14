use anyhow::Context;
use semver;
use serde::Serialize;
use std::path::PathBuf;

/// Domain types for version management operations
#[derive(Debug, Clone, PartialEq)]
pub enum VersionBump {
    Major(i32),
    Minor(i32),
    Patch(i32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum VersionField {
    Absent,
    Concrete(String),
    Inherited,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageSelection {
    pub packages: Vec<String>,
    pub workspace: bool,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionChange {
    pub package: String,
    pub old_version: String,
    pub new_version: String,
    pub path: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct OperationResult {
    pub changes: Vec<VersionChange>,
    pub operation: String,
}

impl OperationResult {
    pub fn new(operation: String) -> Self {
        Self {
            changes: Vec::new(),
            operation,
        }
    }

    pub fn add_change(&mut self, change: VersionChange) {
        self.changes.push(change);
    }

    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }
}

/// A workspace member, which can be a Rust package or a Node.js package
#[derive(Debug, Clone)]
pub enum WorkspaceMember {
    Cargo {
        name: String,
        path: PathBuf,
        version: VersionField,
    },
    Node {
        name: String,
        path: PathBuf,
        version: VersionField,
    },
}

impl WorkspaceMember {
    /// Get the name of the package
    pub fn name(&self) -> &str {
        match self {
            WorkspaceMember::Cargo { name, .. } => name,
            WorkspaceMember::Node { name, .. } => name,
        }
    }

    /// Get the path to the package
    pub fn path(&self) -> &PathBuf {
        match self {
            WorkspaceMember::Cargo { path, .. } => path,
            WorkspaceMember::Node { path, .. } => path,
        }
    }

    /// Get the version of the package
    pub fn version(&self) -> &VersionField {
        match self {
            WorkspaceMember::Cargo { version, .. } => version,
            WorkspaceMember::Node { version, .. } => version,
        }
    }

    /// Set the version of the package
    pub fn set_version(&mut self, new_version: VersionField) {
        match self {
            WorkspaceMember::Cargo { version, .. } => *version = new_version,
            WorkspaceMember::Node { version, .. } => *version = new_version,
        }
    }
}

/// A workspace, which is a collection of packages
#[derive(Debug, Clone)]
pub struct Workspace {
    /// The members of the workspace
    pub members: Vec<WorkspaceMember>,
}

impl Workspace {
    pub fn roll_version(
        &mut self,
        bump: VersionBump,
        selection: &PackageSelection,
    ) -> anyhow::Result<OperationResult> {
        let mut result = OperationResult::new(format!(
            "roll {}",
            match bump {
                VersionBump::Major(amount) => format!("major {}", amount),
                VersionBump::Minor(amount) => format!("minor {}", amount),
                VersionBump::Patch(amount) => format!("patch {}", amount),
            }
        ));

        let indices = self.select_member_indices(selection)?;
        for &index in &indices {
            let member = &mut self.members[index];

            let old_version = match member.version() {
                VersionField::Concrete(version) => version.clone(),
                _ => continue,
            };

            let new_version = bump.apply_to_version(&old_version)?;

            if old_version != new_version {
                result.add_change(VersionChange {
                    package: member.name().to_string(),
                    old_version: old_version.clone(),
                    new_version: new_version.clone(),
                    path: member.path().clone(),
                });

                member.set_version(VersionField::Concrete(new_version));
            }
        }

        Ok(result)
    }

    pub fn set_version(
        &mut self,
        version: &str,
        selection: &PackageSelection,
    ) -> anyhow::Result<OperationResult> {
        let mut result = OperationResult::new(format!("set {}", version));

        let indices = self.select_member_indices(selection)?;

        for &index in &indices {
            let member = &mut self.members[index];

            let old_version = match member.version() {
                VersionField::Concrete(version) => version.clone(),
                _ => continue,
            };

            if old_version != version {
                result.add_change(VersionChange {
                    package: member.name().to_string(),
                    old_version: old_version.clone(),
                    new_version: version.to_string(),
                    path: member.path().clone(),
                });

                member.set_version(VersionField::Concrete(version.to_string()));
            }
        }

        Ok(result)
    }

    pub fn sync_version(&mut self, version: &str) -> anyhow::Result<OperationResult> {
        let mut result = OperationResult::new(format!("sync {}", version));

        for member in &mut self.members {
            let old_version = match member.version() {
                VersionField::Concrete(version) => version.clone(),
                _ => continue,
            };

            if old_version != version {
                result.add_change(VersionChange {
                    package: member.name().to_string(),
                    old_version: old_version.clone(),
                    new_version: version.to_string(),
                    path: member.path().clone(),
                });

                member.set_version(VersionField::Concrete(version.to_string()));
            }
        }
        Ok(result)
    }

    pub fn show(&self, selection: &PackageSelection) -> String {
        let mut output = String::new();
        for member in self.selected_members(selection) {
            let version = match member.version() {
                VersionField::Concrete(version) => version.clone(),
                _ => continue,
            };

            output.push_str(&format!("{}: {}\n", member.name(), version));
        }
        output
    }

    pub fn lint(&self, selection: &PackageSelection) -> Vec<LintError> {
        let mut errors = Vec::new();
        for member in self.selected_members(selection) {
            let version = match member.version() {
                VersionField::Concrete(version) => version.clone(),
                _ => continue,
            };

            if let Err(e) = semver::Version::parse(&version) {
                errors.push(LintError {
                    member: member.name().to_string(),
                    message: format!("Invalid version '{}': {}", version, e),
                });
            }
        }
        errors
    }

    pub fn selected_members(&self, selection: &PackageSelection) -> Vec<&WorkspaceMember> {
        let members: Vec<&WorkspaceMember> = match self.select_member_indices(selection) {
            Ok(indices) => indices.iter().map(|&i| &self.members[i]).collect(),
            Err(_) => self.members.iter().collect(), // fallback to all members
        };
        let mut sorted = members;
        sorted.sort_by(|a, b| a.name().cmp(b.name()));
        sorted
    }

    fn select_member_indices(&self, selection: &PackageSelection) -> anyhow::Result<Vec<usize>> {
        if !selection.packages.is_empty() {
            // Select specific packages, excluding any in the exclude list
            let mut indices = Vec::new();
            for package_name in &selection.packages {
                if selection.exclude.iter().any(|e| e == package_name) {
                    continue;
                }
                match self.members.iter().position(|m| m.name() == *package_name) {
                    Some(index) => indices.push(index),
                    None => anyhow::bail!("Package '{}' not found in workspace", package_name),
                }
            }
            Ok(indices)
        } else if selection.workspace {
            // Select all workspace members, excluding any in the exclude list
            Ok(self
                .members
                .iter()
                .enumerate()
                .filter(|(_, m)| !selection.exclude.iter().any(|e| e == m.name()))
                .map(|(i, _)| i)
                .collect())
        } else {
            // Default: select the first member (root package in single crate, or workspace root)
            // but only if it's not excluded
            if self.members.is_empty() {
                anyhow::bail!("No packages found in workspace")
            } else if selection
                .exclude
                .iter()
                .any(|e| e == self.members[0].name())
            {
                Ok(vec![])
            } else {
                Ok(vec![0])
            }
        }
    }
}

#[derive(Debug)]
pub struct LintError {
    pub member: String,
    pub message: String,
}

impl VersionBump {
    pub fn apply_to_version(&self, current: &str) -> anyhow::Result<String> {
        let mut version = semver::Version::parse(current)
            .with_context(|| format!("Invalid semver version: '{}'", current))?;

        match self {
            VersionBump::Major(amount) => {
                if *amount < 0 {
                    let abs_amount = amount.unsigned_abs() as u64;
                    if version.major < abs_amount {
                        anyhow::bail!(
                            "Cannot decrement major version by {} from {}: would result in negative version",
                            abs_amount, current
                        );
                    }
                    version.major -= abs_amount;
                } else {
                    version.major += *amount as u64;
                }
                version.minor = 0;
                version.patch = 0;
            }
            VersionBump::Minor(amount) => {
                if *amount < 0 {
                    let abs_amount = amount.unsigned_abs() as u64;
                    if version.minor < abs_amount {
                        anyhow::bail!(
                            "Cannot decrement minor version by {} from {}: would result in negative version",
                            abs_amount, current
                        );
                    }
                    version.minor -= abs_amount;
                } else {
                    version.minor += *amount as u64;
                }
                version.patch = 0;
            }
            VersionBump::Patch(amount) => {
                if *amount < 0 {
                    let abs_amount = amount.unsigned_abs() as u64;
                    if version.patch < abs_amount {
                        anyhow::bail!(
                            "Cannot decrement patch version by {} from {}: would result in negative version",
                            abs_amount, current
                        );
                    }
                    version.patch -= abs_amount;
                } else {
                    version.patch += *amount as u64;
                }
            }
        }

        Ok(version.to_string())
    }
}

impl PackageSelection {
    #[cfg(test)]
    pub fn workspace() -> Self {
        Self {
            packages: vec![],
            workspace: true,
            exclude: vec![],
        }
    }

    #[cfg(test)]
    pub fn root_only() -> Self {
        Self {
            packages: vec![],
            workspace: false,
            exclude: vec![],
        }
    }

    #[cfg(test)]
    pub fn packages(packages: Vec<String>) -> Self {
        Self {
            packages,
            workspace: false,
            exclude: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_member(name: &str, version: VersionField) -> WorkspaceMember {
        WorkspaceMember::Cargo {
            name: name.to_string(),
            path: PathBuf::from(format!("{}/Cargo.toml", name)),
            version,
        }
    }

    fn create_test_workspace(members: Vec<(&str, VersionField)>) -> Workspace {
        Workspace {
            members: members
                .into_iter()
                .map(|(name, version)| create_test_member(name, version))
                .collect(),
        }
    }

    #[test]
    fn test_version_bump_patch() {
        let bump = VersionBump::Patch(1);
        assert_eq!(bump.apply_to_version("1.0.0").unwrap(), "1.0.1");
        assert_eq!(bump.apply_to_version("0.5.9").unwrap(), "0.5.10");
    }

    #[test]
    fn test_version_bump_patch_custom_amount() {
        let bump = VersionBump::Patch(5);
        assert_eq!(bump.apply_to_version("1.0.0").unwrap(), "1.0.5");

        let bump = VersionBump::Patch(-2);
        assert_eq!(bump.apply_to_version("1.0.5").unwrap(), "1.0.3");
    }

    #[test]
    fn test_version_bump_minor() {
        let bump = VersionBump::Minor(1);
        assert_eq!(bump.apply_to_version("1.5.3").unwrap(), "1.6.0");

        let bump = VersionBump::Minor(3);
        assert_eq!(bump.apply_to_version("0.1.0").unwrap(), "0.4.0");
    }

    #[test]
    fn test_version_bump_major() {
        let bump = VersionBump::Major(1);
        assert_eq!(bump.apply_to_version("1.5.3").unwrap(), "2.0.0");

        let bump = VersionBump::Major(2);
        assert_eq!(bump.apply_to_version("0.1.0").unwrap(), "2.0.0");
    }

    #[test]
    fn test_version_bump_negative_errors() {
        // Test that operations resulting in negative versions return errors
        let bump = VersionBump::Patch(-10);
        let result = bump.apply_to_version("1.0.3");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("would result in negative version"));

        let bump = VersionBump::Minor(-5);
        let result = bump.apply_to_version("1.2.0");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("would result in negative version"));

        let bump = VersionBump::Major(-10);
        let result = bump.apply_to_version("2.0.0");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("would result in negative version"));
    }

    #[test]
    fn test_version_bump_patch_invalid_operations() {
        // Test the exact scenario we saw: patch -2 on 0.1.0 should error
        let bump = VersionBump::Patch(-2);
        let result = bump.apply_to_version("0.1.0");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot decrement patch version by 2 from 0.1.0"));

        // Also test patch -1 on 0.1.0 (should also error)
        let bump = VersionBump::Patch(-1);
        let result = bump.apply_to_version("0.1.0");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot decrement patch version by 1 from 0.1.0"));

        // Test that positive still works
        let bump = VersionBump::Patch(2);
        assert_eq!(bump.apply_to_version("0.1.0").unwrap(), "0.1.2");
    }

    #[test]
    fn test_workspace_member_methods() {
        let mut member =
            create_test_member("test-crate", VersionField::Concrete("1.0.0".to_string()));

        assert_eq!(member.name(), "test-crate");
        assert_eq!(
            member.version(),
            &VersionField::Concrete("1.0.0".to_string())
        );

        member.set_version(VersionField::Concrete("2.0.0".to_string()));
        assert_eq!(
            member.version(),
            &VersionField::Concrete("2.0.0".to_string())
        );
    }

    #[test]
    fn test_workspace_roll_version_default() {
        let mut workspace = create_test_workspace(vec![
            ("app", VersionField::Concrete("1.0.0".to_string())),
            ("lib", VersionField::Concrete("0.5.0".to_string())),
        ]);

        // Default selection (first member only)
        let selection = PackageSelection::root_only();
        workspace
            .roll_version(VersionBump::Patch(1), &selection)
            .unwrap();

        assert_eq!(
            workspace.members[0].version(),
            &VersionField::Concrete("1.0.1".to_string())
        ); // app bumped
        assert_eq!(
            workspace.members[1].version(),
            &VersionField::Concrete("0.5.0".to_string())
        ); // lib unchanged
    }

    #[test]
    fn test_workspace_roll_version_specific_packages() {
        let mut workspace = create_test_workspace(vec![
            ("app", VersionField::Concrete("1.0.0".to_string())),
            ("lib", VersionField::Concrete("0.5.0".to_string())),
            ("utils", VersionField::Concrete("0.1.0".to_string())),
        ]);

        let selection = PackageSelection::packages(vec!["lib".to_string(), "utils".to_string()]);

        workspace
            .roll_version(VersionBump::Minor(1), &selection)
            .unwrap();

        assert_eq!(
            workspace.members[0].version(),
            &VersionField::Concrete("1.0.0".to_string())
        ); // app unchanged
        assert_eq!(
            workspace.members[1].version(),
            &VersionField::Concrete("0.6.0".to_string())
        ); // lib bumped
        assert_eq!(
            workspace.members[2].version(),
            &VersionField::Concrete("0.2.0".to_string())
        ); // utils bumped
    }

    #[test]
    fn test_workspace_roll_version_workspace() {
        let mut workspace = create_test_workspace(vec![
            ("app", VersionField::Concrete("1.0.0".to_string())),
            ("lib", VersionField::Concrete("0.5.0".to_string())),
            ("utils", VersionField::Concrete("0.1.0".to_string())),
        ]);

        let selection = PackageSelection::workspace();
        workspace
            .roll_version(VersionBump::Patch(1), &selection)
            .unwrap();

        assert_eq!(
            workspace.members[0].version(),
            &VersionField::Concrete("1.0.1".to_string())
        ); // app bumped

        assert_eq!(
            workspace.members[1].version(),
            &VersionField::Concrete("0.5.1".to_string())
        ); // lib bumped

        assert_eq!(
            workspace.members[2].version(),
            &VersionField::Concrete("0.1.1".to_string())
        ); // utils bumped
    }

    #[test]
    fn test_workspace_roll_version_workspace_with_exclude() {
        let mut workspace = create_test_workspace(vec![
            ("app", VersionField::Concrete("1.0.0".to_string())),
            ("lib", VersionField::Concrete("0.5.0".to_string())),
            ("utils", VersionField::Concrete("0.1.0".to_string())),
        ]);

        let selection = PackageSelection {
            packages: vec![],
            workspace: true,
            exclude: vec!["lib".to_string()],
        };

        workspace
            .roll_version(VersionBump::Patch(1), &selection)
            .unwrap();

        assert_eq!(
            workspace.members[0].version(),
            &VersionField::Concrete("1.0.1".to_string())
        ); // app bumped
        assert_eq!(
            workspace.members[1].version(),
            &VersionField::Concrete("0.5.0".to_string())
        ); // lib excluded

        assert_eq!(
            workspace.members[2].version(),
            &VersionField::Concrete("0.1.1".to_string())
        ); // utils bumped
    }

    #[test]
    fn test_workspace_set_version() {
        let mut workspace = create_test_workspace(vec![
            ("app", VersionField::Concrete("1.0.0".to_string())),
            ("lib", VersionField::Concrete("0.5.0".to_string())),
        ]);

        let selection = PackageSelection::packages(vec!["lib".to_string()]);
        workspace.set_version("2.0.0", &selection).unwrap();

        assert_eq!(
            workspace.members[0].version(),
            &VersionField::Concrete("1.0.0".to_string())
        ); // app unchanged

        assert_eq!(
            workspace.members[1].version(),
            &VersionField::Concrete("2.0.0".to_string())
        ); // lib set
    }

    #[test]
    fn test_workspace_sync_version() {
        let mut workspace = create_test_workspace(vec![
            ("app", VersionField::Concrete("1.0.0".to_string())),
            ("lib", VersionField::Concrete("0.5.0".to_string())),
            ("utils", VersionField::Concrete("2.1.0".to_string())),
        ]);

        workspace.sync_version("1.5.0").unwrap();

        assert_eq!(
            workspace.members[0].version(),
            &VersionField::Concrete("1.5.0".to_string())
        );

        assert_eq!(
            workspace.members[1].version(),
            &VersionField::Concrete("1.5.0".to_string())
        );

        assert_eq!(
            workspace.members[2].version(),
            &VersionField::Concrete("1.5.0".to_string())
        );
    }

    #[test]
    fn test_workspace_show() {
        let workspace = create_test_workspace(vec![
            ("app", VersionField::Concrete("1.0.0".to_string())),
            ("lib", VersionField::Concrete("0.5.0".to_string())),
        ]);

        let output = workspace.show(&PackageSelection::workspace());
        assert_eq!(output, "app: 1.0.0\nlib: 0.5.0\n");
    }

    #[test]
    fn test_workspace_lint_valid() {
        let workspace = create_test_workspace(vec![
            ("app", VersionField::Concrete("1.0.0".to_string())),
            ("lib", VersionField::Concrete("0.5.0".to_string())),
        ]);

        let errors = workspace.lint(&PackageSelection::root_only());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_workspace_lint_invalid() {
        let workspace = create_test_workspace(vec![
            ("app", VersionField::Concrete("1.0.0".to_string())),
            ("lib", VersionField::Concrete("invalid-version".to_string())),
        ]);

        let errors = workspace.lint(&PackageSelection::workspace());
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].member, "lib");
        assert!(errors[0]
            .message
            .contains("Invalid version 'invalid-version'"));
    }

    #[test]
    fn test_package_selection_not_found() {
        let mut workspace =
            create_test_workspace(vec![("app", VersionField::Concrete("1.0.0".to_string()))]);

        let selection = PackageSelection::packages(vec!["nonexistent".to_string()]);
        let result = workspace.roll_version(VersionBump::Patch(1), &selection);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Package 'nonexistent' not found"));
    }

    #[test]
    fn test_workspace_roll_version_invalid_operation() {
        // Test what happens when a roll operation would result in invalid versions
        let mut workspace = create_test_workspace(vec![
            ("app", VersionField::Concrete("0.1.0".to_string())),
            ("lib", VersionField::Concrete("0.1.0".to_string())),
        ]);

        let selection = PackageSelection::workspace();
        let result = workspace.roll_version(VersionBump::Patch(-2), &selection);

        // Should return an error explaining why the operation failed
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot decrement patch version by 2 from 0.1.0"));

        // Versions should remain unchanged
        assert_eq!(
            workspace.members[0].version(),
            &VersionField::Concrete("0.1.0".to_string())
        );

        assert_eq!(
            workspace.members[1].version(),
            &VersionField::Concrete("0.1.0".to_string())
        );
    }

    #[test]
    fn test_version_bump_cross_component_boundaries() {
        // Test that we don't "borrow" from higher components
        let bump = VersionBump::Patch(-1);
        let result = bump.apply_to_version("1.2.0");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot decrement patch version by 1 from 1.2.0"));

        let bump = VersionBump::Minor(-1);
        let result = bump.apply_to_version("1.0.5");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot decrement minor version by 1 from 1.0.5"));
    }

    #[test]
    fn test_version_bump_absolute_minimum() {
        // Test rolling back from 0.0.0
        let bump = VersionBump::Patch(-1);
        let result = bump.apply_to_version("0.0.0");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot decrement patch version by 1 from 0.0.0"));

        let bump = VersionBump::Minor(-1);
        let result = bump.apply_to_version("0.0.0");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot decrement minor version by 1 from 0.0.0"));

        let bump = VersionBump::Major(-1);
        let result = bump.apply_to_version("0.0.0");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot decrement major version by 1 from 0.0.0"));
    }

    #[test]
    fn test_workspace_roll_version_mixed_validity() {
        // Test workspace with mixed versions where some can handle rollback and others can't
        let mut workspace = create_test_workspace(vec![
            ("app", VersionField::Concrete("1.0.2".to_string())), // Can handle patch -2
            ("lib", VersionField::Concrete("0.1.0".to_string())), // Cannot handle patch -2 (will cause error)
            ("utils", VersionField::Concrete("2.5.3".to_string())), // Can handle patch -2
        ]);

        let selection = PackageSelection::workspace();
        let result = workspace.roll_version(VersionBump::Patch(-2), &selection);

        // Should fail on the invalid package
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot decrement patch version by 2 from 0.1.0"));

        // Note: Even though first package gets modified in-memory during processing,
        // the CLI layer won't save the workspace when domain operations fail,
        // so from the user's perspective, no changes occur to disk
        //
        // However, this test is only exercising domain logic, so we see the in-memory changes:
        assert_eq!(
            workspace.members[0].version(),
            &VersionField::Concrete("1.0.0".to_string()),
            "app was modified in-memory"
        );

        assert_eq!(
            workspace.members[1].version(),
            &VersionField::Concrete("0.1.0".to_string()),
            "lib unchanged (caused error)"
        );

        assert_eq!(
            workspace.members[2].version(),
            &VersionField::Concrete("2.5.3".to_string()),
            "utils unchanged (not processed)"
        );
    }

    #[test]
    fn test_version_bump_large_negative_numbers() {
        // Test very large negative numbers
        let bump = VersionBump::Patch(-999999);
        let result = bump.apply_to_version("1.0.5");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot decrement patch version by 999999 from 1.0.5"));
    }

    #[test]
    fn test_version_bump_prerelease_versions() {
        // Test how pre-release versions behave with rollback
        let bump = VersionBump::Patch(-1);

        // Rolling back from 1.0.1-alpha preserves the pre-release identifier
        assert_eq!(bump.apply_to_version("1.0.1-alpha").unwrap(), "1.0.0-alpha");

        // Rolling back from 1.0.0-alpha should fail (can't go to 0.-1.0-alpha)
        let result = bump.apply_to_version("1.0.0-alpha");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot decrement patch version by 1 from 1.0.0-alpha"));

        // Test minor rollback with pre-release (resets patch to 0, preserves pre-release)
        let bump = VersionBump::Minor(-1);
        assert_eq!(bump.apply_to_version("1.1.5-beta").unwrap(), "1.0.0-beta");

        // Test major rollback with pre-release (resets minor and patch to 0, preserves pre-release)
        let bump = VersionBump::Major(-1);
        assert_eq!(bump.apply_to_version("2.3.5-rc.1").unwrap(), "1.0.0-rc.1");
    }

    #[test]
    fn test_selected_members_sorting() {
        let workspace = create_test_workspace(vec![
            ("zebra", VersionField::Concrete("1.0.0".to_string())),
            ("apple", VersionField::Concrete("2.0.0".to_string())),
            ("banana", VersionField::Concrete("3.0.0".to_string())),
        ]);

        let members = workspace.selected_members(&PackageSelection::workspace());
        assert_eq!(members[0].name(), "apple");
        assert_eq!(members[1].name(), "banana");
        assert_eq!(members[2].name(), "zebra");
    }

    #[test]
    fn test_selected_members_sorting_with_selection() {
        let workspace = create_test_workspace(vec![
            ("zebra", VersionField::Concrete("1.0.0".to_string())),
            ("apple", VersionField::Concrete("2.0.0".to_string())),
            ("banana", VersionField::Concrete("3.0.0".to_string())),
        ]);

        let selection = PackageSelection::packages(vec!["zebra".to_string(), "apple".to_string()]);
        let members = workspace.selected_members(&selection);
        assert_eq!(members[0].name(), "apple");
        assert_eq!(members[1].name(), "zebra");
    }

    #[test]
    fn test_version_bump_with_build_metadata() {
        let bump = VersionBump::Patch(1);
        let result = bump.apply_to_version("1.2.3+20130313144700").unwrap();
        assert_eq!(result, "1.2.4+20130313144700");
    }

    #[test]
    fn test_version_bump_with_prerelease_and_build() {
        let bump = VersionBump::Patch(1);
        let result = bump
            .apply_to_version("1.2.3-beta.1+20130313144700")
            .unwrap();
        assert_eq!(result, "1.2.4-beta.1+20130313144700");
    }

    #[test]
    fn test_version_bump_zero_amount() {
        let bump = VersionBump::Patch(0);
        let result = bump.apply_to_version("1.2.3").unwrap();
        assert_eq!(result, "1.2.3");
    }

    #[test]
    fn test_workspace_roll_version_preserves_inherited() {
        let mut workspace = create_test_workspace(vec![
            ("pkg1", VersionField::Concrete("1.0.0".to_string())),
            ("pkg2", VersionField::Inherited),
        ]);
        let selection = PackageSelection::workspace();
        let result = workspace
            .roll_version(VersionBump::Patch(1), &selection)
            .unwrap();
        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.changes[0].package, "pkg1");
        assert_eq!(result.changes[0].new_version, "1.0.1");
    }

    #[test]
    fn test_workspace_set_version_preserves_inherited() {
        let mut workspace = create_test_workspace(vec![
            ("pkg1", VersionField::Concrete("1.0.0".to_string())),
            ("pkg2", VersionField::Inherited),
        ]);
        let selection = PackageSelection::workspace();
        let result = workspace.set_version("2.0.0", &selection).unwrap();
        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.changes[0].package, "pkg1");
        assert_eq!(result.changes[0].new_version, "2.0.0");
    }

    #[test]
    fn test_workspace_sync_version_preserves_inherited() {
        let mut workspace = create_test_workspace(vec![
            ("pkg1", VersionField::Concrete("1.0.0".to_string())),
            ("pkg2", VersionField::Inherited),
            ("pkg3", VersionField::Concrete("2.0.0".to_string())),
        ]);
        let result = workspace.sync_version("3.0.0").unwrap();
        assert_eq!(result.changes.len(), 2);
        assert!(result
            .changes
            .iter()
            .any(|c| c.package == "pkg1" && c.new_version == "3.0.0"));
        assert!(result
            .changes
            .iter()
            .any(|c| c.package == "pkg3" && c.new_version == "3.0.0"));
    }

    #[test]
    fn test_workspace_show_includes_inherited() {
        let workspace = create_test_workspace(vec![
            ("pkg1", VersionField::Concrete("1.0.0".to_string())),
            ("pkg2", VersionField::Inherited),
        ]);
        let selection = PackageSelection::workspace();
        let output = workspace.show(&selection);
        assert!(output.contains("pkg1: 1.0.0"));
        assert!(!output.contains("pkg2")); // Inherited versions are skipped
    }

    #[test]
    fn test_workspace_lint_skips_inherited() {
        let workspace = create_test_workspace(vec![
            ("pkg1", VersionField::Concrete("1.0.0".to_string())),
            ("pkg2", VersionField::Inherited),
            ("pkg3", VersionField::Concrete("invalid".to_string())),
        ]);
        let selection = PackageSelection::workspace();
        let errors = workspace.lint(&selection);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].member, "pkg3");
    }

    #[test]
    fn test_package_selection_exclude_all() {
        let workspace = create_test_workspace(vec![
            ("pkg1", VersionField::Concrete("1.0.0".to_string())),
            ("pkg2", VersionField::Concrete("2.0.0".to_string())),
            ("pkg3", VersionField::Concrete("3.0.0".to_string())),
        ]);
        let selection = PackageSelection {
            packages: vec![],
            workspace: false,
            exclude: vec!["pkg1".to_string(), "pkg2".to_string(), "pkg3".to_string()],
        };
        let members = workspace.selected_members(&selection);
        assert!(members.is_empty());
    }

    #[test]
    fn test_package_selection_include_and_exclude() {
        let workspace = create_test_workspace(vec![
            ("pkg1", VersionField::Concrete("1.0.0".to_string())),
            ("pkg2", VersionField::Concrete("2.0.0".to_string())),
            ("pkg3", VersionField::Concrete("3.0.0".to_string())),
        ]);
        let selection = PackageSelection {
            packages: vec!["pkg1".to_string(), "pkg2".to_string()],
            workspace: false,
            exclude: vec!["pkg2".to_string()],
        };
        let members = workspace.selected_members(&selection);
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name(), "pkg1");
    }

    #[test]
    fn test_operation_result_has_changes() {
        let mut result = OperationResult::new("test".to_string());
        assert!(!result.has_changes());

        result.add_change(VersionChange {
            package: "pkg1".to_string(),
            old_version: "1.0.0".to_string(),
            new_version: "2.0.0".to_string(),
            path: PathBuf::from("pkg1"),
        });
        assert!(result.has_changes());
    }
}
