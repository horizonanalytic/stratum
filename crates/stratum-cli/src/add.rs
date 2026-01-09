//! Implementation of the `stratum add` command.

use anyhow::{Context, Result};
use std::path::Path;
use stratum_pkg::registry::GitHubPackage;
use stratum_pkg::{Dependency, DependencySpec, Manifest, MANIFEST_FILE};

/// Which section to add the dependency to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencySection {
    /// Regular runtime dependencies.
    Dependencies,
    /// Development-only dependencies.
    Dev,
    /// Build-time dependencies.
    Build,
}

/// Options for adding a dependency.
#[derive(Debug)]
pub struct AddOptions {
    /// Package name (may include @version suffix or github: prefix).
    pub package: String,
    /// Which dependency section to add to.
    pub section: DependencySection,
    /// Path dependency (local filesystem).
    pub path: Option<String>,
    /// Git repository URL.
    pub git: Option<String>,
    /// GitHub shorthand (user/repo).
    pub github: Option<String>,
    /// Git branch name.
    pub branch: Option<String>,
    /// Git tag name.
    pub tag: Option<String>,
    /// Git revision (commit hash).
    pub rev: Option<String>,
    /// Features to enable.
    pub features: Vec<String>,
    /// Whether this is an optional dependency.
    pub optional: bool,
    /// Disable default features.
    pub no_default_features: bool,
}

/// Parse a package specification like "name" or "name@version".
fn parse_package_spec(spec: &str) -> (String, Option<String>) {
    if let Some(at_pos) = spec.rfind('@') {
        // Ensure the @ is not at the start (like @org/pkg in npm style)
        if at_pos > 0 {
            let name = &spec[..at_pos];
            let version = &spec[at_pos + 1..];
            // Only treat as version if the part after @ looks like a version
            if !version.is_empty() && (version.starts_with(|c: char| c.is_ascii_digit())
                || version.starts_with('^')
                || version.starts_with('~')
                || version.starts_with('=')
                || version.starts_with('>')
                || version.starts_with('<')
                || version.starts_with('*'))
            {
                return (name.to_string(), Some(version.to_string()));
            }
        }
    }
    (spec.to_string(), None)
}

/// Build a `DependencySpec` from the provided options.
fn build_dependency_spec(
    version: Option<String>,
    options: &AddOptions,
    git_url: Option<String>,
) -> DependencySpec {
    // If only a version is specified with no other options, use simple format
    if options.path.is_none()
        && options.git.is_none()
        && options.github.is_none()
        && git_url.is_none()
        && options.features.is_empty()
        && !options.optional
        && !options.no_default_features
    {
        if let Some(v) = version {
            return DependencySpec::Simple(v);
        }
    }

    // Use detailed format
    // Prefer explicit git URL, then github shorthand converted to URL
    let effective_git = git_url.or_else(|| options.git.clone());

    DependencySpec::Detailed(Dependency {
        version,
        path: options.path.clone(),
        git: effective_git,
        branch: options.branch.clone(),
        tag: options.tag.clone(),
        rev: options.rev.clone(),
        features: options.features.clone(),
        default_features: !options.no_default_features,
        optional: options.optional,
        package: None,
    })
}

/// Add a dependency to the manifest.
pub fn add_dependency(options: AddOptions) -> Result<()> {
    let manifest_path = Path::new(MANIFEST_FILE);

    // Check if manifest exists
    if !manifest_path.exists() {
        return Err(anyhow::anyhow!(
            "No {} found in current directory. Run `stratum init` first.",
            MANIFEST_FILE
        ));
    }

    // Load existing manifest
    let mut manifest = Manifest::from_path(manifest_path)
        .context("Failed to read manifest")?;

    // Check for github: prefix in package spec or --github flag
    let (name, version, git_url) = if options.package.starts_with("github:") {
        // Parse GitHub package spec like "github:user/repo@v1.0.0"
        let github_pkg = GitHubPackage::parse(&options.package)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let pkg_name = github_pkg.repo.clone();
        let git_url = github_pkg.git_url();
        // Use version as tag if specified
        (pkg_name, None, Some(git_url))
    } else if let Some(ref github_shorthand) = options.github {
        // Handle --github user/repo flag
        let spec = format!("github:{github_shorthand}");
        let github_pkg = GitHubPackage::parse(&spec)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let (pkg_name, version) = parse_package_spec(&options.package);
        let git_url = github_pkg.git_url();
        (pkg_name, version, Some(git_url))
    } else {
        // Standard package spec
        let (name, version) = parse_package_spec(&options.package);
        (name, version, None)
    };

    // Validate the package name
    validate_dependency_name(&name)?;

    // Build the dependency spec
    let dep_spec = build_dependency_spec(version, &options, git_url);

    // Get the appropriate dependency map
    let deps = match options.section {
        DependencySection::Dependencies => &mut manifest.dependencies,
        DependencySection::Dev => &mut manifest.dev_dependencies,
        DependencySection::Build => &mut manifest.build_dependencies,
    };

    // Check if dependency already exists
    let action = if deps.contains_key(&name) {
        "Updated"
    } else {
        "Added"
    };

    // Add or update the dependency
    deps.insert(name.clone(), dep_spec);

    // Serialize and write back
    let content = manifest
        .to_toml_string()
        .context("Failed to serialize manifest")?;
    std::fs::write(manifest_path, content)
        .context("Failed to write manifest")?;

    // Print success message
    let section_name = match options.section {
        DependencySection::Dependencies => "dependencies",
        DependencySection::Dev => "dev-dependencies",
        DependencySection::Build => "build-dependencies",
    };

    println!("{action} `{name}` to [{section_name}]");

    Ok(())
}

