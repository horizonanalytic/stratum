//! Implementation of the `stratum update` command.

use anyhow::{Context, Result};
use std::path::Path;
use stratum_pkg::{LockError, Lockfile, Manifest, LOCK_FILE, MANIFEST_FILE};

/// Options for the update command.
#[derive(Debug, Default)]
pub struct UpdateOptions {
    /// Only update the specified packages.
    pub packages: Vec<String>,
    /// Perform a dry run without writing changes.
    pub dry_run: bool,
}

/// Result of an update operation.
#[derive(Debug, Default)]
pub struct UpdateResult {
    /// Packages that were added.
    pub added: Vec<String>,
    /// Packages that were removed.
    pub removed: Vec<String>,
    /// Packages that were updated (version changed).
    pub updated: Vec<PackageChange>,
    /// Whether the lock file was modified.
    pub modified: bool,
}

/// A change to a package.
#[derive(Debug)]
pub struct PackageChange {
    /// Package name.
    pub name: String,
    /// Old version/spec.
    pub old: String,
    /// New version/spec.
    pub new: String,
}

impl UpdateResult {
    /// Returns true if no changes were made.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.updated.is_empty()
    }

    /// Print a summary of the changes.
    pub fn print_summary(&self) {
        if self.is_empty() {
            println!("No updates available.");
            return;
        }

        if !self.added.is_empty() {
            println!("Added:");
            for name in &self.added {
                println!("  + {name}");
            }
        }

        if !self.removed.is_empty() {
            println!("Removed:");
            for name in &self.removed {
                println!("  - {name}");
            }
        }

        if !self.updated.is_empty() {
            println!("Updated:");
            for change in &self.updated {
                println!("  {} {} -> {}", change.name, change.old, change.new);
            }
        }
    }
}

/// Update dependencies to their latest compatible versions.
///
/// Currently, without a package registry, this command:
/// 1. Re-resolves all dependencies from the manifest
/// 2. Compares with the existing lock file
/// 3. Regenerates the lock file if changes are detected
///
/// Once a registry is available, this will fetch the latest compatible
/// versions for each dependency.
pub fn update_dependencies(options: UpdateOptions) -> Result<UpdateResult> {
    let manifest_path = Path::new(MANIFEST_FILE);
    let lock_path = Path::new(LOCK_FILE);

    // Check if manifest exists
    if !manifest_path.exists() {
        return Err(anyhow::anyhow!(
            "No {} found in current directory. Run `stratum init` first.",
            MANIFEST_FILE
        ));
    }

    // Load manifest
    let manifest = Manifest::from_path(manifest_path).context("Failed to read manifest")?;

    // Load existing lock file if present
    let old_lockfile = if lock_path.exists() {
        Some(Lockfile::from_path(lock_path).context("Failed to read lock file")?)
    } else {
        None
    };

    // Generate new lock file from current manifest
    let new_lockfile =
        Lockfile::generate(&manifest, true).context("Failed to resolve dependencies")?;

    // Compare and compute changes
    let result = compute_changes(&old_lockfile, &new_lockfile, &options.packages);

    // Write the new lock file if not a dry run and there are changes
    if !options.dry_run && (result.modified || old_lockfile.is_none()) {
        new_lockfile
            .write(lock_path)
            .context("Failed to write lock file")?;

        if old_lockfile.is_none() {
            println!("Created {LOCK_FILE}");
        } else if result.modified {
            println!("Updated {LOCK_FILE}");
        }
    } else if options.dry_run && result.modified {
        println!("Would update {LOCK_FILE} (dry run)");
    }

    Ok(result)
}

