use anyhow::{Context, Result};
use serde_json::Value;
use std::{fs, path::Path};

use crate::domain::VersionField;

/// Parse a package.json file and return (name, version, has_workspace_inheritance)
pub fn parse(path: &Path) -> Result<(Option<String>, VersionField)> {
    let content = fs::read_to_string(path). //-
        with_context(|| format!("Failed to read {}", path.display()))?;

    let value: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;

    let name = value
        .get("name")
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    let version_raw = value.get("version").and_then(|v| v.as_str());

    // Check if version uses workspace protocol (workspace:*, workspace:~, etc.)
    let has_workspace_inheritance = version_raw
        .map(|v| v.starts_with("workspace:"))
        .unwrap_or(false);

    let version = if has_workspace_inheritance {
        VersionField::Inherited
    } else {
        match version_raw {
            Some(v) => VersionField::Concrete(v.to_string()),
            None => VersionField::Absent,
        }
    };

    Ok((name, version))
}

/// Update the version in a package.json file
pub fn update_version(path: &Path, new_version: &VersionField) -> Result<()> {
    let new_version = match new_version {
        VersionField::Concrete(version) => version,
        _ => return Ok(()),
    };

    let content = fs::read_to_string(path). //-
        with_context(|| format!("Failed to read {}", path.display()))?;

    let mut value: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;

    // Update the version field directly
    value["version"] = Value::String(new_version.to_string());

    let updated_content = serde_json::to_string_pretty(&value)
        .with_context(|| format!("Failed to serialize {}", path.display()))?;

    fs::write(path, updated_content)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp_json(contents: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", contents).unwrap();
        file
    }

    #[test]
    fn test_parse_basic_package() {
        let json = r#"{
            "name": "my-package",
            "version": "1.2.3"
        }"#;
        let file = write_temp_json(json);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Concrete("1.2.3".to_string()));
    }

    #[test]
    fn test_parse_workspace_inheritance() {
        let json = r#"{
            "name": "workspace-package",
            "version": "workspace:*"
        }"#;
        let file = write_temp_json(json);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("workspace-package".to_string()));
        assert_eq!(version, VersionField::Inherited);
    }

    #[test]
    fn test_parse_workspace_inheritance_tilde() {
        let json = r#"{
            "name": "workspace-package",
            "version": "workspace:~"
        }"#;
        let file = write_temp_json(json);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("workspace-package".to_string()));
        assert_eq!(version, VersionField::Inherited);
    }

    #[test]
    fn test_parse_missing_name() {
        let json = r#"{
            "version": "1.2.3"
        }"#;
        let file = write_temp_json(json);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, None);
        assert_eq!(version, VersionField::Concrete("1.2.3".to_string()));
    }

    #[test]
    fn test_parse_missing_version() {
        let json = r#"{
            "name": "my-package"
        }"#;
        let file = write_temp_json(json);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Absent);
    }

    #[test]
    fn test_parse_no_package_fields() {
        let json = r#"{
            "dependencies": {
                "lodash": "^4.17.0"
            }
        }"#;
        let file = write_temp_json(json);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, None);
        assert_eq!(version, VersionField::Absent);
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = r#"{
            "name": "invalid"
        "#;
        let file = write_temp_json(json);
        let result = parse(file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse"));
    }

    #[test]
    fn test_update_version_basic() {
        let json = r#"{
            "name": "my-package",
            "version": "1.2.3"
        }"#;
        let file = write_temp_json(json);
        let new_version = VersionField::Concrete("2.0.0".to_string());
        update_version(file.path(), &new_version).unwrap();
        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("\"version\": \"2.0.0\""));
    }

    #[test]
    fn test_update_version_adds_if_missing() {
        let json = r#"{
            "name": "my-package"
        }"#;
        let file = write_temp_json(json);
        let new_version = VersionField::Concrete("2.0.0".to_string());
        update_version(file.path(), &new_version).unwrap();
        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("\"version\": \"2.0.0\""));
    }

    #[test]
    fn test_update_version_workspace_inheritance() {
        let json = r#"{
            "name": "workspace-package",
            "version": "workspace:*"
        }"#;
        let file = write_temp_json(json);
        let new_version = VersionField::Concrete("2.0.0".to_string());
        update_version(file.path(), &new_version).unwrap();
        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("\"version\": \"2.0.0\""));
    }

    #[test]
    fn test_update_version_preserves_other_fields() {
        let json = r#"{
            "name": "my-package",
            "version": "1.2.3",
            "description": "A test package",
            "dependencies": {
                "lodash": "^4.17.0"
            }
        }"#;
        let file = write_temp_json(json);
        let new_version = VersionField::Concrete("2.0.0".to_string());
        update_version(file.path(), &new_version).unwrap();
        let content = fs::read_to_string(file.path()).unwrap();

        // Check that version was updated
        assert!(content.contains("\"version\": \"2.0.0\""));
        // Check that other fields are preserved
        assert!(content.contains("\"description\": \"A test package\""));
        assert!(content.contains("\"lodash\": \"^4.17.0\""));
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let result = parse(Path::new("/nonexistent/path/package.json"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read"));
    }

    #[test]
    fn test_parse_version_with_whitespace() {
        let json = r#"{
            "name": "my-package",
            "version": " 1.2.3 "
        }"#;
        let file = write_temp_json(json);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Concrete(" 1.2.3 ".to_string()));
    }

    #[test]
    fn test_parse_version_invalid_semver() {
        let json = r#"{
            "name": "my-package",
            "version": "not-a-version"
        }"#;
        let file = write_temp_json(json);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Concrete("not-a-version".to_string()));
    }

    #[test]
    fn test_parse_workspace_inheritance_with_version() {
        let json = r#"{
            "name": "workspace-package",
            "version": "workspace:1.2.3"
        }"#;
        let file = write_temp_json(json);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("workspace-package".to_string()));
        assert_eq!(version, VersionField::Inherited);
    }

    #[test]
    fn test_parse_workspace_inheritance_with_range() {
        let json = r#"{
            "name": "workspace-package",
            "version": "workspace:^1.2.3"
        }"#;
        let file = write_temp_json(json);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("workspace-package".to_string()));
        assert_eq!(version, VersionField::Inherited);
    }

    #[test]
    fn test_parse_version_null() {
        let json = r#"{
            "name": "my-package",
            "version": null
        }"#;
        let file = write_temp_json(json);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Absent);
    }

    #[test]
    fn test_parse_version_number() {
        let json = r#"{
            "name": "my-package",
            "version": 1.2
        }"#;
        let file = write_temp_json(json);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Absent);
    }

    #[test]
    fn test_parse_version_empty_string() {
        let json = r#"{
            "name": "my-package",
            "version": ""
        }"#;
        let file = write_temp_json(json);
        let (name, version) = parse(file.path()).unwrap();
        assert_eq!(name, Some("my-package".to_string()));
        assert_eq!(version, VersionField::Concrete("".to_string()));
    }
}
