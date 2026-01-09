//! Implementation of the `stratum publish` command.
//!
//! Publishes a Stratum package to GitHub Releases.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;
use stratum_pkg::{Manifest, PackageLayout, MANIFEST_FILE};

/// Options for the publish command.
#[derive(Debug)]
pub struct PublishOptions {
    /// Version tag to publish. If None, uses version from manifest.
    pub tag: Option<String>,
    /// Dry run mode - don't actually publish.
    pub dry_run: bool,
    /// Allow publishing with uncommitted changes.
    pub allow_dirty: bool,
    /// Target repository (owner/repo). If None, detected from git remote.
    pub target: Option<String>,
}

/// Result of package validation.
#[derive(Debug)]
struct ValidationResult {
    /// Package name from manifest.
    name: String,
    /// Version from manifest.
    version: String,
    /// Detected GitHub repository (owner/repo).
    repository: String,
    /// Path to the package root.
    package_root: std::path::PathBuf,
}

/// Publish a package to GitHub Releases.
pub fn publish_package(options: PublishOptions) -> Result<()> {
    // Validate the package
    let validation = validate_package(&options)?;

    // Determine the tag to use
    let tag = options
        .tag
        .unwrap_or_else(|| format!("v{}", validation.version));

    // Create the package tarball
    let tarball_name = format!("{}-{}.tar.gz", validation.name, validation.version);
    let tarball_path = validation.package_root.join("target").join(&tarball_name);

    println!("Packaging {}...", validation.name);
    create_package_tarball(&validation.package_root, &tarball_path)?;

    if options.dry_run {
        println!("\n[Dry run] Would publish:");
        println!("  Package: {}", validation.name);
        println!("  Version: {}", validation.version);
        println!("  Tag: {tag}");
        println!("  Repository: {}", validation.repository);
        println!("  Tarball: {}", tarball_path.display());

        // Clean up tarball
        if tarball_path.exists() {
            std::fs::remove_file(&tarball_path)?;
        }

        println!("\nDry run complete. No changes were made.");
        return Ok(());
    }

    // Check if gh CLI is available
    check_gh_cli()?;

    // Create GitHub release
    println!("Creating GitHub release {}...", tag);
    create_github_release(&validation.repository, &tag, &tarball_path, &validation.name)?;

    // Clean up
    std::fs::remove_file(&tarball_path)?;

    println!("\nPublished {} v{} to GitHub!", validation.name, validation.version);
    println!(
        "View at: https://github.com/{}/releases/tag/{}",
        validation.repository, tag
    );

    Ok(())
}

/// Validate the package for publishing.
fn validate_package(options: &PublishOptions) -> Result<ValidationResult> {
    let manifest_path = Path::new(MANIFEST_FILE);

    // Check if manifest exists
    if !manifest_path.exists() {
        return Err(anyhow::anyhow!(
            "No {} found in current directory. Run `stratum init` first.",
            MANIFEST_FILE
        ));
    }

    // Load manifest
    let manifest = Manifest::from_path(manifest_path).context("Failed to read manifest")?;

    // Validate required fields
    if manifest.package.name.is_empty() {
        return Err(anyhow::anyhow!("Package name is required in stratum.toml"));
    }

    if manifest.package.version.is_empty() {
        return Err(anyhow::anyhow!("Package version is required in stratum.toml"));
    }

    // Validate version is valid semver
    semver::Version::parse(&manifest.package.version).map_err(|e| {
        anyhow::anyhow!(
            "Invalid version '{}': {}. Use semantic versioning (e.g., 1.0.0)",
            manifest.package.version,
            e
        )
    })?;

    // Check for source files
    let layout = PackageLayout::discover(Path::new("."))
        .map_err(|e| anyhow::anyhow!("Invalid package structure: {}", e))?;

    if !layout.has_lib() && !layout.has_bin() {
        return Err(anyhow::anyhow!(
            "Package must have either src/lib.strat or src/main.strat"
        ));
    }

    // Check for uncommitted changes
    if !options.allow_dirty {
        check_git_clean()?;
    }

    // Determine target repository
    let repository = if let Some(ref target) = options.target {
        validate_repo_format(target)?;
        target.clone()
    } else if let Some(ref repo) = manifest.package.repository {
        // Extract owner/repo from URL
        extract_github_repo(repo)?
    } else {
        // Try to detect from git remote
        detect_github_remote()?
    };

    let package_root = std::env::current_dir()?;

    Ok(ValidationResult {
        name: manifest.package.name,
        version: manifest.package.version,
        repository,
        package_root,
    })
}

/// Validate repository format (owner/repo).
fn validate_repo_format(repo: &str) -> Result<()> {
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(anyhow::anyhow!(
            "Invalid repository format '{}'. Expected 'owner/repo'",
            repo
        ));
    }
    Ok(())
}

/// Extract owner/repo from a GitHub URL.
fn extract_github_repo(url: &str) -> Result<String> {
    // Handle various GitHub URL formats:
    // - https://github.com/owner/repo
    // - https://github.com/owner/repo.git
    // - git@github.com:owner/repo.git

    let url = url.trim();

    if let Some(rest) = url.strip_prefix("https://github.com/") {
        let repo = rest.trim_end_matches(".git");
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() >= 2 {
            return Ok(format!("{}/{}", parts[0], parts[1]));
        }
    }

    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let repo = rest.trim_end_matches(".git");
        return Ok(repo.to_string());
    }

    Err(anyhow::anyhow!(
        "Could not extract GitHub repository from URL: {}",
        url
    ))
}

