//! Package structure discovery and layout.
//!
//! Discovers the conventional layout of a Stratum package:
//! ```text
//! my-package/
//! ├── stratum.toml          # Package manifest
//! ├── src/
//! │   ├── lib.strat         # Library entry point (optional)
//! │   └── main.strat        # Binary entry point (optional)
//! ├── tests/                # Integration tests
//! ├── examples/             # Example programs
//! └── benches/              # Benchmarks
//! ```

use crate::{Manifest, ManifestError, Target, TargetKind};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// The manifest filename.
pub const MANIFEST_FILE: &str = "stratum.toml";

/// The source directory.
pub const SOURCE_DIR: &str = "src";

/// Library entry point filename.
pub const LIB_FILE: &str = "lib.strat";

/// Binary entry point filename.
pub const MAIN_FILE: &str = "main.strat";

/// Tests directory.
pub const TESTS_DIR: &str = "tests";

/// Examples directory.
pub const EXAMPLES_DIR: &str = "examples";

/// Benchmarks directory.
pub const BENCHES_DIR: &str = "benches";

/// Source file extension.
pub const SOURCE_EXT: &str = "strat";

/// Errors that can occur when working with package structure.
#[derive(Error, Debug)]
pub enum PackageError {
    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),

    #[error("package directory not found: {0}")]
    NotFound(PathBuf),

    #[error("manifest not found at: {0}")]
    ManifestNotFound(PathBuf),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("no library or binary targets found")]
    NoTargets,
}

/// Discovered layout of a package.
#[derive(Debug, Clone)]
pub struct PackageLayout {
    /// Root directory of the package.
    pub root: PathBuf,

    /// Path to the manifest file.
    pub manifest_path: PathBuf,

    /// Path to source directory (if exists).
    pub src_dir: Option<PathBuf>,

    /// Path to tests directory (if exists).
    pub tests_dir: Option<PathBuf>,

    /// Path to examples directory (if exists).
    pub examples_dir: Option<PathBuf>,

    /// Path to benchmarks directory (if exists).
    pub benches_dir: Option<PathBuf>,
}

impl PackageLayout {
    /// Discover the package layout starting from a directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory doesn't exist or doesn't contain a manifest.
    pub fn discover(root: impl AsRef<Path>) -> Result<Self, PackageError> {
        let root = root.as_ref().to_path_buf();

        if !root.exists() {
            return Err(PackageError::NotFound(root));
        }

        let manifest_path = root.join(MANIFEST_FILE);
        if !manifest_path.exists() {
            return Err(PackageError::ManifestNotFound(manifest_path));
        }

        let src_dir = root.join(SOURCE_DIR);
        let tests_dir = root.join(TESTS_DIR);
        let examples_dir = root.join(EXAMPLES_DIR);
        let benches_dir = root.join(BENCHES_DIR);

        Ok(Self {
            manifest_path,
            src_dir: src_dir.exists().then_some(src_dir),
            tests_dir: tests_dir.exists().then_some(tests_dir),
            examples_dir: examples_dir.exists().then_some(examples_dir),
            benches_dir: benches_dir.exists().then_some(benches_dir),
            root,
        })
    }

    /// Find a package by searching upward from the current directory.
    ///
    /// # Errors
    ///
    /// Returns an error if no manifest is found in the directory tree.
    pub fn find_root(start: impl AsRef<Path>) -> Result<Self, PackageError> {
        let mut current = start.as_ref().to_path_buf();

        loop {
            let manifest = current.join(MANIFEST_FILE);
            if manifest.exists() {
                return Self::discover(&current);
            }

            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => return Err(PackageError::ManifestNotFound(start.as_ref().to_path_buf())),
            }
        }
    }

    /// Check if this package has a library target.
    #[must_use]
    pub fn has_lib(&self) -> bool {
        self.src_dir
            .as_ref()
            .is_some_and(|dir| dir.join(LIB_FILE).exists())
    }

    /// Check if this package has a default binary target.
    #[must_use]
    pub fn has_bin(&self) -> bool {
        self.src_dir
            .as_ref()
            .is_some_and(|dir| dir.join(MAIN_FILE).exists())
    }

    /// Get the path to the library entry point if it exists.
    #[must_use]
    pub fn lib_path(&self) -> Option<PathBuf> {
        self.src_dir.as_ref().and_then(|dir| {
            let path = dir.join(LIB_FILE);
            path.exists().then_some(path)
        })
    }

    /// Get the path to the default binary entry point if it exists.
    #[must_use]
    pub fn main_path(&self) -> Option<PathBuf> {
        self.src_dir.as_ref().and_then(|dir| {
            let path = dir.join(MAIN_FILE);
            path.exists().then_some(path)
        })
    }
}

/// Complete package structure with manifest and layout.
#[derive(Debug, Clone)]
pub struct PackageStructure {
    /// The parsed manifest.
    pub manifest: Manifest,

    /// The discovered layout.
    pub layout: PackageLayout,

