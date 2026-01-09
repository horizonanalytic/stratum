//! GitHub-based package registry for Stratum packages.
//!
//! This module provides functionality to:
//! - Parse GitHub package specifications (e.g., `github:user/repo@v1.0.0`)
//! - Fetch packages from GitHub releases
//! - Cache packages locally
//! - Validate package integrity with checksums

use crate::{Manifest, MANIFEST_FILE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during registry operations.
#[derive(Error, Debug)]
pub enum RegistryError {
    /// Invalid package specification format.
    #[error("invalid package specification '{spec}': {reason}")]
    InvalidSpec { spec: String, reason: String },

    /// Package not found on GitHub.
    #[error("package '{owner}/{repo}' not found on GitHub")]
    PackageNotFound { owner: String, repo: String },

    /// Release not found for the specified version.
    #[error("release '{version}' not found for '{owner}/{repo}'")]
    ReleaseNotFound {
        owner: String,
        repo: String,
        version: String,
    },

    /// Asset not found in the release.
    #[error("package asset not found in release '{version}' for '{owner}/{repo}'")]
    AssetNotFound {
        owner: String,
        repo: String,
        version: String,
    },

    /// Network error during fetch.
    #[error("network error: {0}")]
    Network(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Invalid package contents.
    #[error("invalid package: {0}")]
    InvalidPackage(String),

    /// Checksum verification failed.
    #[error("checksum verification failed for '{package}': expected {expected}, got {actual}")]
    ChecksumMismatch {
        package: String,
        expected: String,
        actual: String,
    },

    /// JSON parsing error.
    #[error("JSON error: {0}")]
    Json(String),

    /// TOML parsing error.
    #[error("TOML error: {0}")]
    Toml(String),

    /// GitHub API rate limit exceeded.
    #[error("GitHub API rate limit exceeded. Try again later or provide a GITHUB_TOKEN")]
    RateLimitExceeded,
}

/// A parsed GitHub package specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GitHubPackage {
    /// GitHub repository owner (user or organization).
    pub owner: String,
    /// Repository name.
    pub repo: String,
    /// Version specification (tag name, branch, or "latest").
    pub version: Option<String>,
}

impl GitHubPackage {
    /// Parse a GitHub package specification.
    ///
    /// Supported formats:
    /// - `github:user/repo` - Latest release
    /// - `github:user/repo@v1.0.0` - Specific tag
    /// - `github:user/repo@1.0.0` - Tag with or without 'v' prefix
    ///
    /// # Errors
    ///
    /// Returns an error if the specification format is invalid.
    pub fn parse(spec: &str) -> Result<Self, RegistryError> {
        // Check for github: prefix
        let rest = spec.strip_prefix("github:").ok_or_else(|| RegistryError::InvalidSpec {
            spec: spec.to_string(),
            reason: "must start with 'github:'".to_string(),
        })?;

        // Split on @ for version
        let (repo_part, version) = if let Some(at_pos) = rest.rfind('@') {
            let repo = &rest[..at_pos];
            let ver = &rest[at_pos + 1..];
            if ver.is_empty() {
                return Err(RegistryError::InvalidSpec {
                    spec: spec.to_string(),
                    reason: "version after '@' cannot be empty".to_string(),
                });
            }
            (repo, Some(ver.to_string()))
        } else {
            (rest, None)
        };

        // Parse owner/repo
        let parts: Vec<&str> = repo_part.split('/').collect();
        if parts.len() != 2 {
            return Err(RegistryError::InvalidSpec {
                spec: spec.to_string(),
                reason: "expected format 'owner/repo'".to_string(),
            });
        }

        let owner = parts[0].to_string();
        let repo = parts[1].to_string();

        // Validate owner and repo names
        if owner.is_empty() || !is_valid_github_name(&owner) {
            return Err(RegistryError::InvalidSpec {
                spec: spec.to_string(),
                reason: format!("invalid owner name '{owner}'"),
            });
        }
        if repo.is_empty() || !is_valid_github_name(&repo) {
            return Err(RegistryError::InvalidSpec {
                spec: spec.to_string(),
                reason: format!("invalid repository name '{repo}'"),
            });
        }

        Ok(Self { owner, repo, version })
    }

    /// Get the package name (derived from repo name).
    #[must_use]
    pub fn package_name(&self) -> &str {
        &self.repo
    }

    /// Convert to a git URL for the repository.
    #[must_use]
    pub fn git_url(&self) -> String {
        format!("https://github.com/{}/{}", self.owner, self.repo)
    }

    /// Get the API URL for releases.
    #[must_use]
    pub fn releases_api_url(&self) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/releases",
            self.owner, self.repo
        )
    }

    /// Get the API URL for a specific release by tag.
    #[must_use]
    pub fn release_by_tag_url(&self, tag: &str) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/releases/tags/{}",
            self.owner, self.repo, tag
        )
    }

    /// Get the API URL for the latest release.
    #[must_use]
    pub fn latest_release_url(&self) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            self.owner, self.repo
        )
    }
}

