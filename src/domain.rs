use anyhow::Context;
use std::path::PathBuf;

/// Domain types for version management operations
#[derive(Debug, Clone)]
pub enum VersionBump {
    Major(i32),
    Minor(i32),
    Patch(i32),
}

#[derive(Debug, Clone)]
pub struct PackageSelection {
    pub packages: Vec<String>,
    pub workspace: bool,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum WorkspaceMember {
    Cargo {
        name: String,
        version: String,
        path: PathBuf,
        has_workspace_inheritance: bool,
    },
    // Node {
    //     name: String,
    //     version: String,
    //     path: PathBuf,
    //     // private: bool,
    //     // workspaces: Option<Vec<String>>,
    // },
    // Python {
    //     name: String,
    //     version: String,
    //     path: PathBuf,
    //     // is_pyproject_toml: bool,
    // },
}

impl WorkspaceMember {
    pub fn name(&self) -> &str {
        match self {
            WorkspaceMember::Cargo { name, .. } => name,
            // WorkspaceMember::Node { name, .. } => name,
            // WorkspaceMember::Python { name, .. } => name,
        }
    }

    pub fn version(&self) -> &str {
        match self {
            WorkspaceMember::Cargo { version, .. } => version,
            // WorkspaceMember::Node { version, .. } => version,
            // WorkspaceMember::Python { version, .. } => version,
        }
    }

    pub fn set_version(&mut self, new_version: &str) {
        match self {
            WorkspaceMember::Cargo { version, .. } => *version = new_version.to_string(),
            // WorkspaceMember::Node { version, .. } => *version = new_version.to_string(),
            // WorkspaceMember::Python { version, .. } => *version = new_version.to_string(),
        }
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            WorkspaceMember::Cargo { path, .. } => path,
            // WorkspaceMember::Node { path, .. } => path,
            // WorkspaceMember::Python { path, .. } => path,
        }
    }
}

#[derive(Debug)]
pub struct Workspace {
    pub members: Vec<WorkspaceMember>,
}

impl Workspace {
    pub fn roll_version(
        &mut self,
        bump: VersionBump,
        selection: &PackageSelection,
    ) -> anyhow::Result<()> {
        let indices = self.select_member_indices(selection)?;
        for &index in &indices {
            let current_version = self.members[index].version().to_string();
            let new_version = bump.apply_to_version(&current_version)?;
            self.members[index].set_version(&new_version);
        }
        Ok(())
    }

    pub fn set_version(
        &mut self,
        version: &str,
        selection: &PackageSelection,
    ) -> anyhow::Result<()> {
        let indices = self.select_member_indices(selection)?;
        for &index in &indices {
            self.members[index].set_version(version);
        }
        Ok(())
    }

    pub fn sync_version(&mut self, version: &str) -> anyhow::Result<()> {
        // Sync sets ALL members to the same version (lockstep)
        for member in &mut self.members {
            member.set_version(version);
        }
        Ok(())
    }

    pub fn show(&self) -> String {
        let mut output = String::new();
        for member in &self.members {
            output.push_str(&format!("{} {}\n", member.name(), member.version()));
        }
        output
    }

    pub fn lint(&self) -> Vec<LintError> {
        let mut errors = Vec::new();
        for member in &self.members {
            if let Err(e) = semver::Version::parse(member.version()) {
                errors.push(LintError {
                    member: member.name().to_string(),
                    message: format!("Invalid version '{}': {}", member.version(), e),
                });
            }
        }
        errors
    }

