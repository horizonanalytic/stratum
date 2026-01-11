//! Workspace support for multi-package projects.
//!
//! A workspace allows multiple Stratum packages to be developed together
//! while sharing dependencies and configuration.
//!
//! ```toml
//! # stratum.toml at workspace root
//! [workspace]
//! members = ["crates/*", "tools/cli"]
//!
//! [workspace.package]
//! version = "0.1.0"
//! edition = "2025"
//! authors = ["Team <team@example.com>"]
//!
//! [workspace.dependencies]
//! http = "1.0"
//! json = "2.1"
//! ```

use crate::manifest::{DependencySpec, Edition, ManifestError};
use crate::package::{PackageError, PackageStructure, MANIFEST_FILE};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur when working with workspaces.
#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),

    #[error("package error: {0}")]
    Package(#[from] PackageError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("glob pattern error: {0}")]
    Glob(#[from] glob::PatternError),

    #[error("not a workspace: missing [workspace] section")]
    NotAWorkspace,

    #[error("member not found: {0}")]
    MemberNotFound(String),

    #[error("circular dependency detected in workspace")]
    CircularDependency,
}

/// A workspace manifest file that may contain workspace configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    /// Workspace configuration (if this is a workspace root).
    #[serde(default)]
    pub workspace: Option<WorkspaceConfig>,

    /// Package configuration (if this is also a package).
    #[serde(default)]
    pub package: Option<PackageInWorkspace>,
}

/// Workspace configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceConfig {
    /// Member packages specified as glob patterns.
    #[serde(default)]
    pub members: Vec<String>,

    /// Packages to exclude from the workspace.
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Shared package metadata that members can inherit.
    #[serde(default)]
    pub package: Option<WorkspacePackageConfig>,

    /// Shared dependencies that members can inherit.
    #[serde(default)]
    pub dependencies: BTreeMap<String, DependencySpec>,
}

/// Shared package configuration for workspace members.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspacePackageConfig {
    /// Shared version.
    #[serde(default)]
    pub version: Option<String>,

    /// Shared edition.
    #[serde(default)]
    pub edition: Option<Edition>,

    /// Shared authors.
    #[serde(default)]
    pub authors: Option<Vec<String>>,

    /// Shared license.
    #[serde(default)]
    pub license: Option<String>,

    /// Shared repository.
    #[serde(default)]
    pub repository: Option<String>,

    /// Shared homepage.
    #[serde(default)]
    pub homepage: Option<String>,

    /// Shared documentation URL.
    #[serde(default)]
    pub documentation: Option<String>,
}

/// Package section that supports workspace inheritance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInWorkspace {
    /// Package name (required, never inherited).
    pub name: String,

    /// Version (can inherit from workspace).
    #[serde(default)]
    pub version: VersionOrWorkspace,

    /// Edition (can inherit from workspace).
    #[serde(default)]
    pub edition: EditionOrWorkspace,

    /// Authors (can inherit from workspace).
    #[serde(default)]
    pub authors: AuthorsOrWorkspace,

    /// Other fields...
    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub license: LicenseOrWorkspace,

    #[serde(default)]
    pub repository: RepositoryOrWorkspace,
}

/// Version that can be explicit or inherited from workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VersionOrWorkspace {
    /// Explicit version string.
    Explicit(String),
    /// Inherit from workspace.
    Workspace(WorkspaceInherit),
}

impl Default for VersionOrWorkspace {
    fn default() -> Self {
        Self::Workspace(WorkspaceInherit::default())
    }
}

/// Edition that can be explicit or inherited.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EditionOrWorkspace {
    /// Explicit edition.
    Explicit(Edition),
    /// Inherit from workspace.
    Workspace(WorkspaceInherit),
}

impl Default for EditionOrWorkspace {
    fn default() -> Self {
        Self::Workspace(WorkspaceInherit::default())
    }
}

/// Authors that can be explicit or inherited.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AuthorsOrWorkspace {
    /// Explicit authors list.
    Explicit(Vec<String>),
    /// Inherit from workspace.
    Workspace(WorkspaceInherit),
}

impl Default for AuthorsOrWorkspace {
    fn default() -> Self {
        Self::Workspace(WorkspaceInherit::default())
    }
}