impl std::fmt::Display for GitHubPackage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "github:{}/{}", self.owner, self.repo)?;
        if let Some(ref v) = self.version {
            write!(f, "@{v}")?;
        }
        Ok(())
    }
}

/// Check if a string is a valid GitHub username or repo name.
fn is_valid_github_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 100 {
        return false;
    }
    // GitHub names can contain alphanumeric characters, hyphens, and underscores
    // They cannot start or end with a hyphen
    if name.starts_with('-') || name.ends_with('-') {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// GitHub release information from the API.
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    /// Release tag name.
    pub tag_name: String,
    /// Release name/title.
    pub name: Option<String>,
    /// Whether this is a prerelease.
    pub prerelease: bool,
    /// Whether this is a draft.
    pub draft: bool,
    /// Release assets (downloadable files).
    pub assets: Vec<GitHubAsset>,
    /// URL to the tarball of the source code.
    pub tarball_url: String,
    /// URL to the zipball of the source code.
    pub zipball_url: String,
}

/// GitHub release asset information.
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    /// Asset name.
    pub name: String,
    /// Content type (MIME type).
    pub content_type: String,
    /// Download URL.
    pub browser_download_url: String,
    /// File size in bytes.
    pub size: u64,
}

/// Package index entry for tracking installed packages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageIndexEntry {
    /// Package name.
    pub name: String,
    /// GitHub owner.
    pub owner: String,
    /// GitHub repo.
    pub repo: String,
    /// Installed version (tag).
    pub version: String,
    /// SHA256 checksum of the package tarball.
    pub checksum: String,
    /// Installation timestamp.
    pub installed_at: String,
}

/// The package index file tracking all installed packages.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageIndex {
    /// Version of the index format.
    pub version: u32,
    /// Map of package name to entry.
    pub packages: HashMap<String, PackageIndexEntry>,
}

impl PackageIndex {
    /// Current index format version.
    pub const CURRENT_VERSION: u32 = 1;

    /// Create a new empty index.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            packages: HashMap::new(),
        }
    }

    /// Load the index from a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, RegistryError> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let content = fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| RegistryError::Toml(e.to_string()))
    }

    /// Save the index to a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), RegistryError> {
        let content = toml::to_string_pretty(self).map_err(|e| RegistryError::Toml(e.to_string()))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    /// Get a package entry by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&PackageIndexEntry> {
        self.packages.get(name)
    }

    /// Add or update a package entry.
    pub fn insert(&mut self, entry: PackageIndexEntry) {
        self.packages.insert(entry.name.clone(), entry);
    }

    /// Remove a package entry.
    pub fn remove(&mut self, name: &str) -> Option<PackageIndexEntry> {
        self.packages.remove(name)
    }
}