    /// Discovered targets.
    pub targets: Vec<DiscoveredTarget>,
}

/// A discovered target with its kind and path.
#[derive(Debug, Clone)]
pub struct DiscoveredTarget {
    /// Name of the target.
    pub name: String,

    /// Kind of target.
    pub kind: TargetKind,

    /// Path to the source file.
    pub path: PathBuf,

    /// Whether this target was explicitly configured in the manifest.
    pub explicit: bool,
}

impl PackageStructure {
    /// Load a package from a directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the package cannot be loaded.
    pub fn load(root: impl AsRef<Path>) -> Result<Self, PackageError> {
        let layout = PackageLayout::discover(&root)?;
        let manifest = Manifest::from_path(&layout.manifest_path)?;
        let targets = Self::discover_targets(&manifest, &layout)?;

        Ok(Self {
            manifest,
            layout,
            targets,
        })
    }

    /// Find and load a package by searching upward from a directory.
    ///
    /// # Errors
    ///
    /// Returns an error if no package is found.
    pub fn find(start: impl AsRef<Path>) -> Result<Self, PackageError> {
        let layout = PackageLayout::find_root(&start)?;
        let manifest = Manifest::from_path(&layout.manifest_path)?;
        let targets = Self::discover_targets(&manifest, &layout)?;

        Ok(Self {
            manifest,
            layout,
            targets,
        })
    }

    /// Discover all targets in the package.
    fn discover_targets(
        manifest: &Manifest,
        layout: &PackageLayout,
    ) -> Result<Vec<DiscoveredTarget>, PackageError> {
        let mut targets = Vec::new();

        // Library target
        if let Some(lib) = &manifest.lib {
            // Explicit library configuration
            let path = lib.path.as_ref().map_or_else(
                || layout.root.join(SOURCE_DIR).join(LIB_FILE),
                |p| layout.root.join(p),
            );
            targets.push(DiscoveredTarget {
                name: lib.name.clone(),
                kind: TargetKind::Lib,
                path,
                explicit: true,
            });
        } else if let Some(lib_path) = layout.lib_path() {
            // Auto-discovered library
            targets.push(DiscoveredTarget {
                name: manifest.package.name.clone(),
                kind: TargetKind::Lib,
                path: lib_path,
                explicit: false,
            });
        }

        // Binary targets
        if manifest.binaries.is_empty() {
            // Auto-discover default binary
            if let Some(main_path) = layout.main_path() {
                targets.push(DiscoveredTarget {
                    name: manifest.package.name.clone(),
                    kind: TargetKind::Bin,
                    path: main_path,
                    explicit: false,
                });
            }

            // Auto-discover additional binaries in src/bin/
            let bin_dir = layout.root.join(SOURCE_DIR).join("bin");
            if bin_dir.exists() {
                targets.extend(Self::discover_in_dir(&bin_dir, TargetKind::Bin)?);
            }
        } else {
            // Explicit binary configurations
            for bin in &manifest.binaries {
                let path = bin.path.as_ref().map_or_else(
                    || {
                        layout
                            .root
                            .join(SOURCE_DIR)
                            .join("bin")
                            .join(&bin.name)
                            .with_extension(SOURCE_EXT)
                    },
                    |p| layout.root.join(p),
                );
                targets.push(DiscoveredTarget {
                    name: bin.name.clone(),
                    kind: TargetKind::Bin,
                    path,
                    explicit: true,
                });
            }
        }

        // Test targets
        if manifest.tests.is_empty() {
            // Auto-discover tests
            if let Some(tests_dir) = &layout.tests_dir {
                targets.extend(Self::discover_in_dir(tests_dir, TargetKind::Test)?);
            }
        } else {
            targets.extend(Self::explicit_targets(
                &manifest.tests,
                &layout.root,
                TargetKind::Test,
                TESTS_DIR,
            ));
        }

        // Example targets
        if manifest.examples.is_empty() {
            // Auto-discover examples
            if let Some(examples_dir) = &layout.examples_dir {
                targets.extend(Self::discover_in_dir(examples_dir, TargetKind::Example)?);
            }
        } else {
            targets.extend(Self::explicit_targets(
                &manifest.examples,
                &layout.root,
                TargetKind::Example,
                EXAMPLES_DIR,
            ));
        }

        // Benchmark targets
        if manifest.benches.is_empty() {
            // Auto-discover benchmarks
            if let Some(benches_dir) = &layout.benches_dir {
                targets.extend(Self::discover_in_dir(benches_dir, TargetKind::Bench)?);
            }
        } else {
            targets.extend(Self::explicit_targets(
                &manifest.benches,
                &layout.root,
                TargetKind::Bench,
                BENCHES_DIR,
            ));
        }

        if targets.is_empty() {
            return Err(PackageError::NoTargets);
        }

        Ok(targets)
    }