/// License that can be explicit or inherited.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LicenseOrWorkspace {
    /// Explicit license.
    Explicit(String),
    /// Inherit from workspace.
    Workspace(WorkspaceInherit),
}

impl Default for LicenseOrWorkspace {
    fn default() -> Self {
        Self::Workspace(WorkspaceInherit::default())
    }
}

/// Repository that can be explicit or inherited.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RepositoryOrWorkspace {
    /// Explicit repository URL.
    Explicit(String),
    /// Inherit from workspace.
    Workspace(WorkspaceInherit),
}

impl Default for RepositoryOrWorkspace {
    fn default() -> Self {
        Self::Workspace(WorkspaceInherit::default())
    }
}

/// Marker for inheriting from workspace.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceInherit {
    /// Must be true to inherit.
    pub workspace: bool,
}

impl WorkspaceInherit {
    /// Create a new workspace inherit marker.
    #[must_use]
    pub fn new() -> Self {
        Self { workspace: true }
    }
}

/// A discovered workspace member.
#[derive(Debug, Clone)]
pub struct WorkspaceMember {
    /// Path to the member package root.
    pub path: PathBuf,

    /// Package name.
    pub name: String,

    /// Loaded package structure.
    pub package: PackageStructure,
}

/// A complete workspace with all discovered members.
#[derive(Debug)]
pub struct Workspace {
    /// Root directory of the workspace.
    pub root: PathBuf,

    /// Path to the workspace manifest.
    pub manifest_path: PathBuf,

    /// The workspace configuration.
    pub config: WorkspaceConfig,

    /// Discovered member packages.
    pub members: Vec<WorkspaceMember>,
}

impl Workspace {
    /// Load a workspace from a directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory doesn't contain a valid workspace.
    pub fn load(root: impl AsRef<Path>) -> Result<Self, WorkspaceError> {
        let root = root.as_ref().to_path_buf();
        let manifest_path = root.join(MANIFEST_FILE);

        let content = std::fs::read_to_string(&manifest_path).map_err(WorkspaceError::Io)?;

        let workspace_manifest: WorkspaceManifest =
            toml::from_str(&content).map_err(ManifestError::Parse)?;

        let config = workspace_manifest
            .workspace
            .ok_or(WorkspaceError::NotAWorkspace)?;

        let members = Self::discover_members(&root, &config)?;

        Ok(Self {
            root,
            manifest_path,
            config,
            members,
        })
    }