/// Configuration for the package registry client.
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Directory to cache downloaded packages.
    pub cache_dir: PathBuf,
    /// Optional GitHub token for API authentication.
    pub github_token: Option<String>,
    /// User agent for HTTP requests.
    pub user_agent: String,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        // Use platform-appropriate cache directory
        let cache_dir = dirs_cache_dir().join("stratum").join("packages");
        Self {
            cache_dir,
            github_token: std::env::var("GITHUB_TOKEN").ok(),
            user_agent: format!("stratum/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

/// Get the platform-appropriate cache directory.
fn dirs_cache_dir() -> PathBuf {
    // Try XDG_CACHE_HOME first, then fall back to ~/.cache or platform default
    if let Ok(cache) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(cache);
    }
    if let Some(home) = std::env::var("HOME").ok().or_else(|| std::env::var("USERPROFILE").ok()) {
        #[cfg(target_os = "macos")]
        {
            return PathBuf::from(&home).join("Library").join("Caches");
        }
        #[cfg(target_os = "windows")]
        {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                return PathBuf::from(local_app_data);
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            return PathBuf::from(home).join(".cache");
        }
    }
    // Ultimate fallback
    PathBuf::from(".cache")
}

/// Client for interacting with the GitHub-based package registry.
pub struct RegistryClient {
    config: RegistryConfig,
    http_client: reqwest::blocking::Client,
}

impl RegistryClient {
    /// Create a new registry client with default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn new() -> Result<Self, RegistryError> {
        Self::with_config(RegistryConfig::default())
    }

    /// Create a new registry client with custom configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn with_config(config: RegistryConfig) -> Result<Self, RegistryError> {
        let http_client = reqwest::blocking::Client::builder()
            .user_agent(&config.user_agent)
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        Ok(Self { config, http_client })
    }

    /// Get the cache directory for packages.
    #[must_use]
    pub fn cache_dir(&self) -> &Path {
        &self.config.cache_dir
    }

    /// Get the package index file path.
    #[must_use]
    pub fn index_path(&self) -> PathBuf {
        self.config.cache_dir.join("index.toml")
    }

    /// Load the package index.
    ///
    /// # Errors
    ///
    /// Returns an error if the index cannot be loaded.
    pub fn load_index(&self) -> Result<PackageIndex, RegistryError> {
        PackageIndex::load(&self.index_path())
    }

    /// Save the package index.
    ///
    /// # Errors
    ///
    /// Returns an error if the index cannot be saved.
    pub fn save_index(&self, index: &PackageIndex) -> Result<(), RegistryError> {
        index.save(&self.index_path())
    }

    /// Build HTTP request with optional authentication.
    fn build_request(&self, url: &str) -> reqwest::blocking::RequestBuilder {
        let mut req = self.http_client.get(url);
        req = req.header("Accept", "application/vnd.github.v3+json");
        if let Some(ref token) = self.config.github_token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        req
    }

    /// Fetch release information from GitHub.
    ///
    /// # Errors
    ///
    /// Returns an error if the release cannot be fetched.
    pub fn fetch_release(&self, pkg: &GitHubPackage) -> Result<GitHubRelease, RegistryError> {
        let url = if let Some(ref version) = pkg.version {
            // Try with and without 'v' prefix
            pkg.release_by_tag_url(version)
        } else {
            pkg.latest_release_url()
        };

        let response = self
            .build_request(&url)
            .send()
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        // Handle rate limiting
        if response.status() == reqwest::StatusCode::FORBIDDEN {
            if response
                .headers()
                .get("X-RateLimit-Remaining")
                .map_or(false, |v| v == "0")
            {
                return Err(RegistryError::RateLimitExceeded);
            }
        }

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            // If version was specified without 'v' prefix, try with it
            if let Some(ref version) = pkg.version {
                if !version.starts_with('v') {
                    let alt_url = pkg.release_by_tag_url(&format!("v{version}"));
                    let alt_response = self
                        .build_request(&alt_url)
                        .send()
                        .map_err(|e| RegistryError::Network(e.to_string()))?;

                    if alt_response.status().is_success() {
                        let release: GitHubRelease = alt_response
                            .json()
                            .map_err(|e| RegistryError::Json(e.to_string()))?;
                        return Ok(release);
                    }
                }
            }

            return Err(if pkg.version.is_some() {
                RegistryError::ReleaseNotFound {
                    owner: pkg.owner.clone(),
                    repo: pkg.repo.clone(),
                    version: pkg.version.clone().unwrap_or_default(),
                }
            } else {
                RegistryError::PackageNotFound {
                    owner: pkg.owner.clone(),
                    repo: pkg.repo.clone(),
                }
            });
        }

        if !response.status().is_success() {
            return Err(RegistryError::Network(format!(
                "GitHub API returned status {}",
                response.status()
            )));
        }

        let release: GitHubRelease = response
            .json()
            .map_err(|e| RegistryError::Json(e.to_string()))?;

        Ok(release)
    }

    /// Find the package tarball asset in a release.
    ///
    /// Looks for assets with names like `{repo}.tar.gz` or `{repo}-{version}.tar.gz`.
    fn find_package_asset<'a>(
        &self,
        release: &'a GitHubRelease,
        pkg: &GitHubPackage,
    ) -> Option<&'a GitHubAsset> {
        let repo_lower = pkg.repo.to_lowercase();
        let tag_lower = release.tag_name.to_lowercase();
        let version_without_v = tag_lower.strip_prefix('v').unwrap_or(&tag_lower);

        // Priority order for asset matching
        let patterns = [
            format!("{repo_lower}.tar.gz"),
            format!("{repo_lower}-{}.tar.gz", version_without_v),
            format!("{repo_lower}-{tag_lower}.tar.gz"),
            format!("{}-package.tar.gz", repo_lower),
        ];

        for pattern in &patterns {
            if let Some(asset) = release
                .assets
                .iter()
                .find(|a| a.name.to_lowercase() == *pattern)
            {
                return Some(asset);
            }
        }

        // Fall back to any .tar.gz file
        release
            .assets
            .iter()
            .find(|a| a.name.ends_with(".tar.gz"))
    }

    /// Download a file from a URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the download fails.
    fn download(&self, url: &str) -> Result<Vec<u8>, RegistryError> {
        let mut req = self.http_client.get(url);
        if let Some(ref token) = self.config.github_token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        req = req.header("Accept", "application/octet-stream");

        let response = req
            .send()
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::Network(format!(
                "download failed with status {}",
                response.status()
            )));
        }

        response
            .bytes()
            .map(|b| b.to_vec())
            .map_err(|e| RegistryError::Network(e.to_string()))
    }

    /// Calculate SHA256 checksum of data.
    fn calculate_checksum(data: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// Fetch and cache a package from GitHub.
    ///
    /// # Errors
    ///
    /// Returns an error if the package cannot be fetched or cached.
    pub fn fetch_package(&self, pkg: &GitHubPackage) -> Result<FetchedPackage, RegistryError> {
        // Fetch release information
        let release = self.fetch_release(pkg)?;

        // Determine download URL (prefer package asset, fall back to source tarball)
        let (download_url, is_source_tarball) =
            if let Some(asset) = self.find_package_asset(&release, pkg) {
                (asset.browser_download_url.clone(), false)
            } else {
                // Use GitHub's source tarball
                (release.tarball_url.clone(), true)
            };

        // Download the package
        let data = self.download(&download_url)?;
        let checksum = Self::calculate_checksum(&data);

        // Create cache directory structure
        let cache_subdir = self
            .config
            .cache_dir
            .join(&pkg.owner)
            .join(&pkg.repo)
            .join(&release.tag_name);
        fs::create_dir_all(&cache_subdir)?;

        // Save the tarball
        let tarball_path = cache_subdir.join("package.tar.gz");
        fs::write(&tarball_path, &data)?;

        // Extract the package
        let extract_dir = cache_subdir.join("src");
        extract_tarball(&tarball_path, &extract_dir, is_source_tarball)?;

        // Validate the package (check for stratum.toml)
        let manifest_path = find_manifest_in_extracted(&extract_dir)?;
        let manifest = Manifest::from_path(&manifest_path)
            .map_err(|e| RegistryError::InvalidPackage(format!("invalid manifest: {e}")))?;

        // Get the package name before moving manifest
        let package_name = manifest.package.name.clone();

        // Update the package index
        let mut index = self.load_index()?;
        index.insert(PackageIndexEntry {
            name: package_name.clone(),
            owner: pkg.owner.clone(),
            repo: pkg.repo.clone(),
            version: release.tag_name.clone(),
            checksum: checksum.clone(),
            installed_at: chrono::Utc::now().to_rfc3339(),
        });
        self.save_index(&index)?;

        Ok(FetchedPackage {
            name: package_name,
            version: release.tag_name,
            checksum,
            path: extract_dir,
            manifest,
        })
    }

    /// Check if a package is cached.
    #[must_use]
    pub fn is_cached(&self, pkg: &GitHubPackage, version: &str) -> bool {
        let cache_subdir = self
            .config
            .cache_dir
            .join(&pkg.owner)
            .join(&pkg.repo)
            .join(version);
        cache_subdir.join("src").exists()
    }

    /// Get the cached package path if it exists.
    #[must_use]
    pub fn cached_path(&self, pkg: &GitHubPackage, version: &str) -> Option<PathBuf> {
        let cache_subdir = self
            .config
            .cache_dir
            .join(&pkg.owner)
            .join(&pkg.repo)
            .join(version)
            .join("src");
        if cache_subdir.exists() {
            Some(cache_subdir)
        } else {
            None
        }
    }
}

