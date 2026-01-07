//! Dependency resolution for Stratum packages.
//!
//! This module provides:
//! - Version requirement parsing and validation
//! - Dependency graph construction
//! - Conflict detection for incompatible version requirements
//! - Resolution of dependencies from a manifest

use crate::{Dependency, DependencySpec, Manifest};
use semver::{Version, VersionReq};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use thiserror::Error;

/// Errors that can occur during dependency resolution.
#[derive(Error, Debug)]
pub enum ResolveError {
    /// Invalid version requirement syntax.
    #[error("invalid version requirement '{requirement}' for package '{package}': {reason}")]
    InvalidVersionReq {
        package: String,
        requirement: String,
        reason: String,
    },

    /// Conflicting version requirements for the same package.
    #[error("conflicting requirements for package '{package}':\n  {}", format_requirements(.requirements))]
    ConflictingRequirements {
        package: String,
        requirements: Vec<VersionRequirement>,
    },

    /// Circular dependency detected.
    #[error("circular dependency detected: {}", .cycle.join(" -> "))]
    CircularDependency { cycle: Vec<String> },

    /// Missing dependency (path or git not found).
    #[error("dependency '{package}' not found: {reason}")]
    MissingDependency { package: String, reason: String },
}

fn format_requirements(reqs: &[VersionRequirement]) -> String {
    reqs.iter()
        .map(|r| format!("{} (from {})", r.version_req, r.source))
        .collect::<Vec<_>>()
        .join("\n  ")
}

/// The source of a dependency.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DependencySource {
    /// Package from a registry with a version requirement.
    Registry { version_req: VersionReq },
    /// Local path dependency.
    Path { path: String },
    /// Git repository dependency.
    Git {
        url: String,
        reference: GitReference,
    },
}

/// A git reference (branch, tag, or revision).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GitReference {
    /// A branch name.
    Branch(String),
    /// A tag name.
    Tag(String),
    /// A specific commit revision.
    Rev(String),
    /// Default branch (HEAD).
    DefaultBranch,
}

impl std::fmt::Display for GitReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Branch(b) => write!(f, "branch:{b}"),
            Self::Tag(t) => write!(f, "tag:{t}"),
            Self::Rev(r) => write!(f, "rev:{r}"),
            Self::DefaultBranch => write!(f, "HEAD"),
        }
    }
}

impl std::fmt::Display for DependencySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Registry { version_req } => write!(f, "registry ({version_req})"),
            Self::Path { path } => write!(f, "path:{path}"),
            Self::Git { url, reference } => write!(f, "git:{url}#{reference}"),
        }
    }
}

/// A version requirement with its source context.
#[derive(Debug, Clone)]
pub struct VersionRequirement {
    /// The parsed version requirement.
    pub version_req: VersionReq,
    /// Human-readable source description (e.g., "dependencies", "dev-dependencies").
    pub source: String,
}

/// A resolved dependency with all information needed to fetch it.
#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    /// The package name.
    pub name: String,
    /// The source of the dependency.
    pub source: DependencySource,
    /// Features to enable.
    pub features: Vec<String>,
    /// Whether default features are enabled.
    pub default_features: bool,
    /// Whether this is an optional dependency.
    pub optional: bool,
    /// Which section this came from.
    pub section: DependencySection,
}

/// Which section a dependency came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DependencySection {
    /// Regular runtime dependencies.
    Dependencies,
    /// Development-only dependencies.
    Dev,
    /// Build-time dependencies.
    Build,
}

impl std::fmt::Display for DependencySection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dependencies => write!(f, "dependencies"),
            Self::Dev => write!(f, "dev-dependencies"),
            Self::Build => write!(f, "build-dependencies"),
        }
    }
}

/// The result of dependency resolution.
#[derive(Debug, Clone)]
pub struct ResolvedDependencies {
    /// All resolved dependencies by name.
    pub dependencies: BTreeMap<String, ResolvedDependency>,
    /// Dependencies that have version requirements (registry deps).
    /// Maps package name to all collected requirements.
    pub version_requirements: HashMap<String, Vec<VersionRequirement>>,
}