    /// Find a workspace by searching upward from a directory.
    ///
    /// # Errors
    ///
    /// Returns an error if no workspace is found.
    pub fn find(start: impl AsRef<Path>) -> Result<Self, WorkspaceError> {
        let mut current = start.as_ref().to_path_buf();

        loop {
            let manifest = current.join(MANIFEST_FILE);
            if manifest.exists() {
                // Check if this is a workspace
                if let Ok(content) = std::fs::read_to_string(&manifest) {
                    if let Ok(wm) = toml::from_str::<WorkspaceManifest>(&content) {
                        if wm.workspace.is_some() {
                            return Self::load(&current);
                        }
                    }
                }
            }

            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => return Err(WorkspaceError::NotAWorkspace),
            }
        }
    }

    /// Discover all member packages.
    fn discover_members(
        root: &Path,
        config: &WorkspaceConfig,
    ) -> Result<Vec<WorkspaceMember>, WorkspaceError> {
        let mut members = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        for pattern in &config.members {
            let full_pattern = root.join(pattern);
            let pattern_str = full_pattern.to_string_lossy();

            for entry in glob::glob(&pattern_str)? {
                let path = entry.map_err(|e| WorkspaceError::Io(e.into_error()))?;

                // Skip if already seen
                if !seen_paths.insert(path.clone()) {
                    continue;
                }

                // Skip if explicitly excluded
                if Self::is_excluded(&path, root, &config.exclude) {
                    continue;
                }

                // Check if this is a package directory
                let manifest_path = path.join(MANIFEST_FILE);
                if !manifest_path.exists() {
                    continue;
                }

                // Load the package
                match PackageStructure::load(&path) {
                    Ok(package) => {
                        members.push(WorkspaceMember {
                            path,
                            name: package.manifest.package.name.clone(),
                            package,
                        });
                    }
                    Err(PackageError::NoTargets) => {
                        // Skip packages with no targets (might be workspace-only manifests)
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }

        Ok(members)
    }

    /// Check if a path is excluded.
    fn is_excluded(path: &Path, root: &Path, excludes: &[String]) -> bool {
        let relative = path.strip_prefix(root).unwrap_or(path);
        let relative_str = relative.to_string_lossy();

        for exclude in excludes {
            if glob::Pattern::new(exclude)
                .map(|p| p.matches(&relative_str))
                .unwrap_or(false)
            {
                return true;
            }
        }

        false
    }

    /// Get a member by name.
    #[must_use]
    pub fn member(&self, name: &str) -> Option<&WorkspaceMember> {
        self.members.iter().find(|m| m.name == name)
    }

    /// Get the names of all members.
    #[must_use]
    pub fn member_names(&self) -> Vec<&str> {
        self.members.iter().map(|m| m.name.as_str()).collect()
    }

    /// Check if a path is within this workspace.
    #[must_use]
    pub fn contains(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }

    /// Resolve a workspace dependency to a concrete version.
    #[must_use]
    pub fn resolve_dependency(&self, name: &str) -> Option<&DependencySpec> {
        self.config.dependencies.get(name)
    }
}

impl WorkspaceManifest {
    /// Load a workspace manifest from a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, WorkspaceError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse a workspace manifest from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML is invalid.
    pub fn parse(content: &str) -> Result<Self, WorkspaceError> {
        toml::from_str(content).map_err(|e| ManifestError::Parse(e).into())
    }

    /// Check if this manifest defines a workspace.
    #[must_use]
    pub fn is_workspace(&self) -> bool {
        self.workspace.is_some()
    }

    /// Check if this manifest is also a package (virtual workspace if not).
    #[must_use]
    pub fn is_package(&self) -> bool {
        self.package.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn parse_workspace_manifest() {
        let toml = r#"
[workspace]
members = ["crates/*"]

[workspace.package]
version = "1.0.0"
edition = "2025"
authors = ["Team <team@example.com>"]

[workspace.dependencies]
http = "1.0"
json = { version = "2.0", features = ["pretty"] }
"#;
        let manifest = WorkspaceManifest::parse(toml).unwrap();
        assert!(manifest.is_workspace());
        assert!(!manifest.is_package());

        let ws = manifest.workspace.unwrap();
        assert_eq!(ws.members, vec!["crates/*"]);
        assert_eq!(ws.dependencies.len(), 2);
    }

    #[test]
    fn parse_workspace_with_package() {
        let toml = r#"
[workspace]
members = ["crates/*"]

[package]
name = "root-pkg"
version = "0.1.0"
edition = "2025"
"#;
        let manifest = WorkspaceManifest::parse(toml).unwrap();
        assert!(manifest.is_workspace());
        assert!(manifest.is_package());

        let pkg = manifest.package.unwrap();
        assert_eq!(pkg.name, "root-pkg");
    }

    #[test]
    fn load_workspace() {
        let tmp = TempDir::new().unwrap();

        // Create workspace root manifest
        let root_manifest = r#"
[workspace]
members = ["crates/*"]
"#;
        fs::write(tmp.path().join(MANIFEST_FILE), root_manifest).unwrap();

        // Create a member package
        let member_dir = tmp.path().join("crates/member-a");
        fs::create_dir_all(member_dir.join("src")).unwrap();
        let member_manifest = r#"
[package]
name = "member-a"
version = "0.1.0"
edition = "2025"
"#;
        fs::write(member_dir.join(MANIFEST_FILE), member_manifest).unwrap();
        fs::write(member_dir.join("src/lib.strat"), "// lib").unwrap();

        // Load the workspace
        let workspace = Workspace::load(tmp.path()).unwrap();
        assert_eq!(workspace.members.len(), 1);
        assert_eq!(workspace.members[0].name, "member-a");
    }

    #[test]
    fn workspace_inheritance() {
        let toml = r#"
[package]
name = "member-pkg"
version.workspace = true
edition.workspace = true
"#;
        let manifest = WorkspaceManifest::parse(toml).unwrap();
        let pkg = manifest.package.unwrap();

        assert!(matches!(pkg.version, VersionOrWorkspace::Workspace(_)));
        assert!(matches!(pkg.edition, EditionOrWorkspace::Workspace(_)));
    }
}
