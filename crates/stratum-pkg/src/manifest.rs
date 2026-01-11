//! Stratum package manifest (`stratum.toml`) parsing and validation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur when working with manifests.
#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("failed to read manifest file: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse manifest: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("missing required field: {0}")]
    MissingField(&'static str),

    #[error("invalid package name '{0}': {1}")]
    InvalidName(String, &'static str),

    #[error("invalid version '{0}': {1}")]
    InvalidVersion(String, String),

    #[error("unknown edition '{0}', expected one of: 2025")]
    UnknownEdition(String),
}

/// The complete stratum.toml manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Package metadata (required).
    pub package: Package,

    /// Runtime dependencies.
    #[serde(default)]
    pub dependencies: BTreeMap<String, DependencySpec>,

    /// Development-only dependencies.
    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: BTreeMap<String, DependencySpec>,

    /// Build-time dependencies.
    #[serde(default, rename = "build-dependencies")]
    pub build_dependencies: BTreeMap<String, DependencySpec>,

    /// Binary targets.
    #[serde(default, rename = "bin")]
    pub binaries: Vec<Target>,

    /// Library target configuration.
    #[serde(default)]
    pub lib: Option<Target>,

    /// Test targets.
    #[serde(default, rename = "test")]
    pub tests: Vec<Target>,

    /// Example targets.
    #[serde(default, rename = "example")]
    pub examples: Vec<Target>,

    /// Benchmark targets.
    #[serde(default, rename = "bench")]
    pub benches: Vec<Target>,
}

/// Package metadata section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    /// Package name (required).
    pub name: String,

    /// Package version (required, semver).
    pub version: String,

    /// Stratum language edition (required).
    pub edition: Edition,

    /// Package authors.
    #[serde(default)]
    pub authors: Vec<String>,

    /// Short description.
    #[serde(default)]
    pub description: Option<String>,

    /// SPDX license identifier.
    #[serde(default)]
    pub license: Option<String>,

    /// Path to license file (alternative to license field).
    #[serde(default, rename = "license-file")]
    pub license_file: Option<String>,

    /// Repository URL.
    #[serde(default)]
    pub repository: Option<String>,

    /// Homepage URL.
    #[serde(default)]
    pub homepage: Option<String>,

    /// Documentation URL.
    #[serde(default)]
    pub documentation: Option<String>,

    /// Path to README file.
    #[serde(default)]
    pub readme: Option<String>,

    /// Keywords for package discovery.
    #[serde(default)]
    pub keywords: Vec<String>,

    /// Category slugs for registry categorization.
    #[serde(default)]
    pub categories: Vec<String>,

    /// Files to exclude from publishing.
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Files to include when publishing.
    #[serde(default)]
    pub include: Vec<String>,

    /// Default execution mode for the package.
    #[serde(default, rename = "default-run")]
    pub default_run: Option<String>,
}

/// Stratum language edition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Edition {
    /// The 2025 edition (initial release).
    #[default]
    #[serde(rename = "2025")]
    Edition2025,
}

impl Edition {
    /// Returns the edition as a string.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Edition2025 => "2025",
        }
    }
}

impl std::fmt::Display for Edition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Edition {
    type Err = ManifestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "2025" => Ok(Self::Edition2025),
            _ => Err(ManifestError::UnknownEdition(s.to_string())),
        }
    }
}

/// Dependency specification.
///
/// Can be either a simple version string or a detailed specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    /// Simple version string: `"1.0"` or `"^2.1.0"`.
    Simple(String),

    /// Detailed dependency specification.
    Detailed(Dependency),
}

impl DependencySpec {
    /// Returns the version requirement string if specified.
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        match self {
            Self::Simple(v) => Some(v),
            Self::Detailed(d) => d.version.as_deref(),
        }
    }

    /// Returns true if this is a path dependency.
    #[must_use]
    pub fn is_path(&self) -> bool {
        matches!(self, Self::Detailed(d) if d.path.is_some())
    }

    /// Returns true if this is a git dependency.
    #[must_use]
    pub fn is_git(&self) -> bool {
        matches!(self, Self::Detailed(d) if d.git.is_some())
    }
}

/// Detailed dependency specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    /// Version requirement.
    #[serde(default)]
    pub version: Option<String>,

    /// Path to local dependency.
    #[serde(default)]
    pub path: Option<String>,

    /// Git repository URL.
    #[serde(default)]
    pub git: Option<String>,

    /// Git branch name.
    #[serde(default)]
    pub branch: Option<String>,

    /// Git tag name.
    #[serde(default)]
    pub tag: Option<String>,

    /// Git commit revision.
    #[serde(default)]
    pub rev: Option<String>,

    /// Features to enable.
    #[serde(default)]
    pub features: Vec<String>,

    /// Whether default features are enabled.
    #[serde(default = "default_true", rename = "default-features")]
    pub default_features: bool,

    /// Whether this dependency is optional.
    #[serde(default)]
    pub optional: bool,

    /// Package name if different from the dependency key.
    #[serde(default)]
    pub package: Option<String>,
}

fn default_true() -> bool {
    true
}

/// A build target (binary, library, test, example, benchmark).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// Target name.
    pub name: String,

    /// Path to the target's entry point file.
    #[serde(default)]
    pub path: Option<String>,

    /// Whether this target is built by default.
    #[serde(default = "default_true")]
    pub build: bool,

    /// Documentation to generate for this target.
    #[serde(default = "default_true")]
    pub doc: bool,

    /// Target-specific dependencies.
    #[serde(default)]
    pub dependencies: BTreeMap<String, DependencySpec>,
}