    /// Discover targets in a directory.
    fn discover_in_dir(
        dir: &Path,
        kind: TargetKind,
    ) -> Result<Vec<DiscoveredTarget>, PackageError> {
        let mut targets = Vec::new();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == SOURCE_EXT) {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();

                targets.push(DiscoveredTarget {
                    name,
                    kind,
                    path,
                    explicit: false,
                });
            }
        }

        Ok(targets)
    }

    /// Convert explicit target configurations to discovered targets.
    fn explicit_targets(
        configs: &[Target],
        root: &Path,
        kind: TargetKind,
        default_dir: &str,
    ) -> Vec<DiscoveredTarget> {
        configs
            .iter()
            .map(|t| {
                let path = t.path.as_ref().map_or_else(
                    || {
                        root.join(default_dir)
                            .join(&t.name)
                            .with_extension(SOURCE_EXT)
                    },
                    |p| root.join(p),
                );
                DiscoveredTarget {
                    name: t.name.clone(),
                    kind,
                    path,
                    explicit: true,
                }
            })
            .collect()
    }

    /// Get all library targets.
    #[must_use]
    pub fn libs(&self) -> Vec<&DiscoveredTarget> {
        self.targets
            .iter()
            .filter(|t| t.kind == TargetKind::Lib)
            .collect()
    }

    /// Get all binary targets.
    #[must_use]
    pub fn bins(&self) -> Vec<&DiscoveredTarget> {
        self.targets
            .iter()
            .filter(|t| t.kind == TargetKind::Bin)
            .collect()
    }

    /// Get all test targets.
    #[must_use]
    pub fn tests(&self) -> Vec<&DiscoveredTarget> {
        self.targets
            .iter()
            .filter(|t| t.kind == TargetKind::Test)
            .collect()
    }

    /// Get all example targets.
    #[must_use]
    pub fn examples(&self) -> Vec<&DiscoveredTarget> {
        self.targets
            .iter()
            .filter(|t| t.kind == TargetKind::Example)
            .collect()
    }

    /// Get all benchmark targets.
    #[must_use]
    pub fn benches(&self) -> Vec<&DiscoveredTarget> {
        self.targets
            .iter()
            .filter(|t| t.kind == TargetKind::Bench)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_package(dir: &Path, manifest: &str) {
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join(MANIFEST_FILE), manifest).unwrap();
    }

    #[test]
    fn discover_minimal_package() {
        let tmp = TempDir::new().unwrap();
        let manifest = r#"
[package]
name = "test-pkg"
version = "0.1.0"
edition = "2025"
"#;
        create_test_package(tmp.path(), manifest);
        fs::write(tmp.path().join("src/main.strat"), "fx main() {}").unwrap();

        let layout = PackageLayout::discover(tmp.path()).unwrap();
        assert!(layout.src_dir.is_some());
        assert!(layout.has_bin());
        assert!(!layout.has_lib());
    }

    #[test]
    fn discover_lib_package() {
        let tmp = TempDir::new().unwrap();
        let manifest = r#"
[package]
name = "test-lib"
version = "0.1.0"
edition = "2025"
"#;
        create_test_package(tmp.path(), manifest);
        fs::write(tmp.path().join("src/lib.strat"), "// library").unwrap();

        let layout = PackageLayout::discover(tmp.path()).unwrap();
        assert!(layout.has_lib());
        assert!(!layout.has_bin());
    }

    #[test]
    fn discover_with_tests() {
        let tmp = TempDir::new().unwrap();
        let manifest = r#"
[package]
name = "test-pkg"
version = "0.1.0"
edition = "2025"
"#;
        create_test_package(tmp.path(), manifest);
        fs::write(tmp.path().join("src/main.strat"), "fx main() {}").unwrap();

        fs::create_dir(tmp.path().join("tests")).unwrap();
        fs::write(tmp.path().join("tests/integration.strat"), "// test").unwrap();

        let layout = PackageLayout::discover(tmp.path()).unwrap();
        assert!(layout.tests_dir.is_some());
    }

    #[test]
    fn find_root_from_subdirectory() {
        let tmp = TempDir::new().unwrap();
        let manifest = r#"
[package]
name = "test-pkg"
version = "0.1.0"
edition = "2025"
"#;
        create_test_package(tmp.path(), manifest);
        fs::write(tmp.path().join("src/main.strat"), "fx main() {}").unwrap();

        // Search from src/ directory
        let layout = PackageLayout::find_root(tmp.path().join("src")).unwrap();
        assert_eq!(layout.root, tmp.path());
    }

    #[test]
    fn load_package_structure() {
        let tmp = TempDir::new().unwrap();
        let manifest = r#"
[package]
name = "my-app"
version = "1.0.0"
edition = "2025"
"#;
        create_test_package(tmp.path(), manifest);
        fs::write(tmp.path().join("src/main.strat"), "fx main() {}").unwrap();
        fs::write(tmp.path().join("src/lib.strat"), "// lib").unwrap();

        let pkg = PackageStructure::load(tmp.path()).unwrap();
        assert_eq!(pkg.manifest.package.name, "my-app");
        assert_eq!(pkg.libs().len(), 1);
        assert_eq!(pkg.bins().len(), 1);
    }
}