    fn select_member_indices(&self, selection: &PackageSelection) -> anyhow::Result<Vec<usize>> {
        if !selection.packages.is_empty() {
            // Select specific packages
            let mut indices = Vec::new();
            for package_name in &selection.packages {
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
            if self.members.is_empty() {
                anyhow::bail!("No packages found in workspace")
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
                    version.major = version.major.saturating_sub(amount.unsigned_abs() as u64);
                } else {
                    version.major = version.major.saturating_add(*amount as u64);
                }
                version.minor = 0;
                version.patch = 0;
            }
            VersionBump::Minor(amount) => {
                if *amount < 0 {
                    version.minor = version.minor.saturating_sub(amount.unsigned_abs() as u64);
                } else {
                    version.minor = version.minor.saturating_add(*amount as u64);
                }
                version.patch = 0;
            }
            VersionBump::Patch(amount) => {
                if *amount < 0 {
                    version.patch = version.patch.saturating_sub(amount.unsigned_abs() as u64);
                } else {
                    version.patch = version.patch.saturating_add(*amount as u64);
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

    fn create_test_member(name: &str, version: &str) -> WorkspaceMember {
        WorkspaceMember::Cargo {
            name: name.to_string(),
            version: version.to_string(),
            path: PathBuf::from(format!("{}/Cargo.toml", name)),
            has_workspace_inheritance: false,
        }
    }

    fn create_test_workspace(members: Vec<(&str, &str)>) -> Workspace {
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
    fn test_version_bump_negative_saturating() {
        // Test saturating subtraction (doesn't go below 0)
        let bump = VersionBump::Patch(-10);
        assert_eq!(bump.apply_to_version("1.0.3").unwrap(), "1.0.0");

        let bump = VersionBump::Minor(-5);
        assert_eq!(bump.apply_to_version("1.2.0").unwrap(), "1.0.0");

        let bump = VersionBump::Major(-10);
        assert_eq!(bump.apply_to_version("2.0.0").unwrap(), "0.0.0");
    }

    #[test]
    fn test_workspace_member_methods() {
        let mut member = create_test_member("test-crate", "1.0.0");

        assert_eq!(member.name(), "test-crate");
        assert_eq!(member.version(), "1.0.0");

        member.set_version("2.0.0");
        assert_eq!(member.version(), "2.0.0");
    }

    #[test]
    fn test_workspace_roll_version_default() {
        let mut workspace = create_test_workspace(vec![("app", "1.0.0"), ("lib", "0.5.0")]);

        // Default selection (first member only)
        let selection = PackageSelection::root_only();
        workspace
            .roll_version(VersionBump::Patch(1), &selection)
            .unwrap();

        assert_eq!(workspace.members[0].version(), "1.0.1"); // app bumped
        assert_eq!(workspace.members[1].version(), "0.5.0"); // lib unchanged
    }

    #[test]
    fn test_workspace_roll_version_specific_packages() {
        let mut workspace =
            create_test_workspace(vec![("app", "1.0.0"), ("lib", "0.5.0"), ("utils", "0.1.0")]);

        let selection = PackageSelection::packages(vec!["lib".to_string(), "utils".to_string()]);
        workspace
            .roll_version(VersionBump::Minor(1), &selection)
            .unwrap();

        assert_eq!(workspace.members[0].version(), "1.0.0"); // app unchanged
        assert_eq!(workspace.members[1].version(), "0.6.0"); // lib bumped
        assert_eq!(workspace.members[2].version(), "0.2.0"); // utils bumped
    }

    #[test]
    fn test_workspace_roll_version_workspace() {
        let mut workspace =
            create_test_workspace(vec![("app", "1.0.0"), ("lib", "0.5.0"), ("utils", "0.1.0")]);

        let selection = PackageSelection::workspace();
        workspace
            .roll_version(VersionBump::Patch(1), &selection)
            .unwrap();

        assert_eq!(workspace.members[0].version(), "1.0.1"); // app bumped
        assert_eq!(workspace.members[1].version(), "0.5.1"); // lib bumped
        assert_eq!(workspace.members[2].version(), "0.1.1"); // utils bumped
    }

    #[test]
    fn test_workspace_roll_version_workspace_with_exclude() {
        let mut workspace =
            create_test_workspace(vec![("app", "1.0.0"), ("lib", "0.5.0"), ("utils", "0.1.0")]);

        let selection = PackageSelection {
            packages: vec![],
            workspace: true,
            exclude: vec!["lib".to_string()],
        };

        workspace
            .roll_version(VersionBump::Patch(1), &selection)
            .unwrap();

        assert_eq!(workspace.members[0].version(), "1.0.1"); // app bumped
        assert_eq!(workspace.members[1].version(), "0.5.0"); // lib excluded
        assert_eq!(workspace.members[2].version(), "0.1.1"); // utils bumped
    }

    #[test]
    fn test_workspace_set_version() {
        let mut workspace = create_test_workspace(vec![("app", "1.0.0"), ("lib", "0.5.0")]);

        let selection = PackageSelection::packages(vec!["lib".to_string()]);
        workspace.set_version("2.0.0", &selection).unwrap();

        assert_eq!(workspace.members[0].version(), "1.0.0"); // app unchanged
        assert_eq!(workspace.members[1].version(), "2.0.0"); // lib set
    }

    #[test]
    fn test_workspace_sync_version() {
        let mut workspace =
            create_test_workspace(vec![("app", "1.0.0"), ("lib", "0.5.0"), ("utils", "2.1.0")]);

        workspace.sync_version("1.5.0").unwrap();

        assert_eq!(workspace.members[0].version(), "1.5.0");
        assert_eq!(workspace.members[1].version(), "1.5.0");
        assert_eq!(workspace.members[2].version(), "1.5.0");
    }

    #[test]
    fn test_workspace_show() {
        let workspace = create_test_workspace(vec![("app", "1.0.0"), ("lib", "0.5.0")]);

        let output = workspace.show();
        assert_eq!(output, "app 1.0.0\nlib 0.5.0\n");
    }

    #[test]
    fn test_workspace_lint_valid() {
        let workspace = create_test_workspace(vec![("app", "1.0.0"), ("lib", "0.5.0")]);

        let errors = workspace.lint();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_workspace_lint_invalid() {
        let workspace = create_test_workspace(vec![("app", "1.0.0"), ("lib", "invalid-version")]);

        let errors = workspace.lint();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].member, "lib");
        assert!(
            errors[0]
                .message
                .contains("Invalid version 'invalid-version'")
        );
    }

    #[test]
    fn test_package_selection_not_found() {
        let mut workspace = create_test_workspace(vec![("app", "1.0.0")]);

        let selection = PackageSelection::packages(vec!["nonexistent".to_string()]);
        let result = workspace.roll_version(VersionBump::Patch(1), &selection);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Package 'nonexistent' not found")
        );
    }
}
