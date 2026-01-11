//! Implementation of the `stratum remove` command.

use anyhow::{Context, Result};
use std::path::Path;
use stratum_pkg::{Manifest, MANIFEST_FILE};

use crate::add::DependencySection;

/// Options for removing a dependency.
#[derive(Debug)]
pub struct RemoveOptions {
    /// Package name to remove.
    pub package: String,
    /// Which dependency section to remove from (None = search all sections).
    pub section: Option<DependencySection>,
}

/// Remove a dependency from the manifest in the current directory.
pub fn remove_dependency(options: RemoveOptions) -> Result<()> {
    remove_dependency_at(Path::new(MANIFEST_FILE), options)
}

/// Remove a dependency from a manifest at a specific path.
pub fn remove_dependency_at(manifest_path: &Path, options: RemoveOptions) -> Result<()> {
    // Check if manifest exists
    if !manifest_path.exists() {
        return Err(anyhow::anyhow!(
            "No {} found. Run `stratum init` first.",
            manifest_path.display()
        ));
    }

    // Load existing manifest
    let mut manifest = Manifest::from_path(manifest_path).context("Failed to read manifest")?;

    let name = &options.package;

    // Determine which section(s) to search
    match options.section {
        Some(section) => {
            // Remove from specific section
            let (deps, section_name) = match section {
                DependencySection::Dependencies => (&mut manifest.dependencies, "dependencies"),
                DependencySection::Dev => (&mut manifest.dev_dependencies, "dev-dependencies"),
                DependencySection::Build => {
                    (&mut manifest.build_dependencies, "build-dependencies")
                }
            };

            if deps.remove(name).is_none() {
                return Err(anyhow::anyhow!(
                    "Dependency `{name}` not found in [{section_name}]"
                ));
            }

            // Serialize and write back
            write_manifest(&manifest, manifest_path)?;

            println!("Removed `{name}` from [{section_name}]");
        }
        None => {
            // Search all sections
            let in_deps = manifest.dependencies.contains_key(name);
            let in_dev = manifest.dev_dependencies.contains_key(name);
            let in_build = manifest.build_dependencies.contains_key(name);

            let count = [in_deps, in_dev, in_build].iter().filter(|&&x| x).count();

            if count == 0 {
                return Err(anyhow::anyhow!(
                    "Dependency `{name}` not found in any section"
                ));
            }

            if count > 1 {
                // Found in multiple sections - require user to specify
                let mut sections = Vec::new();
                if in_deps {
                    sections.push("[dependencies]");
                }
                if in_dev {
                    sections.push("[dev-dependencies]");
                }
                if in_build {
                    sections.push("[build-dependencies]");
                }
                return Err(anyhow::anyhow!(
                    "Dependency `{name}` found in multiple sections: {}. Use --dev or --build to specify which to remove.",
                    sections.join(", ")
                ));
            }

            // Remove from the one section it's in
            let section_name = if in_deps {
                manifest.dependencies.remove(name);
                "dependencies"
            } else if in_dev {
                manifest.dev_dependencies.remove(name);
                "dev-dependencies"
            } else {
                manifest.build_dependencies.remove(name);
                "build-dependencies"
            };

            // Serialize and write back
            write_manifest(&manifest, manifest_path)?;

            println!("Removed `{name}` from [{section_name}]");
        }
    }

    Ok(())
}

/// Write manifest back to file.
fn write_manifest(manifest: &Manifest, path: &Path) -> Result<()> {
    let content = manifest
        .to_toml_string()
        .context("Failed to serialize manifest")?;
    std::fs::write(path, content).context("Failed to write manifest")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_manifest(dir: &TempDir, content: &str) -> std::path::PathBuf {
        let manifest_path = dir.path().join(MANIFEST_FILE);
        fs::write(&manifest_path, content).unwrap();
        manifest_path
    }

    #[test]
    fn test_remove_from_dependencies() {
        let dir = TempDir::new().unwrap();
        let manifest_path = setup_test_manifest(
            &dir,
            r#"
[package]
name = "test"
version = "0.1.0"
edition = "2025"

[dependencies]
myhttp = "1.0"
json = "2.0"
"#,
        );

        let options = RemoveOptions {
            package: "myhttp".to_string(),
            section: None,
        };
        let result = remove_dependency_at(&manifest_path, options);

        assert!(result.is_ok());

        // Verify the manifest was updated
        let content = fs::read_to_string(&manifest_path).unwrap();
        assert!(!content.contains("myhttp"));
        assert!(content.contains("json"));
    }

    #[test]
    fn test_remove_from_dev_dependencies() {
        let dir = TempDir::new().unwrap();
        let manifest_path = setup_test_manifest(
            &dir,
            r#"
[package]
name = "test"
version = "0.1.0"
edition = "2025"

[dev-dependencies]
test-utils = "0.5"
"#,
        );

        let options = RemoveOptions {
            package: "test-utils".to_string(),
            section: Some(DependencySection::Dev),
        };
        let result = remove_dependency_at(&manifest_path, options);

        assert!(result.is_ok());

        let content = fs::read_to_string(&manifest_path).unwrap();
        assert!(!content.contains("test-utils"));
    }

    #[test]
    fn test_remove_nonexistent_package() {
        let dir = TempDir::new().unwrap();
        let manifest_path = setup_test_manifest(
            &dir,
            r#"
[package]
name = "test"
version = "0.1.0"
edition = "2025"

[dependencies]
myhttp = "1.0"
"#,
        );

        let options = RemoveOptions {
            package: "nonexistent".to_string(),
            section: None,
        };
        let result = remove_dependency_at(&manifest_path, options);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not found in any section"));
    }

    #[test]
    fn test_remove_wrong_section() {
        let dir = TempDir::new().unwrap();
        let manifest_path = setup_test_manifest(
            &dir,
            r#"
[package]
name = "test"
version = "0.1.0"
edition = "2025"

[dependencies]
myhttp = "1.0"
"#,
        );

        let options = RemoveOptions {
            package: "myhttp".to_string(),
            section: Some(DependencySection::Dev),
        };
        let result = remove_dependency_at(&manifest_path, options);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not found in [dev-dependencies]"));
    }

    #[test]
    fn test_remove_in_multiple_sections_requires_flag() {
        let dir = TempDir::new().unwrap();
        let manifest_path = setup_test_manifest(
            &dir,
            r#"
[package]
name = "test"
version = "0.1.0"
edition = "2025"

[dependencies]
shared = "1.0"

[dev-dependencies]
shared = "1.0"
"#,
        );

        let options = RemoveOptions {
            package: "shared".to_string(),
            section: None,
        };
        let result = remove_dependency_at(&manifest_path, options);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("found in multiple sections"));
        assert!(err.contains("--dev or --build"));
    }
}