/// A successfully fetched package.
#[derive(Debug)]
pub struct FetchedPackage {
    /// Package name from manifest.
    pub name: String,
    /// Version (tag) that was fetched.
    pub version: String,
    /// SHA256 checksum of the tarball.
    pub checksum: String,
    /// Path to the extracted package.
    pub path: PathBuf,
    /// The package manifest.
    pub manifest: Manifest,
}

/// Extract a tarball to a directory.
fn extract_tarball(
    tarball_path: &Path,
    dest_dir: &Path,
    is_github_source_tarball: bool,
) -> Result<(), RegistryError> {
    use flate2::read::GzDecoder;
    use std::fs::File;

    // Remove destination if it exists
    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir)?;
    }
    fs::create_dir_all(dest_dir)?;

    let file = File::open(tarball_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    if is_github_source_tarball {
        // GitHub source tarballs have a top-level directory like "repo-tag/"
        // We need to strip this prefix
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;

            // Skip the first component (the top-level directory)
            let components: Vec<_> = path.components().skip(1).collect();
            if components.is_empty() {
                continue;
            }

            let dest_path: PathBuf = components.iter().collect();
            let full_dest = dest_dir.join(&dest_path);

            if entry.header().entry_type().is_dir() {
                fs::create_dir_all(&full_dest)?;
            } else {
                if let Some(parent) = full_dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut file = File::create(&full_dest)?;
                io::copy(&mut entry, &mut file)?;
            }
        }
    } else {
        // Regular tarball - extract directly
        archive.unpack(dest_dir)?;
    }

    Ok(())
}