impl ResolvedDependencies {
    /// Returns true if no dependencies were found.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.dependencies.is_empty()
    }

    /// Returns the number of resolved dependencies.
    #[must_use]
    pub fn len(&self) -> usize {
        self.dependencies.len()
    }

    /// Get a dependency by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ResolvedDependency> {
        self.dependencies.get(name)
    }

    /// Iterate over all resolved dependencies.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ResolvedDependency)> {
        self.dependencies.iter()
    }

    /// Get all registry dependencies (those with version requirements).
    pub fn registry_deps(&self) -> impl Iterator<Item = &ResolvedDependency> {
        self.dependencies
            .values()
            .filter(|d| matches!(d.source, DependencySource::Registry { .. }))
    }

    /// Get all path dependencies.
    pub fn path_deps(&self) -> impl Iterator<Item = &ResolvedDependency> {
        self.dependencies
            .values()
            .filter(|d| matches!(d.source, DependencySource::Path { .. }))
    }

    /// Get all git dependencies.
    pub fn git_deps(&self) -> impl Iterator<Item = &ResolvedDependency> {
        self.dependencies
            .values()
            .filter(|d| matches!(d.source, DependencySource::Git { .. }))
    }
}

/// Dependency resolver for Stratum packages.
#[derive(Debug, Default)]
pub struct Resolver {
    /// Whether to include dev dependencies in resolution.
    include_dev: bool,
    /// Whether to include build dependencies in resolution.
    include_build: bool,
}

impl Resolver {
    /// Create a new resolver with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Include dev-dependencies in resolution.
    #[must_use]
    pub fn with_dev(mut self, include: bool) -> Self {
        self.include_dev = include;
        self
    }

    /// Include build-dependencies in resolution.
    #[must_use]
    pub fn with_build(mut self, include: bool) -> Self {
        self.include_build = include;
        self
    }

    /// Resolve dependencies from a manifest.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A version requirement is invalid
    /// - Conflicting requirements exist for the same package
    pub fn resolve(&self, manifest: &Manifest) -> Result<ResolvedDependencies, ResolveError> {
        let mut dependencies = BTreeMap::new();
        let mut version_requirements: HashMap<String, Vec<VersionRequirement>> = HashMap::new();

        // Process regular dependencies
        for (name, spec) in &manifest.dependencies {
            let resolved = self.resolve_dependency(name, spec, DependencySection::Dependencies)?;
            if let DependencySource::Registry { ref version_req } = resolved.source {
                version_requirements
                    .entry(name.clone())
                    .or_default()
                    .push(VersionRequirement {
                        version_req: version_req.clone(),
                        source: DependencySection::Dependencies.to_string(),
                    });
            }
            dependencies.insert(name.clone(), resolved);
        }

        // Process dev-dependencies if requested
        if self.include_dev {
            for (name, spec) in &manifest.dev_dependencies {
                let resolved = self.resolve_dependency(name, spec, DependencySection::Dev)?;

                // Check for conflicts with existing dependencies
                if let Some(existing) = dependencies.get(name) {
                    self.check_conflict(name, existing, &resolved, &version_requirements)?;
                }

                if let DependencySource::Registry { ref version_req } = resolved.source {
                    version_requirements
                        .entry(name.clone())
                        .or_default()
                        .push(VersionRequirement {
                            version_req: version_req.clone(),
                            source: DependencySection::Dev.to_string(),
                        });
                }
                dependencies.insert(name.clone(), resolved);
            }
        }

        // Process build-dependencies if requested
        if self.include_build {
            for (name, spec) in &manifest.build_dependencies {
                let resolved = self.resolve_dependency(name, spec, DependencySection::Build)?;

                // Check for conflicts with existing dependencies
                if let Some(existing) = dependencies.get(name) {
                    self.check_conflict(name, existing, &resolved, &version_requirements)?;
                }

                if let DependencySource::Registry { ref version_req } = resolved.source {
                    version_requirements
                        .entry(name.clone())
                        .or_default()
                        .push(VersionRequirement {
                            version_req: version_req.clone(),
                            source: DependencySection::Build.to_string(),
                        });
                }
                dependencies.insert(name.clone(), resolved);
            }
        }

        // Check for version conflicts across all registry dependencies
        self.check_version_conflicts(&version_requirements)?;

        Ok(ResolvedDependencies {
            dependencies,
            version_requirements,
        })
    }

