//! Project initialization for `stratum init`.

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::{env, fs};
use stratum_pkg::{Edition, Manifest, Package};

/// Options for project initialization.
#[derive(Debug, Clone)]
pub struct InitOptions {
    /// Create a library project instead of a binary.
    pub lib: bool,

    /// Package name (defaults to directory name).
    pub name: Option<String>,

    /// Initialize a git repository.
    pub git: bool,
}

impl Default for InitOptions {
    fn default() -> Self {
        Self {
            lib: false,
            name: None,
            git: false,
        }
    }
}

/// Initialize a new Stratum project in the current directory.
pub fn init_project(options: InitOptions) -> Result<()> {
    let current_dir = env::current_dir().context("Failed to get current directory")?;

    // Check if already initialized
    let manifest_path = current_dir.join(stratum_pkg::MANIFEST_FILE);
    if manifest_path.exists() {
        bail!(
            "Cannot initialize: `{}` already exists in this directory",
            stratum_pkg::MANIFEST_FILE
        );
    }

    // Determine package name
    let name = match options.name {
        Some(n) => n,
        None => infer_package_name(&current_dir)?,
    };

    // Validate the name
    validate_package_name(&name)?;

    // Create the manifest
    let manifest = create_manifest(&name);

    // Create directory structure
    create_directory_structure(&current_dir, options.lib)?;

    // Write the manifest
    let manifest_content = manifest
        .to_toml_string()
        .context("Failed to serialize manifest")?;
    fs::write(&manifest_path, manifest_content).context("Failed to write stratum.toml")?;

    // Write the template source file
    write_template_source(&current_dir, &name, options.lib)?;

    // Initialize git if requested
    if options.git {
        init_git(&current_dir)?;
    }

    // Print success message
    let project_type = if options.lib { "library" } else { "binary" };
    println!("Created {} `{}` package", project_type, name);

    Ok(())
}

/// Infer the package name from the current directory.
fn infer_package_name(dir: &Path) -> Result<String> {
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(ToString::to_string)
        .context("Cannot infer package name from directory")?;

    Ok(name)
}

/// Validate a package name according to Stratum rules.
fn validate_package_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Package name cannot be empty");
    }

    if name.len() > 64 {
        bail!("Package name cannot exceed 64 characters");
    }

    // Must start with a letter
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() {
        bail!("Package name must start with a letter");
    }

    // Only alphanumeric, hyphens, and underscores
    for c in name.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
            bail!(
                "Package name can only contain letters, numbers, hyphens, and underscores"
            );
        }
    }

    Ok(())
}

/// Create a default manifest with the given package name.
fn create_manifest(name: &str) -> Manifest {
    Manifest {
        package: Package {
            name: name.to_string(),
            version: String::from("0.1.0"),
            edition: Edition::Edition2025,
            authors: Vec::new(),
            description: None,
            license: None,
            license_file: None,
            repository: None,
            homepage: None,
            documentation: None,
            readme: None,
            keywords: Vec::new(),
            categories: Vec::new(),
            exclude: Vec::new(),
            include: Vec::new(),
            default_run: None,
        },
        ..Default::default()
    }
}

/// Create the directory structure for a new project.
fn create_directory_structure(root: &Path, is_lib: bool) -> Result<()> {
    // Create src directory
    let src_dir = root.join(stratum_pkg::SOURCE_DIR);
    fs::create_dir_all(&src_dir).context("Failed to create src directory")?;

    // Create additional directories for library projects
    if is_lib {
        let examples_dir = root.join(stratum_pkg::EXAMPLES_DIR);
        fs::create_dir_all(&examples_dir).context("Failed to create examples directory")?;
    }

    Ok(())
}

/// Write the template source file.
fn write_template_source(root: &Path, name: &str, is_lib: bool) -> Result<()> {
    let src_dir = root.join(stratum_pkg::SOURCE_DIR);

    if is_lib {
        // Library template
        let lib_path = src_dir.join(stratum_pkg::LIB_FILE);
        let content = format!(
            r#"/// {} library module.

/// Add two numbers together.
fx add(a: Int, b: Int) -> Int {{
    a + b
}}

#[test]
fx test_add() {{
    assert(add(2, 2) == 4)
}}
"#,
            name
        );
        fs::write(&lib_path, content).context("Failed to write lib.strat")?;
    } else {
        // Binary template
        let main_path = src_dir.join(stratum_pkg::MAIN_FILE);
        let content = r#"/// Entry point for the application.
fx main() {
    println("Hello, Stratum!")
}
"#;
        fs::write(&main_path, content).context("Failed to write main.strat")?;
    }

    Ok(())
}

/// Initialize a git repository in the given directory.
fn init_git(root: &Path) -> Result<()> {
    use std::process::Command;

    // Check if git is available
    let git_check = Command::new("git").arg("--version").output();
    if git_check.is_err() {
        eprintln!("Warning: git not found, skipping repository initialization");
        return Ok(());
    }

    // Check if already in a git repository
    let status = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(root)
        .output();

    if status.is_ok() && status.unwrap().status.success() {
        // Already in a git repo, just create .gitignore
        write_gitignore(root)?;
        return Ok(());
    }

    // Initialize new git repository
    let init_result = Command::new("git")
        .args(["init"])
        .current_dir(root)
        .output()
        .context("Failed to run git init")?;

    if !init_result.status.success() {
        let stderr = String::from_utf8_lossy(&init_result.stderr);
        bail!("git init failed: {}", stderr);
    }

    // Write .gitignore
    write_gitignore(root)?;

    Ok(())
}

/// Write a .gitignore file for Stratum projects.
fn write_gitignore(root: &Path) -> Result<()> {
    let gitignore_path = root.join(".gitignore");

    // Don't overwrite existing .gitignore
    if gitignore_path.exists() {
        return Ok(());
    }

    let content = r#"# Stratum build artifacts
/target/
*.stratum

# IDE files
.idea/
.vscode/
*.swp
*.swo
*~

# OS files
.DS_Store
Thumbs.db
"#;

    fs::write(&gitignore_path, content).context("Failed to write .gitignore")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_package_name_valid() {
        assert!(validate_package_name("my-app").is_ok());
        assert!(validate_package_name("my_app").is_ok());
        assert!(validate_package_name("app123").is_ok());
        assert!(validate_package_name("MyApp").is_ok());
    }

    #[test]
    fn test_validate_package_name_invalid() {
        assert!(validate_package_name("").is_err());
        assert!(validate_package_name("123app").is_err());
        assert!(validate_package_name("-app").is_err());
        assert!(validate_package_name("my app").is_err());
        assert!(validate_package_name("my.app").is_err());
    }

    #[test]
    fn test_infer_package_name() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("my-project");
        fs::create_dir(&path).unwrap();

        let name = infer_package_name(&path).unwrap();
        assert_eq!(name, "my-project");
    }
}