/// Find the manifest file in an extracted package directory.
fn find_manifest_in_extracted(dir: &Path) -> Result<PathBuf, RegistryError> {
    // Check direct path first
    let direct = dir.join(MANIFEST_FILE);
    if direct.exists() {
        return Ok(direct);
    }

    // Check subdirectories (in case of nested structure)
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let nested = path.join(MANIFEST_FILE);
                if nested.exists() {
                    return Ok(nested);
                }
            }
        }
    }

    Err(RegistryError::InvalidPackage(format!(
        "no {} found in package",
        MANIFEST_FILE
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_package_basic() {
        let pkg = GitHubPackage::parse("github:user/repo").unwrap();
        assert_eq!(pkg.owner, "user");
        assert_eq!(pkg.repo, "repo");
        assert_eq!(pkg.version, None);
    }

    #[test]
    fn test_parse_github_package_with_version() {
        let pkg = GitHubPackage::parse("github:user/repo@v1.0.0").unwrap();
        assert_eq!(pkg.owner, "user");
        assert_eq!(pkg.repo, "repo");
        assert_eq!(pkg.version, Some("v1.0.0".to_string()));
    }

    #[test]
    fn test_parse_github_package_version_no_v() {
        let pkg = GitHubPackage::parse("github:user/repo@1.0.0").unwrap();
        assert_eq!(pkg.version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_parse_github_package_org_name() {
        let pkg = GitHubPackage::parse("github:my-org/my-repo").unwrap();
        assert_eq!(pkg.owner, "my-org");
        assert_eq!(pkg.repo, "my-repo");
    }

    #[test]
    fn test_parse_github_package_underscore() {
        let pkg = GitHubPackage::parse("github:user_name/repo_name").unwrap();
        assert_eq!(pkg.owner, "user_name");
        assert_eq!(pkg.repo, "repo_name");
    }

    #[test]
    fn test_parse_github_package_invalid_no_prefix() {
        let result = GitHubPackage::parse("user/repo");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_github_package_invalid_no_slash() {
        let result = GitHubPackage::parse("github:userrepo");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_github_package_invalid_empty_owner() {
        let result = GitHubPackage::parse("github:/repo");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_github_package_invalid_empty_repo() {
        let result = GitHubPackage::parse("github:user/");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_github_package_invalid_empty_version() {
        let result = GitHubPackage::parse("github:user/repo@");
        assert!(result.is_err());
    }

    #[test]
    fn test_github_package_display() {
        let pkg = GitHubPackage {
            owner: "user".to_string(),
            repo: "repo".to_string(),
            version: Some("v1.0.0".to_string()),
        };
        assert_eq!(pkg.to_string(), "github:user/repo@v1.0.0");
    }

    #[test]
    fn test_github_package_git_url() {
        let pkg = GitHubPackage::parse("github:owner/repo").unwrap();
        assert_eq!(pkg.git_url(), "https://github.com/owner/repo");
    }

    #[test]
    fn test_github_package_api_urls() {
        let pkg = GitHubPackage::parse("github:owner/repo").unwrap();
        assert_eq!(
            pkg.releases_api_url(),
            "https://api.github.com/repos/owner/repo/releases"
        );
        assert_eq!(
            pkg.latest_release_url(),
            "https://api.github.com/repos/owner/repo/releases/latest"
        );
        assert_eq!(
            pkg.release_by_tag_url("v1.0.0"),
            "https://api.github.com/repos/owner/repo/releases/tags/v1.0.0"
        );
    }

    #[test]
    fn test_is_valid_github_name() {
        assert!(is_valid_github_name("user"));
        assert!(is_valid_github_name("user-name"));
        assert!(is_valid_github_name("user_name"));
        assert!(is_valid_github_name("user123"));
        assert!(!is_valid_github_name("-user"));
        assert!(!is_valid_github_name("user-"));
        assert!(!is_valid_github_name(""));
        assert!(!is_valid_github_name("user.name"));
    }

    #[test]
    fn test_package_index_new() {
        let index = PackageIndex::new();
        assert_eq!(index.version, PackageIndex::CURRENT_VERSION);
        assert!(index.packages.is_empty());
    }

    #[test]
    fn test_package_index_operations() {
        let mut index = PackageIndex::new();

        let entry = PackageIndexEntry {
            name: "test-pkg".to_string(),
            owner: "user".to_string(),
            repo: "test-pkg".to_string(),
            version: "v1.0.0".to_string(),
            checksum: "abc123".to_string(),
            installed_at: "2024-01-01T00:00:00Z".to_string(),
        };

        index.insert(entry.clone());
        assert!(index.get("test-pkg").is_some());
        assert_eq!(index.get("test-pkg").unwrap().version, "v1.0.0");

        let removed = index.remove("test-pkg");
        assert!(removed.is_some());
        assert!(index.get("test-pkg").is_none());
    }
}