    /// Resolve a single dependency specification.
    fn resolve_dependency(
        &self,
        name: &str,
        spec: &DependencySpec,
        section: DependencySection,
    ) -> Result<ResolvedDependency, ResolveError> {
        match spec {
            DependencySpec::Simple(version_str) => {
                let version_req = parse_version_req(name, version_str)?;
                Ok(ResolvedDependency {
                    name: name.to_string(),
                    source: DependencySource::Registry { version_req },
                    features: Vec::new(),
                    default_features: true,
                    optional: false,
                    section,
                })
            }
            DependencySpec::Detailed(dep) => {
                let source = if let Some(ref path) = dep.path {
                    DependencySource::Path { path: path.clone() }
                } else if let Some(ref git) = dep.git {
                    let reference = if let Some(ref branch) = dep.branch {
                        GitReference::Branch(branch.clone())
                    } else if let Some(ref tag) = dep.tag {
                        GitReference::Tag(tag.clone())
                    } else if let Some(ref rev) = dep.rev {
                        GitReference::Rev(rev.clone())
                    } else {
                        GitReference::DefaultBranch
                    };
                    DependencySource::Git {
                        url: git.clone(),
                        reference,
                    }
                } else if let Some(ref version_str) = dep.version {
                    let version_req = parse_version_req(name, version_str)?;
                    DependencySource::Registry { version_req }
                } else {
                    // No source specified - treat as wildcard registry dependency
                    DependencySource::Registry {
                        version_req: VersionReq::STAR,
                    }
                };

                Ok(ResolvedDependency {
                    name: name.to_string(),
                    source,
                    features: dep.features.clone(),
                    default_features: dep.default_features,
                    optional: dep.optional,
                    section,
                })
            }
        }
    }

    /// Check for conflicts between an existing and new dependency.
    fn check_conflict(
        &self,
        name: &str,
        existing: &ResolvedDependency,
        new: &ResolvedDependency,
        version_requirements: &HashMap<String, Vec<VersionRequirement>>,
    ) -> Result<(), ResolveError> {
        // Path/git dependencies can override registry deps without conflict
        // But registry deps with conflicting requirements are a problem
        match (&existing.source, &new.source) {
            (DependencySource::Registry { .. }, DependencySource::Registry { .. }) => {
                // Both are registry deps - check version compatibility
                if let Some(reqs) = version_requirements.get(name) {
                    if !are_requirements_compatible(reqs) {
                        return Err(ResolveError::ConflictingRequirements {
                            package: name.to_string(),
                            requirements: reqs.clone(),
                        });
                    }
                }
            }
            (DependencySource::Path { path: p1 }, DependencySource::Path { path: p2 }) => {
                if p1 != p2 {
                    return Err(ResolveError::ConflictingRequirements {
                        package: name.to_string(),
                        requirements: vec![
                            VersionRequirement {
                                version_req: VersionReq::STAR,
                                source: format!("path:{p1}"),
                            },
                            VersionRequirement {
                                version_req: VersionReq::STAR,
                                source: format!("path:{p2}"),
                            },
                        ],
                    });
                }
            }
            _ => {
                // Mixed sources - might be intentional override, allow for now
            }
        }
        Ok(())
    }

    /// Check for version conflicts across all collected requirements.
    fn check_version_conflicts(
        &self,
        requirements: &HashMap<String, Vec<VersionRequirement>>,
    ) -> Result<(), ResolveError> {
        for (name, reqs) in requirements {
            if reqs.len() > 1 && !are_requirements_compatible(reqs) {
                return Err(ResolveError::ConflictingRequirements {
                    package: name.clone(),
                    requirements: reqs.clone(),
                });
            }
        }
        Ok(())
    }
}

/// Parse a version requirement string.
fn parse_version_req(package: &str, version_str: &str) -> Result<VersionReq, ResolveError> {
    // Handle bare version numbers (e.g., "1.0.0") by treating them as caret requirements
    let normalized = if version_str
        .chars()
        .next()
        .map_or(false, |c| c.is_ascii_digit())
    {
        // If it starts with a digit, it's a bare version - treat as caret
        format!("^{version_str}")
    } else {
        version_str.to_string()
    };

    VersionReq::parse(&normalized).map_err(|e| ResolveError::InvalidVersionReq {
        package: package.to_string(),
        requirement: version_str.to_string(),
        reason: e.to_string(),
    })
}