/// Compute the changes between old and new lock files.
fn compute_changes(
    old: &Option<Lockfile>,
    new: &Lockfile,
    filter_packages: &[String],
) -> UpdateResult {
    let mut result = UpdateResult::default();

    let old_packages: std::collections::HashMap<_, _> = old
        .as_ref()
        .map(|l| l.packages.iter().map(|p| (&p.name, p)).collect())
        .unwrap_or_default();

    let new_packages: std::collections::HashMap<_, _> =
        new.packages.iter().map(|p| (&p.name, p)).collect();

    // Check for added packages
    for (name, _pkg) in &new_packages {
        if !old_packages.contains_key(name) {
            if filter_packages.is_empty() || filter_packages.iter().any(|f| f == *name) {
                result.added.push((*name).clone());
                result.modified = true;
            }
        }
    }

    // Check for removed packages
    for (name, _pkg) in &old_packages {
        if !new_packages.contains_key(name) {
            if filter_packages.is_empty() || filter_packages.iter().any(|f| f == *name) {
                result.removed.push((*name).clone());
                result.modified = true;
            }
        }
    }

    // Check for updated packages
    for (name, new_pkg) in &new_packages {
        if let Some(old_pkg) = old_packages.get(name) {
            if !filter_packages.is_empty() && !filter_packages.iter().any(|f| f == *name) {
                continue;
            }

            // Compare versions/specs
            let old_spec = format_package_spec(old_pkg);
            let new_spec = format_package_spec(new_pkg);

            if old_spec != new_spec {
                result.updated.push(PackageChange {
                    name: (*name).clone(),
                    old: old_spec,
                    new: new_spec,
                });
                result.modified = true;
            }
        }
    }

    // Sort for consistent output
    result.added.sort();
    result.removed.sort();
    result.updated.sort_by(|a, b| a.name.cmp(&b.name));

    result
}

/// Format a package spec for display.
fn format_package_spec(pkg: &stratum_pkg::LockedPackage) -> String {
    match pkg.source.as_str() {
        "registry" => pkg.version.clone().unwrap_or_else(|| "*".to_string()),
        "path" => format!("path:{}", pkg.path.as_deref().unwrap_or("?")),
        "git" => {
            let url = pkg.git.as_deref().unwrap_or("?");
            if let Some(ref branch) = pkg.branch {
                format!("git:{url}#{branch}")
            } else if let Some(ref tag) = pkg.tag {
                format!("git:{url}@{tag}")
            } else if let Some(ref rev) = pkg.rev {
                format!("git:{url}@{}", &rev[..7.min(rev.len())])
            } else {
                format!("git:{url}")
            }
        }
        _ => pkg.source.clone(),
    }
}