/// Validate a dependency name.
fn validate_dependency_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow::anyhow!("Dependency name cannot be empty"));
    }

    if name.len() > 64 {
        return Err(anyhow::anyhow!(
            "Dependency name cannot exceed 64 characters"
        ));
    }

    // Must start with a letter
    if !name.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return Err(anyhow::anyhow!(
            "Dependency name must start with a letter"
        ));
    }

    // Only alphanumeric, hyphens, and underscores
    for c in name.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
            return Err(anyhow::anyhow!(
                "Dependency name can only contain letters, numbers, hyphens, and underscores"
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_spec_name_only() {
        let (name, version) = parse_package_spec("http");
        assert_eq!(name, "http");
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_package_spec_with_version() {
        let (name, version) = parse_package_spec("http@1.0.0");
        assert_eq!(name, "http");
        assert_eq!(version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_parse_package_spec_with_caret_version() {
        let (name, version) = parse_package_spec("json@^2.1");
        assert_eq!(name, "json");
        assert_eq!(version, Some("^2.1".to_string()));
    }

    #[test]
    fn test_parse_package_spec_with_tilde_version() {
        let (name, version) = parse_package_spec("serde@~1.0");
        assert_eq!(name, "serde");
        assert_eq!(version, Some("~1.0".to_string()));
    }

    #[test]
    fn test_parse_package_spec_star_version() {
        let (name, version) = parse_package_spec("any@*");
        assert_eq!(name, "any");
        assert_eq!(version, Some("*".to_string()));
    }

    #[test]
    fn test_validate_dependency_name_valid() {
        assert!(validate_dependency_name("http").is_ok());
        assert!(validate_dependency_name("my-package").is_ok());
        assert!(validate_dependency_name("my_package").is_ok());
        assert!(validate_dependency_name("pkg123").is_ok());
    }

    #[test]
    fn test_validate_dependency_name_empty() {
        assert!(validate_dependency_name("").is_err());
    }

    #[test]
    fn test_validate_dependency_name_starts_with_number() {
        assert!(validate_dependency_name("123pkg").is_err());
    }

    #[test]
    fn test_validate_dependency_name_invalid_chars() {
        assert!(validate_dependency_name("my.package").is_err());
        assert!(validate_dependency_name("my/package").is_err());
    }

    #[test]
    fn test_build_simple_dependency_spec() {
        let options = AddOptions {
            package: "http".to_string(),
            section: DependencySection::Dependencies,
            path: None,
            git: None,
            github: None,
            branch: None,
            tag: None,
            rev: None,
            features: Vec::new(),
            optional: false,
            no_default_features: false,
        };

        let spec = build_dependency_spec(Some("1.0".to_string()), &options, None);
        assert!(matches!(spec, DependencySpec::Simple(v) if v == "1.0"));
    }

    #[test]
    fn test_build_detailed_dependency_spec_with_features() {
        let options = AddOptions {
            package: "json".to_string(),
            section: DependencySection::Dependencies,
            path: None,
            git: None,
            github: None,
            branch: None,
            tag: None,
            rev: None,
            features: vec!["pretty".to_string()],
            optional: false,
            no_default_features: false,
        };

        let spec = build_dependency_spec(Some("2.1".to_string()), &options, None);
        match spec {
            DependencySpec::Detailed(dep) => {
                assert_eq!(dep.version, Some("2.1".to_string()));
                assert_eq!(dep.features, vec!["pretty".to_string()]);
            }
            _ => panic!("Expected detailed spec"),
        }
    }

    #[test]
    fn test_build_path_dependency_spec() {
        let options = AddOptions {
            package: "local-lib".to_string(),
            section: DependencySection::Dependencies,
            path: Some("../local-lib".to_string()),
            git: None,
            github: None,
            branch: None,
            tag: None,
            rev: None,
            features: Vec::new(),
            optional: false,
            no_default_features: false,
        };

        let spec = build_dependency_spec(None, &options, None);
        match spec {
            DependencySpec::Detailed(dep) => {
                assert_eq!(dep.path, Some("../local-lib".to_string()));
                assert!(dep.version.is_none());
            }
            _ => panic!("Expected detailed spec"),
        }
    }

    #[test]
    fn test_build_git_dependency_spec() {
        let options = AddOptions {
            package: "remote-lib".to_string(),
            section: DependencySection::Dependencies,
            path: None,
            git: Some("https://github.com/example/lib".to_string()),
            github: None,
            branch: Some("main".to_string()),
            tag: None,
            rev: None,
            features: Vec::new(),
            optional: false,
            no_default_features: false,
        };

        let spec = build_dependency_spec(None, &options, None);
        match spec {
            DependencySpec::Detailed(dep) => {
                assert_eq!(dep.git, Some("https://github.com/example/lib".to_string()));
                assert_eq!(dep.branch, Some("main".to_string()));
            }
            _ => panic!("Expected detailed spec"),
        }
    }
}