/// Check if multiple version requirements can potentially be satisfied by the same version.
///
/// This is a heuristic check - it doesn't guarantee a solution exists,
/// but can detect obvious conflicts.
fn are_requirements_compatible(requirements: &[VersionRequirement]) -> bool {
    if requirements.len() <= 1 {
        return true;
    }

    // Try a set of common versions to see if any satisfies all requirements
    let test_versions = [
        "0.0.1", "0.1.0", "0.2.0", "0.5.0", "1.0.0", "1.1.0", "1.5.0", "2.0.0", "2.1.0", "3.0.0",
        "5.0.0", "10.0.0",
    ];

    for version_str in &test_versions {
        if let Ok(version) = Version::parse(version_str) {
            if requirements.iter().all(|r| r.version_req.matches(&version)) {
                return true;
            }
        }
    }

    // If no test version satisfies all, check for obvious conflicts:
    // - `^1.x` and `^2.x` are definitely incompatible
    // - `>=2.0` and `<1.5` are definitely incompatible

    // Extract major version requirements if possible
    let mut major_versions: BTreeSet<u64> = BTreeSet::new();
    for req in requirements {
        // Check each comparator in the requirement
        for comparator in &req.version_req.comparators {
            // Caret and tilde requirements lock the major version
            if comparator.op == semver::Op::Caret || comparator.op == semver::Op::Tilde {
                major_versions.insert(comparator.major);
            }
        }
    }

    // If we have caret requirements for different major versions, they conflict
    if major_versions.len() > 1 {
        // Check if any of these could potentially overlap
        // ^0.x versions are special - ^0.1 and ^0.2 are incompatible
        let min = *major_versions.first().unwrap();
        let max = *major_versions.last().unwrap();
        if min == 0 && max == 0 {
            // All are ^0.x - check minor versions more carefully
            // For now, assume they might be compatible
            return true;
        }
        if max - min > 0 {
            // Different non-zero major versions - definitely incompatible
            return false;
        }
    }

    // If we can't prove incompatibility, assume compatible
    // A real resolver would do proper constraint solving
    true
}