/// Sync the lock file with the manifest without updating versions.
///
/// This is useful for ensuring the lock file reflects the current manifest
/// without fetching new versions.
pub fn sync_lockfile() -> Result<()> {
    let manifest_path = Path::new(MANIFEST_FILE);
    let lock_path = Path::new(LOCK_FILE);

    if !manifest_path.exists() {
        return Err(anyhow::anyhow!(
            "No {} found in current directory",
            MANIFEST_FILE
        ));
    }

    let manifest = Manifest::from_path(manifest_path).context("Failed to read manifest")?;

    // Check if lock file exists and is in sync
    if lock_path.exists() {
        let lockfile = Lockfile::from_path(lock_path).context("Failed to read lock file")?;
        match lockfile.check_sync(&manifest) {
            Ok(()) => {
                println!("Lock file is up to date");
                return Ok(());
            }
            Err(LockError::OutOfSync { reason }) => {
                println!("Lock file out of sync: {reason}");
                println!("Regenerating...");
            }
            Err(e) => return Err(e.into()),
        }
    }

    // Generate and write new lock file
    let lockfile = Lockfile::generate(&manifest, true).context("Failed to resolve dependencies")?;
    lockfile
        .write(lock_path)
        .context("Failed to write lock file")?;

    println!("Lock file synchronized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use stratum_pkg::LockedPackage;

    fn make_locked_package(name: &str, version: &str) -> LockedPackage {
        LockedPackage {
            name: name.to_string(),
            version: Some(version.to_string()),
            source: "registry".to_string(),
            path: None,
            git: None,
            branch: None,
            tag: None,
            rev: None,
            features: Vec::new(),
            checksum: None,
            section: Some("dependencies".to_string()),
        }
    }

    #[test]
    fn test_compute_changes_no_old() {
        let new = Lockfile {
            version: 1,
            packages: vec![make_locked_package("http", "^1.0")],
        };

        let result = compute_changes(&None, &new, &[]);

        assert_eq!(result.added, vec!["http"]);
        assert!(result.removed.is_empty());
        assert!(result.updated.is_empty());
        assert!(result.modified);
    }

    #[test]
    fn test_compute_changes_added() {
        let old = Lockfile {
            version: 1,
            packages: vec![make_locked_package("http", "^1.0")],
        };

        let new = Lockfile {
            version: 1,
            packages: vec![
                make_locked_package("http", "^1.0"),
                make_locked_package("json", "^2.0"),
            ],
        };

        let result = compute_changes(&Some(old), &new, &[]);

        assert_eq!(result.added, vec!["json"]);
        assert!(result.removed.is_empty());
        assert!(result.updated.is_empty());
        assert!(result.modified);
    }

    #[test]
    fn test_compute_changes_removed() {
        let old = Lockfile {
            version: 1,
            packages: vec![
                make_locked_package("http", "^1.0"),
                make_locked_package("json", "^2.0"),
            ],
        };

        let new = Lockfile {
            version: 1,
            packages: vec![make_locked_package("http", "^1.0")],
        };

        let result = compute_changes(&Some(old), &new, &[]);

        assert!(result.added.is_empty());
        assert_eq!(result.removed, vec!["json"]);
        assert!(result.updated.is_empty());
        assert!(result.modified);
    }

    #[test]
    fn test_compute_changes_updated() {
        let old = Lockfile {
            version: 1,
            packages: vec![make_locked_package("http", "^1.0")],
        };

        let new = Lockfile {
            version: 1,
            packages: vec![make_locked_package("http", "^2.0")],
        };

        let result = compute_changes(&Some(old), &new, &[]);

        assert!(result.added.is_empty());
        assert!(result.removed.is_empty());
        assert_eq!(result.updated.len(), 1);
        assert_eq!(result.updated[0].name, "http");
        assert_eq!(result.updated[0].old, "^1.0");
        assert_eq!(result.updated[0].new, "^2.0");
        assert!(result.modified);
    }

    #[test]
    fn test_compute_changes_no_change() {
        let old = Lockfile {
            version: 1,
            packages: vec![make_locked_package("http", "^1.0")],
        };

        let new = old.clone();

        let result = compute_changes(&Some(old), &new, &[]);

        assert!(result.is_empty());
        assert!(!result.modified);
    }

    #[test]
    fn test_compute_changes_filter() {
        let old = Lockfile {
            version: 1,
            packages: vec![make_locked_package("http", "^1.0")],
        };

        let new = Lockfile {
            version: 1,
            packages: vec![
                make_locked_package("http", "^2.0"), // Changed
                make_locked_package("json", "^1.0"), // Added
            ],
        };

        // Filter to only "http"
        let result = compute_changes(&Some(old), &new, &["http".to_string()]);

        assert!(result.added.is_empty()); // json filtered out
        assert!(result.removed.is_empty());
        assert_eq!(result.updated.len(), 1);
        assert_eq!(result.updated[0].name, "http");
    }

    #[test]
    fn test_format_package_spec_registry() {
        let pkg = make_locked_package("http", "^1.0");
        assert_eq!(format_package_spec(&pkg), "^1.0");
    }

    #[test]
    fn test_format_package_spec_path() {
        let pkg = LockedPackage {
            name: "local".to_string(),
            version: None,
            source: "path".to_string(),
            path: Some("../local".to_string()),
            git: None,
            branch: None,
            tag: None,
            rev: None,
            features: Vec::new(),
            checksum: None,
            section: None,
        };
        assert_eq!(format_package_spec(&pkg), "path:../local");
    }

    #[test]
    fn test_format_package_spec_git() {
        let pkg = LockedPackage {
            name: "remote".to_string(),
            version: None,
            source: "git".to_string(),
            path: None,
            git: Some("https://github.com/example/lib".to_string()),
            branch: Some("main".to_string()),
            tag: None,
            rev: None,
            features: Vec::new(),
            checksum: None,
            section: None,
        };
        assert_eq!(
            format_package_spec(&pkg),
            "git:https://github.com/example/lib#main"
        );
    }
}