/// Detect GitHub repository from git remote.
fn detect_github_remote() -> Result<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .context("Failed to run git command")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "No git remote 'origin' found. Use --target to specify the repository."
        ));
    }

    let url = String::from_utf8_lossy(&output.stdout);
    extract_github_repo(url.trim())
}

/// Check if the git working directory is clean.
fn check_git_clean() -> Result<()> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to run git command")?;

    if !output.status.success() {
        // Not a git repo, that's fine
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "Working directory has uncommitted changes.\n\
             Commit your changes or use --allow-dirty to publish anyway."
        ));
    }

    Ok(())
}

/// Check if the GitHub CLI (gh) is available and authenticated.
fn check_gh_cli() -> Result<()> {
    // Check if gh is installed
    let output = Command::new("gh")
        .args(["--version"])
        .output()
        .context("GitHub CLI (gh) is not installed. Install it from https://cli.github.com/")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "GitHub CLI (gh) is not working properly"
        ));
    }

    // Check if authenticated
    let auth_output = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .context("Failed to check GitHub CLI authentication")?;

    if !auth_output.status.success() {
        return Err(anyhow::anyhow!(
            "GitHub CLI is not authenticated. Run `gh auth login` first."
        ));
    }

    Ok(())
}

/// Create a tarball of the package.
fn create_package_tarball(package_root: &Path, tarball_path: &Path) -> Result<()> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::fs::File;

    // Create target directory if needed
    if let Some(parent) = tarball_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let tar_file = File::create(tarball_path)?;
    let encoder = GzEncoder::new(tar_file, Compression::default());
    let mut builder = tar::Builder::new(encoder);

    // Add package files
    add_package_files(&mut builder, package_root)?;

    builder.finish()?;
    Ok(())
}

/// Add package files to the tarball.
fn add_package_files<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    package_root: &Path,
) -> Result<()> {
    // Files and directories to include
    let include = [
        MANIFEST_FILE,
        "src",
        "tests",
        "examples",
        "benches",
        "README.md",
        "README",
        "LICENSE",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "CHANGELOG.md",
    ];

    // Directories to exclude
    let exclude = ["target", ".git", "node_modules", ".DS_Store"];

    for entry in include {
        let path = package_root.join(entry);
        if path.exists() {
            if path.is_file() {
                builder
                    .append_path_with_name(&path, entry)
                    .with_context(|| format!("Failed to add {entry} to tarball"))?;
            } else if path.is_dir() {
                add_directory_recursive(builder, &path, entry, &exclude)?;
            }
        }
    }

    Ok(())
}

/// Recursively add a directory to the tarball.
fn add_directory_recursive<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    dir: &Path,
    prefix: &str,
    exclude: &[&str],
) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip excluded directories/files
        if exclude.iter().any(|e| *e == name_str) {
            continue;
        }

        // Skip hidden files
        if name_str.starts_with('.') {
            continue;
        }

        let archive_path = format!("{prefix}/{name_str}");

        if path.is_file() {
            builder
                .append_path_with_name(&path, &archive_path)
                .with_context(|| format!("Failed to add {} to tarball", path.display()))?;
        } else if path.is_dir() {
            add_directory_recursive(builder, &path, &archive_path, exclude)?;
        }
    }

    Ok(())
}

/// Create a GitHub release using the gh CLI.
fn create_github_release(
    repository: &str,
    tag: &str,
    tarball_path: &Path,
    package_name: &str,
) -> Result<()> {
    // Check if tag already exists
    let tag_check = Command::new("gh")
        .args([
            "release",
            "view",
            tag,
            "--repo",
            repository,
        ])
        .output();

    if let Ok(output) = tag_check {
        if output.status.success() {
            return Err(anyhow::anyhow!(
                "Release {} already exists in {}. Use a different version.",
                tag,
                repository
            ));
        }
    }

    // Create the release
    let _tarball_name = tarball_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("package.tar.gz");

    let output = Command::new("gh")
        .args([
            "release",
            "create",
            tag,
            "--repo",
            repository,
            "--title",
            &format!("{} {}", package_name, tag),
            "--notes",
            &format!("Release {} of {}", tag, package_name),
            tarball_path.to_str().unwrap_or(""),
        ])
        .output()
        .context("Failed to create GitHub release")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to create release: {}", stderr));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_repo_format_valid() {
        assert!(validate_repo_format("owner/repo").is_ok());
        assert!(validate_repo_format("my-org/my-repo").is_ok());
    }

    #[test]
    fn test_validate_repo_format_invalid() {
        assert!(validate_repo_format("owner").is_err());
        assert!(validate_repo_format("owner/").is_err());
        assert!(validate_repo_format("/repo").is_err());
        assert!(validate_repo_format("owner/repo/extra").is_err());
    }

    #[test]
    fn test_extract_github_repo_https() {
        let repo = extract_github_repo("https://github.com/user/repo").unwrap();
        assert_eq!(repo, "user/repo");
    }

    #[test]
    fn test_extract_github_repo_https_with_git() {
        let repo = extract_github_repo("https://github.com/user/repo.git").unwrap();
        assert_eq!(repo, "user/repo");
    }

    #[test]
    fn test_extract_github_repo_ssh() {
        let repo = extract_github_repo("git@github.com:user/repo.git").unwrap();
        assert_eq!(repo, "user/repo");
    }

    #[test]
    fn test_extract_github_repo_invalid() {
        assert!(extract_github_repo("https://gitlab.com/user/repo").is_err());
        assert!(extract_github_repo("invalid").is_err());
    }
}