/// The kind of target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// Library target.
    Lib,
    /// Binary target.
    Bin,
    /// Test target.
    Test,
    /// Example target.
    Example,
    /// Benchmark target.
    Bench,
}

impl Manifest {
    /// Load a manifest from a file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse a manifest from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML is invalid or missing required fields.
    pub fn parse(content: &str) -> Result<Self, ManifestError> {
        let manifest: Self = toml::from_str(content)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate the manifest.
    fn validate(&self) -> Result<(), ManifestError> {
        self.validate_name()?;
        self.validate_version()?;
        Ok(())
    }

    /// Validate the package name.
    fn validate_name(&self) -> Result<(), ManifestError> {
        let name = &self.package.name;

        if name.is_empty() {
            return Err(ManifestError::InvalidName(
                name.clone(),
                "name cannot be empty",
            ));
        }

        if name.len() > 64 {
            return Err(ManifestError::InvalidName(
                name.clone(),
                "name cannot exceed 64 characters",
            ));
        }

        // Must start with a letter
        if !name.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
            return Err(ManifestError::InvalidName(
                name.clone(),
                "name must start with a letter",
            ));
        }

        // Only alphanumeric, hyphens, and underscores
        for c in name.chars() {
            if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
                return Err(ManifestError::InvalidName(
                    name.clone(),
                    "name can only contain letters, numbers, hyphens, and underscores",
                ));
            }
        }

        Ok(())
    }

    /// Validate the version string.
    fn validate_version(&self) -> Result<(), ManifestError> {
        let version = &self.package.version;

        semver::Version::parse(version)
            .map_err(|e| ManifestError::InvalidVersion(version.clone(), e.to_string()))?;

        Ok(())
    }

    /// Serialize the manifest to a TOML string.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Get all dependencies including dev and build dependencies.
    pub fn all_dependencies(&self) -> impl Iterator<Item = (&String, &DependencySpec)> {
        self.dependencies
            .iter()
            .chain(self.dev_dependencies.iter())
            .chain(self.build_dependencies.iter())
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            package: Package {
                name: String::from("my-package"),
                version: String::from("0.1.0"),
                edition: Edition::default(),
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
            dependencies: BTreeMap::new(),
            dev_dependencies: BTreeMap::new(),
            build_dependencies: BTreeMap::new(),
            binaries: Vec::new(),
            lib: None,
            tests: Vec::new(),
            examples: Vec::new(),
            benches: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_manifest() {
        let toml = r#"
[package]
name = "test-pkg"
version = "0.1.0"
edition = "2025"
"#;
        let manifest = Manifest::parse(toml).unwrap();
        assert_eq!(manifest.package.name, "test-pkg");
        assert_eq!(manifest.package.version, "0.1.0");
        assert_eq!(manifest.package.edition, Edition::Edition2025);
    }

    #[test]
    fn parse_full_manifest() {
        let toml = r#"
[package]
name = "my-app"
version = "1.2.3"
edition = "2025"
authors = ["Alice <alice@example.com>", "Bob <bob@example.com>"]
description = "A sample Stratum application"
license = "MIT"
repository = "https://github.com/example/my-app"
keywords = ["cli", "tool"]

[dependencies]
http = "1.0"
json = { version = "2.1", features = ["pretty"] }

[dev-dependencies]
test-utils = "0.5"
"#;
        let manifest = Manifest::parse(toml).unwrap();
        assert_eq!(manifest.package.name, "my-app");
        assert_eq!(manifest.package.authors.len(), 2);
        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(manifest.dev_dependencies.len(), 1);
    }

    #[test]
    fn parse_path_dependency() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"
edition = "2025"

[dependencies]
local-lib = { path = "../local-lib" }
"#;
        let manifest = Manifest::parse(toml).unwrap();
        let dep = &manifest.dependencies["local-lib"];
        assert!(dep.is_path());
    }

    #[test]
    fn parse_git_dependency() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"
edition = "2025"

[dependencies]
remote-lib = { git = "https://github.com/example/lib", branch = "main" }
"#;
        let manifest = Manifest::parse(toml).unwrap();
        let dep = &manifest.dependencies["remote-lib"];
        assert!(dep.is_git());
    }

    #[test]
    fn invalid_name_empty() {
        let toml = r#"
[package]
name = ""
version = "0.1.0"
edition = "2025"
"#;
        let err = Manifest::parse(toml).unwrap_err();
        assert!(matches!(err, ManifestError::InvalidName(..)));
    }

    #[test]
    fn invalid_name_starts_with_number() {
        let toml = r#"
[package]
name = "123pkg"
version = "0.1.0"
edition = "2025"
"#;
        let err = Manifest::parse(toml).unwrap_err();
        assert!(matches!(err, ManifestError::InvalidName(..)));
    }

    #[test]
    fn invalid_version() {
        let toml = r#"
[package]
name = "test"
version = "not-a-version"
edition = "2025"
"#;
        let err = Manifest::parse(toml).unwrap_err();
        assert!(matches!(err, ManifestError::InvalidVersion(..)));
    }

    #[test]
    fn unknown_edition() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"
edition = "2099"
"#;
        let err = Manifest::parse(toml).unwrap_err();
        // This will be a parse error since "2099" doesn't match the enum
        assert!(matches!(err, ManifestError::Parse(..)));
    }
}