/// Check a version against a requirement.
#[must_use]
pub fn matches_version(version: &Version, requirement: &VersionReq) -> bool {
    requirement.matches(version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dependency;

    fn make_manifest(deps: Vec<(&str, DependencySpec)>) -> Manifest {
        let mut manifest = Manifest::default();
        manifest.package.name = "test-pkg".to_string();
        manifest.package.version = "0.1.0".to_string();
        for (name, spec) in deps {
            manifest.dependencies.insert(name.to_string(), spec);
        }
        manifest
    }

    #[test]
    fn test_parse_version_req_caret() {
        let req = parse_version_req("test", "^1.0").unwrap();
        assert!(req.matches(&Version::parse("1.0.0").unwrap()));
        assert!(req.matches(&Version::parse("1.5.0").unwrap()));
        assert!(!req.matches(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_parse_version_req_tilde() {
        let req = parse_version_req("test", "~1.2").unwrap();
        assert!(req.matches(&Version::parse("1.2.0").unwrap()));
        assert!(req.matches(&Version::parse("1.2.9").unwrap()));
        assert!(!req.matches(&Version::parse("1.3.0").unwrap()));
    }

    #[test]
    fn test_parse_version_req_bare() {
        // Bare versions should be treated as caret
        let req = parse_version_req("test", "1.0.0").unwrap();
        assert!(req.matches(&Version::parse("1.0.0").unwrap()));
        assert!(req.matches(&Version::parse("1.5.0").unwrap()));
        assert!(!req.matches(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_parse_version_req_exact() {
        let req = parse_version_req("test", "=1.0.0").unwrap();
        assert!(req.matches(&Version::parse("1.0.0").unwrap()));
        assert!(!req.matches(&Version::parse("1.0.1").unwrap()));
    }

    #[test]
    fn test_parse_version_req_range() {
        let req = parse_version_req("test", ">=1.0, <2.0").unwrap();
        assert!(req.matches(&Version::parse("1.0.0").unwrap()));
        assert!(req.matches(&Version::parse("1.9.9").unwrap()));
        assert!(!req.matches(&Version::parse("2.0.0").unwrap()));
        assert!(!req.matches(&Version::parse("0.9.0").unwrap()));
    }

    #[test]
    fn test_parse_version_req_star() {
        let req = parse_version_req("test", "*").unwrap();
        assert!(req.matches(&Version::parse("0.0.1").unwrap()));
        assert!(req.matches(&Version::parse("99.99.99").unwrap()));
    }

    #[test]
    fn test_parse_version_req_invalid() {
        let result = parse_version_req("test", "not-a-version");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_simple_deps() {
        let manifest = make_manifest(vec![
            ("http", DependencySpec::Simple("^1.0".to_string())),
            ("json", DependencySpec::Simple("2.0".to_string())),
        ]);

        let resolver = Resolver::new();
        let resolved = resolver.resolve(&manifest).unwrap();

        assert_eq!(resolved.len(), 2);
        assert!(resolved.get("http").is_some());
        assert!(resolved.get("json").is_some());
    }

    #[test]
    fn test_resolve_path_dep() {
        let manifest = make_manifest(vec![(
            "local-lib",
            DependencySpec::Detailed(Dependency {
                path: Some("../local-lib".to_string()),
                ..Default::default()
            }),
        )]);

        let resolver = Resolver::new();
        let resolved = resolver.resolve(&manifest).unwrap();

        let dep = resolved.get("local-lib").unwrap();
        assert!(matches!(dep.source, DependencySource::Path { .. }));
    }

    #[test]
    fn test_resolve_git_dep() {
        let manifest = make_manifest(vec![(
            "remote-lib",
            DependencySpec::Detailed(Dependency {
                git: Some("https://github.com/example/lib".to_string()),
                branch: Some("main".to_string()),
                ..Default::default()
            }),
        )]);

        let resolver = Resolver::new();
        let resolved = resolver.resolve(&manifest).unwrap();

        let dep = resolved.get("remote-lib").unwrap();
        match &dep.source {
            DependencySource::Git { url, reference } => {
                assert_eq!(url, "https://github.com/example/lib");
                assert!(matches!(reference, GitReference::Branch(b) if b == "main"));
            }
            _ => panic!("Expected git source"),
        }
    }

    #[test]
    fn test_resolve_with_features() {
        let manifest = make_manifest(vec![(
            "json",
            DependencySpec::Detailed(Dependency {
                version: Some("2.0".to_string()),
                features: vec!["pretty".to_string(), "async".to_string()],
                ..Default::default()
            }),
        )]);

        let resolver = Resolver::new();
        let resolved = resolver.resolve(&manifest).unwrap();

        let dep = resolved.get("json").unwrap();
        assert_eq!(dep.features, vec!["pretty", "async"]);
    }

    #[test]
    fn test_detect_conflicting_requirements() {
        // ^1.0 and ^2.0 are incompatible
        let reqs = vec![
            VersionRequirement {
                version_req: VersionReq::parse("^1.0").unwrap(),
                source: "dependencies".to_string(),
            },
            VersionRequirement {
                version_req: VersionReq::parse("^2.0").unwrap(),
                source: "dev-dependencies".to_string(),
            },
        ];

        assert!(!are_requirements_compatible(&reqs));
    }

    #[test]
    fn test_compatible_requirements() {
        // ^1.0 and ^1.2 can both be satisfied by 1.2.x
        let reqs = vec![
            VersionRequirement {
                version_req: VersionReq::parse("^1.0").unwrap(),
                source: "dependencies".to_string(),
            },
            VersionRequirement {
                version_req: VersionReq::parse("^1.2").unwrap(),
                source: "dev-dependencies".to_string(),
            },
        ];

        assert!(are_requirements_compatible(&reqs));
    }

    #[test]
    fn test_resolve_includes_dev_deps() {
        let mut manifest = make_manifest(vec![(
            "http",
            DependencySpec::Simple("^1.0".to_string()),
        )]);
        manifest.dev_dependencies.insert(
            "test-utils".to_string(),
            DependencySpec::Simple("0.5".to_string()),
        );

        // Without dev deps
        let resolved = Resolver::new().resolve(&manifest).unwrap();
        assert_eq!(resolved.len(), 1);

        // With dev deps
        let resolved = Resolver::new().with_dev(true).resolve(&manifest).unwrap();
        assert_eq!(resolved.len(), 2);
        assert!(resolved.get("test-utils").is_some());
    }

    #[test]
    fn test_matches_version() {
        let req = VersionReq::parse("^1.0").unwrap();
        let v1 = Version::parse("1.5.0").unwrap();
        let v2 = Version::parse("2.0.0").unwrap();

        assert!(matches_version(&v1, &req));
        assert!(!matches_version(&v2, &req));
    }

    #[test]
    fn test_iterator_methods() {
        let manifest = make_manifest(vec![
            ("http", DependencySpec::Simple("^1.0".to_string())),
            (
                "local",
                DependencySpec::Detailed(Dependency {
                    path: Some("./local".to_string()),
                    ..Default::default()
                }),
            ),
            (
                "remote",
                DependencySpec::Detailed(Dependency {
                    git: Some("https://example.com/repo".to_string()),
                    ..Default::default()
                }),
            ),
        ]);

        let resolved = Resolver::new().resolve(&manifest).unwrap();

        assert_eq!(resolved.registry_deps().count(), 1);
        assert_eq!(resolved.path_deps().count(), 1);
        assert_eq!(resolved.git_deps().count(), 1);
    }
}

impl Default for Dependency {
    fn default() -> Self {
        Self {
            version: None,
            path: None,
            git: None,
            branch: None,
            tag: None,
            rev: None,
            features: Vec::new(),
            default_features: true,
            optional: false,
            package: None,
        }
    }
}
