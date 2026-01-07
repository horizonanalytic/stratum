//! Package management for the Stratum programming language.
//!
//! This crate provides:
//! - Parsing and validation of `stratum.toml` manifests
//! - Package structure discovery (src/, tests/, examples/)
//! - Workspace support for multi-package projects
//! - Dependency resolution and conflict detection
//! - Lock file support for reproducible builds

mod lockfile;
mod manifest;
mod package;
mod resolve;
mod workspace;

pub use lockfile::{LockError, Lockfile, LockedPackage, LOCK_FILE};
pub use manifest::{
    Dependency, DependencySpec, Edition, Manifest, ManifestError, Package, Target, TargetKind,
};
pub use package::{
    PackageLayout, PackageStructure, BENCHES_DIR, EXAMPLES_DIR, LIB_FILE, MAIN_FILE,
    MANIFEST_FILE, SOURCE_DIR, SOURCE_EXT, TESTS_DIR,
};
pub use resolve::{
    matches_version, DependencySection, DependencySource, GitReference, ResolveError,
    ResolvedDependencies, ResolvedDependency, Resolver, VersionRequirement,
};
pub use workspace::{Workspace, WorkspaceManifest, WorkspaceMember};
