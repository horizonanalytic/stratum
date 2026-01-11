//! Implementation of the `stratum self` command for self-management.
//!
//! Provides commands for updating and uninstalling Stratum itself.

use anyhow::{bail, Context, Result};
use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;

/// Installation metadata stored in ~/.stratum/.install-meta
#[derive(Debug, Default, Clone)]
pub struct InstallMeta {
    pub version: String,
    pub tier: String,
    pub target: String,
    pub installed_at: String,
    pub installer_version: String,
}

impl InstallMeta {
    /// Parse installation metadata from file contents
    fn parse(content: &str) -> Self {
        let mut meta = InstallMeta::default();
        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    "version" => meta.version = value.trim().to_string(),
                    "tier" => meta.tier = value.trim().to_string(),
                    "target" => meta.target = value.trim().to_string(),
                    "installed_at" => meta.installed_at = value.trim().to_string(),
                    "installer_version" => meta.installer_version = value.trim().to_string(),
                    _ => {}
                }
            }
        }
        meta
    }

    /// Write metadata to file
    fn write(&self, path: &PathBuf) -> Result<()> {
        let content = format!(
            "version={}\ntier={}\ntarget={}\ninstalled_at={}\ninstaller_version={}\n",
            self.version, self.tier, self.target, self.installed_at, self.installer_version
        );
        fs::write(path, content).context("Failed to write installation metadata")?;
        Ok(())
    }
}

/// Options for the update command
pub struct UpdateOptions {
    /// Force update even if already on latest version
    pub force: bool,
    /// Target tier for upgrade (None = keep current tier)
    pub tier: Option<String>,
    /// Skip confirmation prompt
    pub yes: bool,
    /// Perform a dry run without making changes
    pub dry_run: bool,
}

/// Options for the uninstall command
pub struct UninstallOptions {
    /// Remove all configuration and user data
    pub purge: bool,
    /// Skip confirmation prompt
    pub yes: bool,
}

/// Options for installing a specific version
pub struct InstallVersionOptions {
    /// Version to install (e.g., "1.0.0" or "v1.0.0")
    pub version: String,
    /// Installation tier (core, data, gui, full)
    pub tier: String,
    /// Skip confirmation prompt
    pub yes: bool,
    /// Make this version active after installation
    pub activate: bool,
}

/// Get the Stratum home directory
pub fn get_stratum_home() -> Result<PathBuf> {
    // Check environment variable first
    if let Ok(home) = std::env::var("STRATUM_HOME") {
        return Ok(PathBuf::from(home));
    }

    // Default to ~/.stratum
    let home_dir = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home_dir.join(".stratum"))
}

/// Read installation metadata
pub fn read_install_meta() -> Result<Option<InstallMeta>> {
    let stratum_home = get_stratum_home()?;
    let meta_path = stratum_home.join(".install-meta");

    if !meta_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&meta_path).context("Failed to read installation metadata")?;
    Ok(Some(InstallMeta::parse(&content)))
}

/// Detect the current installation method
#[derive(Debug, Clone, PartialEq)]
pub enum InstallMethod {
    /// Installed via install.sh script
    Script,
    /// Installed via Homebrew
    Homebrew,
    /// Installed via apt/dpkg
    Apt,
    /// Installed via rpm/dnf
    Rpm,
    /// Built from source (cargo install)
    Cargo,
    /// Unknown installation method
    Unknown,
}

impl std::fmt::Display for InstallMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallMethod::Script => write!(f, "install script"),
            InstallMethod::Homebrew => write!(f, "Homebrew"),
            InstallMethod::Apt => write!(f, "apt/dpkg"),
            InstallMethod::Rpm => write!(f, "rpm/dnf"),
            InstallMethod::Cargo => write!(f, "cargo install"),
            InstallMethod::Unknown => write!(f, "unknown method"),
        }
    }
}

/// Detect how Stratum was installed
pub fn detect_install_method() -> Result<InstallMethod> {
    let stratum_home = get_stratum_home()?;

    // Check for install.sh metadata
    if stratum_home.join(".install-meta").exists() {
        return Ok(InstallMethod::Script);
    }

    // Check for Homebrew
    if let Ok(output) = std::process::Command::new("brew")
        .args(["list", "stratum"])
        .output()
    {
        if output.status.success() {
            return Ok(InstallMethod::Homebrew);
        }
    }

    // Check for dpkg (Debian/Ubuntu)
    if let Ok(output) = std::process::Command::new("dpkg")
        .args(["-s", "stratum"])
        .output()
    {
        if output.status.success() {
            return Ok(InstallMethod::Apt);
        }
    }

    // Check for rpm (Fedora/RHEL)
    if let Ok(output) = std::process::Command::new("rpm")
        .args(["-q", "stratum"])
        .output()
    {
        if output.status.success() {
            return Ok(InstallMethod::Rpm);
        }
    }

    // Check if running from cargo install location
    if let Ok(exe_path) = std::env::current_exe() {
        if exe_path
            .to_string_lossy()
            .contains(".cargo/bin")
        {
            return Ok(InstallMethod::Cargo);
        }
    }

    Ok(InstallMethod::Unknown)
}

/// GitHub release information
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ReleaseInfo {
    pub version: String,
    pub tag_name: String,
    pub published_at: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
    pub size: u64,
}

/// Check for the latest version from GitHub releases
fn fetch_latest_release() -> Result<ReleaseInfo> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("stratum-cli")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get("https://api.github.com/repos/horizon-analytic/stratum/releases/latest")
        .send()
        .context("Failed to fetch release information")?;

    if !response.status().is_success() {
        bail!(
            "Failed to fetch release information: HTTP {}",
            response.status()
        );
    }

    let json: serde_json::Value = response.json().context("Failed to parse release JSON")?;

    let tag_name = json["tag_name"]
        .as_str()
        .context("Missing tag_name in release")?
        .to_string();

    let version = tag_name.trim_start_matches('v').to_string();

    let published_at = json["published_at"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let assets = json["assets"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|asset| {
                    Some(ReleaseAsset {
                        name: asset["name"].as_str()?.to_string(),
                        download_url: asset["browser_download_url"].as_str()?.to_string(),
                        size: asset["size"].as_u64().unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(ReleaseInfo {
        version,
        tag_name,
        published_at,
        assets,
    })
}

/// Detect the current platform target triple
fn detect_target() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let target = match (os, arch) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => {
            // Check for musl vs glibc
            if is_musl_libc() {
                "x86_64-unknown-linux-musl"
            } else {
                "x86_64-unknown-linux-gnu"
            }
        }
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        _ => bail!("Unsupported platform: {}-{}", os, arch),
    };

    Ok(target.to_string())
}

/// Check if the system uses musl libc
fn is_musl_libc() -> bool {
    // Check if /lib/ld-musl-* exists
    if let Ok(entries) = fs::read_dir("/lib") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("ld-musl") {
                    return true;
                }
            }
        }
    }

    // Check ldd output
    if let Ok(output) = std::process::Command::new("ldd").arg("--version").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains("musl") {
            return true;
        }
        // Also check stderr as some versions output there
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        if stderr_str.contains("musl") {
            return true;
        }
    }

    false
}

/// Download a file with progress reporting
fn download_file(url: &str, dest: &PathBuf) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("stratum-cli")
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client.get(url).send().context("Failed to download file")?;

    if !response.status().is_success() {
        bail!("Download failed: HTTP {}", response.status());
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut file = fs::File::create(dest).context("Failed to create destination file")?;

    let mut reader = BufReader::new(response);
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer).context("Failed to read response")?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])
            .context("Failed to write file")?;

        downloaded += bytes_read as u64;

        // Print progress
        if total_size > 0 {
            let percent = (downloaded as f64 / total_size as f64 * 100.0) as u32;
            print!("\r  Downloading... {}%", percent);
            std::io::stdout().flush().ok();
        }
    }

    println!("\r  Downloading... done    ");

    Ok(())
}

/// Extract a tar.gz archive
fn extract_tarball(archive_path: &PathBuf, dest_dir: &PathBuf) -> Result<()> {
    let file = fs::File::open(archive_path).context("Failed to open archive")?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    archive
        .unpack(dest_dir)
        .context("Failed to extract archive")?;

    Ok(())
}

/// Verify SHA256 checksum of a file
fn verify_checksum(file_path: &PathBuf, checksum_path: &PathBuf) -> Result<bool> {
    use sha2::{Digest, Sha256};

    // Read expected checksum
    let expected = fs::read_to_string(checksum_path)
        .context("Failed to read checksum file")?
        .split_whitespace()
        .next()
        .context("Invalid checksum file format")?
        .to_lowercase();

    // Calculate actual checksum
    let mut file = fs::File::open(file_path).context("Failed to open file for checksum")?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).context("Failed to read file for checksum")?;
    let actual = format!("{:x}", hasher.finalize());

    Ok(actual == expected)
}

/// Detect shell configuration files to clean up
fn get_shell_profile_paths() -> Vec<PathBuf> {
    let mut profiles = Vec::new();

    if let Some(home) = dirs::home_dir() {
        // Bash
        profiles.push(home.join(".bashrc"));
        profiles.push(home.join(".bash_profile"));
        profiles.push(home.join(".profile"));

        // Zsh
        profiles.push(home.join(".zshrc"));
        profiles.push(home.join(".zprofile"));

        // Fish
        profiles.push(home.join(".config/fish/conf.d/stratum.fish"));
        profiles.push(home.join(".config/fish/config.fish"));
    }

    profiles
}

/// Get shell completion file paths
fn get_completion_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(home) = dirs::home_dir() {
        // Bash
        paths.push(home.join(".local/share/bash-completion/completions/stratum"));

        // Zsh
        paths.push(home.join(".zfunc/_stratum"));

        // Fish
        paths.push(home.join(".config/fish/completions/stratum.fish"));
    }

    // System locations (may need sudo, but we try anyway)
    paths.push(PathBuf::from("/usr/local/share/zsh/site-functions/_stratum"));

    paths
}

/// Get paths to user data that should be removed with --purge
fn get_user_data_paths() -> Result<Vec<(PathBuf, &'static str)>> {
    let stratum_home = get_stratum_home()?;
    let mut paths = Vec::new();

    // Packages directory
    paths.push((stratum_home.join("packages"), "installed packages"));

    // Cache directory
    paths.push((stratum_home.join("cache"), "build cache"));

    // REPL history
    paths.push((stratum_home.join("history"), "REPL history"));
    paths.push((stratum_home.join(".repl_history"), "REPL history (legacy)"));

    // LSP cache
    paths.push((stratum_home.join("lsp-cache"), "LSP cache"));

    // Versions directory (multi-version installs)
    paths.push((stratum_home.join("versions"), "installed versions"));

    // Active version marker
    paths.push((stratum_home.join(".active-version"), "active version marker"));

    Ok(paths)
}

/// Get Workshop IDE application data paths (platform-specific)
fn get_workshop_ide_paths() -> Vec<(PathBuf, &'static str)> {
    let mut paths = Vec::new();

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            // Application Support
            paths.push((
                home.join("Library/Application Support/Stratum Workshop"),
                "Workshop IDE settings",
            ));

            // Preferences
            paths.push((
                home.join("Library/Preferences/dev.stratum-lang.workshop.plist"),
                "Workshop IDE preferences",
            ));

            // Caches
            paths.push((
                home.join("Library/Caches/dev.stratum-lang.workshop"),
                "Workshop IDE cache",
            ));

            // Logs
            paths.push((
                home.join("Library/Logs/Stratum Workshop"),
                "Workshop IDE logs",
            ));

            // Saved Application State
            paths.push((
                home.join("Library/Saved Application State/dev.stratum-lang.workshop.savedState"),
                "Workshop IDE saved state",
            ));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs::home_dir() {
            // XDG config directory
            let config_home = std::env::var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join(".config"));

            paths.push((
                config_home.join("stratum-workshop"),
                "Workshop IDE settings",
            ));

            // XDG data directory
            let data_home = std::env::var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join(".local/share"));

            paths.push((
                data_home.join("stratum-workshop"),
                "Workshop IDE data",
            ));

            // XDG cache directory
            let cache_home = std::env::var("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join(".cache"));

            paths.push((
                cache_home.join("stratum-workshop"),
                "Workshop IDE cache",
            ));

            // Desktop entry
            paths.push((
                data_home.join("applications/stratum-workshop.desktop"),
                "Workshop IDE desktop entry",
            ));

            // Icon
            paths.push((
                data_home.join("icons/hicolor/256x256/apps/stratum-workshop.png"),
                "Workshop IDE icon",
            ));
        }
    }

    paths
}

/// Remove a path (file or directory) with proper error handling
fn remove_path_with_warning(path: &PathBuf, description: &str, dry_run: bool) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    if dry_run {
        println!("  Would remove: {} ({})", path.display(), description);
        return Ok(true);
    }

    let result = if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };

    match result {
        Ok(()) => Ok(true),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                eprintln!(
                    "  Warning: Permission denied removing {} ({})",
                    path.display(),
                    description
                );
                eprintln!("           Try running with sudo or remove manually");
            } else {
                eprintln!(
                    "  Warning: Could not remove {} ({}): {}",
                    path.display(),
                    description,
                    e
                );
            }
            Ok(false)
        }
    }
}

/// Remove Stratum PATH entries and environment variables from shell profile files
fn remove_path_from_profiles(dry_run: bool) -> Result<Vec<String>> {
    let mut cleaned_files = Vec::new();
    let profiles = get_shell_profile_paths();
    let stratum_home = get_stratum_home()?;
    let stratum_bin = stratum_home.join("bin").to_string_lossy().to_string();
    let stratum_home_str = stratum_home.to_string_lossy().to_string();

    for profile in profiles {
        if !profile.exists() {
            continue;
        }

        let content = match fs::read_to_string(&profile) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Check if file contains any Stratum-related entries
        let has_stratum = content.contains(&stratum_bin)
            || content.contains("STRATUM_HOME")
            || content.contains(&stratum_home_str)
            || content.to_lowercase().contains("stratum");

        if !has_stratum {
            continue;
        }

        // Filter out Stratum-related lines
        // This handles:
        // - export STRATUM_HOME=...
        // - export PATH="$STRATUM_HOME/bin:$PATH"
        // - source completions
        // - # Stratum comments
        // - Fish-specific: set -x STRATUM_HOME ...
        let new_content: String = content
            .lines()
            .filter(|line| {
                let line_lower = line.to_lowercase();
                let line_trimmed = line.trim();

                // Skip comment lines about Stratum
                if line_trimmed.starts_with('#') && line_lower.contains("stratum") {
                    return false;
                }

                // Skip STRATUM_HOME exports (bash/zsh)
                if line_lower.contains("stratum_home") {
                    return false;
                }

                // Skip PATH modifications containing stratum bin
                if line_lower.contains(&stratum_bin.to_lowercase()) {
                    return false;
                }

                // Skip fish set commands for stratum
                if line_trimmed.starts_with("set ") && line_lower.contains("stratum") {
                    return false;
                }

                // Skip source commands for stratum completions
                if (line_trimmed.starts_with("source ") || line_trimmed.starts_with(". "))
                    && line_lower.contains("stratum")
                {
                    return false;
                }

                // Skip fpath modifications for stratum completions
                if line_lower.contains("fpath") && line_lower.contains("stratum") {
                    return false;
                }

                // Skip stratum-specific config files
                if line_lower.contains(&stratum_home_str.to_lowercase()) {
                    return false;
                }

                true
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Add trailing newline if original had one
        let new_content = if content.ends_with('\n') && !new_content.ends_with('\n') {
            new_content + "\n"
        } else {
            new_content
        };

        // Remove any double blank lines that may have been created
        let new_content = remove_excessive_blank_lines(&new_content);

        if content != new_content {
            if dry_run {
                println!("  Would update: {}", profile.display());
            } else {
                fs::write(&profile, &new_content).with_context(|| {
                    format!("Failed to update shell profile: {}", profile.display())
                })?;
            }
            cleaned_files.push(profile.display().to_string());
        }
    }

    Ok(cleaned_files)
}

/// Remove excessive blank lines (more than 2 consecutive)
fn remove_excessive_blank_lines(content: &str) -> String {
    let mut result = Vec::new();
    let mut blank_count = 0;

    for line in content.lines() {
        if line.trim().is_empty() {
            blank_count += 1;
            // Allow up to 1 consecutive blank line (2 newlines)
            if blank_count <= 1 {
                result.push("");
            }
        } else {
            blank_count = 0;
            result.push(line);
        }
    }

    // Join with newlines and add trailing newline if content had one
    let mut output = result.join("\n");
    if content.ends_with('\n') && !output.is_empty() {
        output.push('\n');
    }
    output
}

/// Remove shell completion files
fn remove_completions(dry_run: bool) -> Result<Vec<String>> {
    let mut removed = Vec::new();
    let paths = get_completion_paths();

    for path in paths {
        if path.exists() {
            if dry_run {
                println!("  Would remove: {}", path.display());
            } else {
                if let Err(e) = fs::remove_file(&path) {
                    eprintln!("  Warning: Could not remove {}: {}", path.display(), e);
                    continue;
                }
            }
            removed.push(path.display().to_string());
        }
    }

    Ok(removed)
}

/// Prompt user for confirmation
fn confirm(message: &str) -> bool {
    print!("{} [y/N] ", message);
    std::io::stdout().flush().ok();

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Update Stratum to the latest version
pub fn update(options: UpdateOptions) -> Result<()> {
    println!("Checking for Stratum updates...\n");

    // Check installation method
    let install_method = detect_install_method()?;
    match install_method {
        InstallMethod::Homebrew => {
            println!("Stratum was installed via Homebrew.");
            println!("Please use: brew upgrade stratum");
            return Ok(());
        }
        InstallMethod::Apt => {
            println!("Stratum was installed via apt/dpkg.");
            println!("Please use: sudo apt update && sudo apt upgrade stratum");
            return Ok(());
        }
        InstallMethod::Rpm => {
            println!("Stratum was installed via rpm.");
            println!("Please use: sudo dnf upgrade stratum");
            return Ok(());
        }
        InstallMethod::Cargo => {
            println!("Stratum was installed via cargo install.");
            println!("Please use: cargo install stratum-cli --force");
            return Ok(());
        }
        InstallMethod::Script | InstallMethod::Unknown => {
            // Continue with self-update
        }
    }

    // Read current installation metadata
    let meta = read_install_meta()?;
    let current_version = meta
        .as_ref()
        .map(|m| m.version.clone())
        .unwrap_or_else(|| stratum_core::VERSION.to_string());
    let current_tier = meta
        .as_ref()
        .map(|m| m.tier.clone())
        .unwrap_or_else(|| "full".to_string());

    println!("Current version: {}", current_version);
    println!("Current tier: {}", current_tier);

    // Fetch latest release
    let release = fetch_latest_release().context("Failed to check for updates")?;
    let target_tier = options.tier.unwrap_or(current_tier.clone());

    println!("Latest version: {}", release.version);

    // Compare versions
    let current_semver: semver::Version = current_version
        .parse()
        .unwrap_or(semver::Version::new(0, 0, 0));
    let latest_semver: semver::Version = release
        .version
        .parse()
        .unwrap_or(semver::Version::new(0, 0, 0));

    let needs_update = latest_semver > current_semver;
    let tier_changed = target_tier != current_tier;

    if !needs_update && !tier_changed && !options.force {
        println!("\nYou are already on the latest version.");
        return Ok(());
    }

    if !needs_update && tier_changed {
        println!("\nChanging tier from '{}' to '{}'", current_tier, target_tier);
    } else if needs_update {
        println!("\nNew version available!");
    }

    if options.dry_run {
        println!("\n[DRY RUN] Would update to {} (tier: {})", release.version, target_tier);
        return Ok(());
    }

    // Confirm update
    if !options.yes {
        let message = if tier_changed {
            format!(
                "Update to {} and change tier to '{}'?",
                release.version, target_tier
            )
        } else {
            format!("Update to {}?", release.version)
        };

        if !confirm(&message) {
            println!("Update cancelled.");
            return Ok(());
        }
    }

    // Detect platform
    let target = detect_target()?;
    println!("\nTarget platform: {}", target);

    // Find the appropriate asset
    let archive_name = format!("stratum-{}-{}-{}.tar.gz", release.version, target_tier, target);
    let checksum_name = format!("{}.sha256", archive_name);

    let archive_asset = release
        .assets
        .iter()
        .find(|a| a.name == archive_name)
        .with_context(|| format!("Release asset not found: {}", archive_name))?;

    let checksum_asset = release.assets.iter().find(|a| a.name == checksum_name);

    // Create temp directory for download
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let archive_path = temp_dir.path().join(&archive_name);
    let checksum_path = temp_dir.path().join(&checksum_name);

    // Download archive
    println!("\nDownloading {}...", archive_name);
    download_file(&archive_asset.download_url, &archive_path)?;

    // Download and verify checksum if available
    if let Some(checksum_asset) = checksum_asset {
        println!("Downloading checksum...");
        download_file(&checksum_asset.download_url, &checksum_path)?;

        print!("Verifying checksum... ");
        std::io::stdout().flush().ok();

        if !verify_checksum(&archive_path, &checksum_path)? {
            bail!("Checksum verification failed! The download may be corrupted.");
        }
        println!("OK");
    } else {
        println!("Warning: No checksum available for verification");
    }

    // Get installation directory
    let stratum_home = get_stratum_home()?;
    let backup_dir = temp_dir.path().join("backup");

    // Backup current installation
    println!("\nBacking up current installation...");
    if stratum_home.exists() {
        fs::create_dir_all(&backup_dir).context("Failed to create backup directory")?;

        // Backup bin directory
        let bin_dir = stratum_home.join("bin");
        if bin_dir.exists() {
            let backup_bin = backup_dir.join("bin");
            copy_dir_all(&bin_dir, &backup_bin)?;
        }
    }

    // Extract new version
    println!("Extracting new version...");
    let extract_dir = temp_dir.path().join("extract");
    fs::create_dir_all(&extract_dir).context("Failed to create extraction directory")?;
    extract_tarball(&archive_path, &extract_dir)?;

    // Install new files
    println!("Installing new version...");
    fs::create_dir_all(&stratum_home).context("Failed to create Stratum home directory")?;

    // Copy extracted files to installation directory
    copy_dir_all(&extract_dir, &stratum_home)?;

    // Update metadata
    let meta_path = stratum_home.join(".install-meta");
    let new_meta = InstallMeta {
        version: release.version.clone(),
        tier: target_tier.clone(),
        target,
        installed_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        installer_version: "1.0.0".to_string(),
    };
    new_meta.write(&meta_path)?;

    println!("\nStratum updated successfully to version {}!", release.version);
    println!("\nTo verify, run: stratum --version");

    Ok(())
}

/// Recursively copy a directory
fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;

            // Preserve executable permissions on Unix
            #[cfg(unix)]
            {
                let metadata = fs::metadata(&src_path)?;
                fs::set_permissions(&dst_path, metadata.permissions())?;
            }
        }
    }

    Ok(())
}

/// Uninstall Stratum
pub fn uninstall(options: UninstallOptions) -> Result<()> {
    println!("Stratum Uninstaller\n");

    // Check installation method
    let install_method = detect_install_method()?;
    match install_method {
        InstallMethod::Homebrew => {
            println!("Stratum was installed via Homebrew.");
            println!("Please use: brew uninstall stratum");
            println!("\nNote: User data in ~/.stratum will not be removed.");
            println!("To remove it manually: rm -rf ~/.stratum");
            if options.purge {
                println!("\nTo also remove Workshop IDE data on macOS:");
                println!("  rm -rf ~/Library/Application\\ Support/Stratum\\ Workshop");
                println!("  rm -rf ~/Library/Caches/dev.stratum-lang.workshop");
            }
            return Ok(());
        }
        InstallMethod::Apt => {
            println!("Stratum was installed via apt/dpkg.");
            println!("Please use: sudo apt remove stratum");
            if options.purge {
                println!("         or: sudo apt purge stratum");
                println!("\nNote: To also remove user data:");
                println!("  rm -rf ~/.stratum");
                println!("  rm -rf ~/.config/stratum-workshop");
                println!("  rm -rf ~/.local/share/stratum-workshop");
            }
            return Ok(());
        }
        InstallMethod::Rpm => {
            println!("Stratum was installed via rpm.");
            println!("Please use: sudo dnf remove stratum");
            if options.purge {
                println!("\nNote: To also remove user data:");
                println!("  rm -rf ~/.stratum");
                println!("  rm -rf ~/.config/stratum-workshop");
                println!("  rm -rf ~/.local/share/stratum-workshop");
            }
            return Ok(());
        }
        InstallMethod::Cargo => {
            println!("Stratum was installed via cargo install.");
            println!("Please use: cargo uninstall stratum-cli");
            println!("\nNote: User data in ~/.stratum will not be removed.");
            println!("To remove it manually: rm -rf ~/.stratum");
            return Ok(());
        }
        InstallMethod::Script | InstallMethod::Unknown => {
            // Continue with self-uninstall
        }
    }

    let stratum_home = get_stratum_home()?;

    // Collect items that will be removed
    let shell_profiles = get_shell_profile_paths()
        .into_iter()
        .filter(|p| p.exists())
        .collect::<Vec<_>>();

    let completions = get_completion_paths()
        .into_iter()
        .filter(|p| p.exists())
        .collect::<Vec<_>>();

    let user_data = get_user_data_paths()?
        .into_iter()
        .filter(|(p, _)| p.exists())
        .collect::<Vec<_>>();

    let workshop_paths = get_workshop_ide_paths()
        .into_iter()
        .filter(|(p, _)| p.exists())
        .collect::<Vec<_>>();

    // Show what will be removed
    println!("The following will be removed:\n");

    // Core installation
    println!("  Core installation:");
    let core_items = ["bin", "lib", "share", ".install-meta"];
    for item in &core_items {
        let path = stratum_home.join(item);
        if path.exists() {
            println!("    - {}", path.display());
        }
    }

    // Shell profiles
    if !shell_profiles.is_empty() {
        println!("\n  Shell profile modifications (PATH, STRATUM_HOME):");
        for profile in &shell_profiles {
            // Check if profile actually contains Stratum entries
            if let Ok(content) = fs::read_to_string(profile) {
                if content.to_lowercase().contains("stratum") {
                    println!("    - {}", profile.display());
                }
            }
        }
    }

    // Shell completions
    if !completions.is_empty() {
        println!("\n  Shell completions:");
        for completion in &completions {
            println!("    - {}", completion.display());
        }
    }

    // Purge-only items
    if options.purge {
        println!("\n  User data (--purge):");

        // User data in stratum home
        if !user_data.is_empty() {
            for (path, desc) in &user_data {
                println!("    - {} ({})", path.display(), desc);
            }
        }

        // Config file
        let config_file = stratum_home.join("config.toml");
        if config_file.exists() {
            println!("    - {} (user configuration)", config_file.display());
        }

        // Workshop IDE data
        if !workshop_paths.is_empty() {
            println!("\n  Workshop IDE data:");
            for (path, desc) in &workshop_paths {
                println!("    - {} ({})", path.display(), desc);
            }
        }
    } else {
        // Show preserved items
        let mut preserved = Vec::new();

        let config_file = stratum_home.join("config.toml");
        if config_file.exists() {
            preserved.push(("config.toml", "user configuration"));
        }

        for (path, desc) in &user_data {
            if path.exists() {
                if let Some(name) = path.file_name() {
                    preserved.push((name.to_str().unwrap_or("unknown"), *desc));
                }
            }
        }

        if !preserved.is_empty() {
            println!("\n  Note: The following will be preserved:");
            for (name, desc) in &preserved {
                println!("    - {} ({})", name, desc);
            }
            println!("        Use --purge to remove all user data");
        }

        if !workshop_paths.is_empty() {
            println!("\n  Note: Workshop IDE data will be preserved:");
            for (path, desc) in &workshop_paths {
                println!("    - {} ({})", path.display(), desc);
            }
        }
    }

    println!();

    // Confirm uninstall
    if !options.yes {
        let message = if options.purge {
            "This will remove Stratum and ALL user data including Workshop IDE settings. Continue?"
        } else {
            "This will remove Stratum. User configuration will be preserved. Continue?"
        };

        if !confirm(message) {
            println!("Uninstall cancelled.");
            return Ok(());
        }
    }

    println!("\nUninstalling Stratum...\n");

    // Track statistics
    let mut removed_count = 0;
    let mut warning_count = 0;

    // Remove PATH entries and STRATUM_HOME from shell profiles
    print!("Cleaning shell profiles... ");
    std::io::stdout().flush().ok();
    let cleaned_profiles = remove_path_from_profiles(false)?;
    if cleaned_profiles.is_empty() {
        println!("no changes needed");
    } else {
        println!("done ({} files updated)", cleaned_profiles.len());
        removed_count += cleaned_profiles.len();
    }

    // Remove shell completions
    print!("Removing shell completions... ");
    std::io::stdout().flush().ok();
    let removed_completions = remove_completions(false)?;
    if removed_completions.is_empty() {
        println!("none found");
    } else {
        println!("done ({} files removed)", removed_completions.len());
        removed_count += removed_completions.len();
    }

    // Remove core installation files
    if stratum_home.exists() {
        print!("Removing core installation... ");
        std::io::stdout().flush().ok();

        let core_items = ["bin", "lib", "share", ".install-meta"];
        let mut core_removed = 0;

        for item in core_items {
            let path = stratum_home.join(item);
            if remove_path_with_warning(&path, item, false)? {
                core_removed += 1;
            }
        }

        if core_removed > 0 {
            println!("done ({} items removed)", core_removed);
            removed_count += core_removed;
        } else {
            println!("no items found");
        }
    }

    // Handle purge mode - remove all user data
    if options.purge {
        // Remove user data directories (packages, cache, history, etc.)
        print!("Removing user data... ");
        std::io::stdout().flush().ok();

        let user_data_paths = get_user_data_paths()?;
        let mut user_removed = 0;

        for (path, desc) in user_data_paths {
            match remove_path_with_warning(&path, desc, false) {
                Ok(true) => user_removed += 1,
                Ok(false) => {}
                Err(_) => warning_count += 1,
            }
        }

        // Remove config file
        let config_path = stratum_home.join("config.toml");
        if remove_path_with_warning(&config_path, "user configuration", false)? {
            user_removed += 1;
        }

        if user_removed > 0 {
            println!("done ({} items removed)", user_removed);
            removed_count += user_removed;
        } else {
            println!("no user data found");
        }

        // Remove Workshop IDE data
        let workshop_paths = get_workshop_ide_paths();
        if !workshop_paths.is_empty() {
            print!("Removing Workshop IDE data... ");
            std::io::stdout().flush().ok();

            let mut workshop_removed = 0;
            for (path, desc) in workshop_paths {
                match remove_path_with_warning(&path, desc, false) {
                    Ok(true) => workshop_removed += 1,
                    Ok(false) => {}
                    Err(_) => warning_count += 1,
                }
            }

            if workshop_removed > 0 {
                println!("done ({} items removed)", workshop_removed);
                removed_count += workshop_removed;
            } else {
                println!("no Workshop IDE data found");
            }
        }

        // Finally, remove the stratum home directory itself if empty
        if stratum_home.exists() {
            match fs::remove_dir_all(&stratum_home) {
                Ok(()) => {}
                Err(e) => {
                    // Check if it's just because it's not empty (preserved files)
                    if e.kind() != std::io::ErrorKind::NotFound {
                        eprintln!(
                            "  Warning: Could not fully remove {}: {}",
                            stratum_home.display(),
                            e
                        );
                        warning_count += 1;
                    }
                }
            }
        }
    } else {
        // Non-purge: Check if stratum home is empty after removing core files
        if stratum_home.exists() {
            let remaining: Vec<_> = fs::read_dir(&stratum_home)
                .ok()
                .map(|entries| entries.filter_map(|e| e.ok()).collect())
                .unwrap_or_default();

            if remaining.is_empty() {
                fs::remove_dir(&stratum_home).ok();
            }
        }
    }

    // Print summary
    println!();
    println!("Stratum has been uninstalled.");

    if removed_count > 0 {
        println!("  {} items removed", removed_count);
    }

    if warning_count > 0 {
        println!("  {} warnings (some items may need manual removal)", warning_count);
    }

    // Remind user to reload shell
    println!("\nPlease restart your shell or run:");

    // Detect which shell the user is likely using
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("zsh") {
            println!("  source ~/.zshrc");
        } else if shell.contains("fish") {
            println!("  source ~/.config/fish/config.fish");
        } else {
            println!("  source ~/.bashrc");
        }
    } else {
        println!("  source ~/.bashrc  # or ~/.zshrc, etc.");
    }

    Ok(())
}

/// Get the versions directory (~/.stratum/versions)
pub fn get_versions_dir() -> Result<PathBuf> {
    let stratum_home = get_stratum_home()?;
    Ok(stratum_home.join("versions"))
}

/// Get the path to the active version file
fn get_active_version_path() -> Result<PathBuf> {
    let stratum_home = get_stratum_home()?;
    Ok(stratum_home.join(".active-version"))
}

/// Read the currently active version
pub fn get_active_version() -> Result<Option<String>> {
    let path = get_active_version_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let version = fs::read_to_string(&path)
        .context("Failed to read active version")?
        .trim()
        .to_string();
    if version.is_empty() {
        return Ok(None);
    }
    Ok(Some(version))
}

/// Set the active version
fn set_active_version(version: &str) -> Result<()> {
    let path = get_active_version_path()?;
    fs::write(&path, format!("{}\n", version)).context("Failed to write active version")?;
    Ok(())
}

/// Get list of installed versions with their metadata
pub fn get_installed_versions() -> Result<Vec<(String, Option<InstallMeta>)>> {
    let versions_dir = get_versions_dir()?;
    let mut versions = Vec::new();

    if !versions_dir.exists() {
        return Ok(versions);
    }

    for entry in fs::read_dir(&versions_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let version = entry.file_name().to_string_lossy().to_string();

            // Try to read metadata for this version
            let meta_path = entry.path().join(".install-meta");
            let meta = if meta_path.exists() {
                fs::read_to_string(&meta_path)
                    .ok()
                    .map(|content| InstallMeta::parse(&content))
            } else {
                None
            };

            versions.push((version, meta));
        }
    }

    // Sort versions using semver
    versions.sort_by(|a, b| {
        let a_ver: semver::Version = a.0.parse().unwrap_or(semver::Version::new(0, 0, 0));
        let b_ver: semver::Version = b.0.parse().unwrap_or(semver::Version::new(0, 0, 0));
        b_ver.cmp(&a_ver) // Descending order (newest first)
    });

    Ok(versions)
}

/// Check if a specific version is installed
fn is_version_installed(version: &str) -> Result<bool> {
    let versions_dir = get_versions_dir()?;
    let version_dir = versions_dir.join(version);
    Ok(version_dir.exists() && version_dir.is_dir())
}

/// Fetch a specific release by version tag from GitHub
fn fetch_release(version: &str) -> Result<ReleaseInfo> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("stratum-cli")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    // Normalize version: add 'v' prefix if not present for the tag
    let tag = if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{}", version)
    };

    let url = format!(
        "https://api.github.com/repos/horizon-analytic/stratum/releases/tags/{}",
        tag
    );

    let response = client
        .get(&url)
        .send()
        .context("Failed to fetch release information")?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        bail!("Version {} not found in GitHub releases", version);
    }

    if !response.status().is_success() {
        bail!(
            "Failed to fetch release information: HTTP {}",
            response.status()
        );
    }

    let json: serde_json::Value = response.json().context("Failed to parse release JSON")?;

    let tag_name = json["tag_name"]
        .as_str()
        .context("Missing tag_name in release")?
        .to_string();

    let version_str = tag_name.trim_start_matches('v').to_string();

    let published_at = json["published_at"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let assets = json["assets"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|asset| {
                    Some(ReleaseAsset {
                        name: asset["name"].as_str()?.to_string(),
                        download_url: asset["browser_download_url"].as_str()?.to_string(),
                        size: asset["size"].as_u64().unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(ReleaseInfo {
        version: version_str,
        tag_name,
        published_at,
        assets,
    })
}

/// Fetch available releases from GitHub (paginated)
fn fetch_available_releases(limit: usize) -> Result<Vec<ReleaseInfo>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("stratum-cli")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let url = format!(
        "https://api.github.com/repos/horizon-analytic/stratum/releases?per_page={}",
        limit
    );

    let response = client
        .get(&url)
        .send()
        .context("Failed to fetch releases")?;

    if !response.status().is_success() {
        bail!(
            "Failed to fetch releases: HTTP {}",
            response.status()
        );
    }

    let json: Vec<serde_json::Value> = response.json().context("Failed to parse releases JSON")?;

    let releases = json
        .iter()
        .filter_map(|release| {
            let tag_name = release["tag_name"].as_str()?.to_string();
            let version = tag_name.trim_start_matches('v').to_string();
            let published_at = release["published_at"].as_str().unwrap_or("").to_string();

            let assets = release["assets"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|asset| {
                            Some(ReleaseAsset {
                                name: asset["name"].as_str()?.to_string(),
                                download_url: asset["browser_download_url"].as_str()?.to_string(),
                                size: asset["size"].as_u64().unwrap_or(0),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            Some(ReleaseInfo {
                version,
                tag_name,
                published_at,
                assets,
            })
        })
        .collect();

    Ok(releases)
}

/// Create symlinks from the active version to the bin directory
fn activate_version_symlinks(version: &str) -> Result<()> {
    let stratum_home = get_stratum_home()?;
    let versions_dir = get_versions_dir()?;
    let version_dir = versions_dir.join(version);
    let version_bin = version_dir.join("bin");
    let main_bin = stratum_home.join("bin");

    // Ensure the version bin directory exists
    if !version_bin.exists() {
        bail!("Version {} bin directory not found at {}", version, version_bin.display());
    }

    // Create main bin directory if it doesn't exist
    fs::create_dir_all(&main_bin).context("Failed to create bin directory")?;

    // Remove existing symlinks/files in main bin
    if main_bin.exists() {
        for entry in fs::read_dir(&main_bin)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_symlink() || path.is_file() {
                fs::remove_file(&path).ok();
            }
        }
    }

    // Create symlinks to the version's binaries
    for entry in fs::read_dir(&version_bin)? {
        let entry = entry?;
        let src = entry.path();
        let filename = entry.file_name();
        let dst = main_bin.join(&filename);

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&src, &dst).with_context(|| {
                format!("Failed to create symlink: {} -> {}", dst.display(), src.display())
            })?;
        }

        #[cfg(windows)]
        {
            // On Windows, copy the file instead of symlinking (requires admin for symlinks)
            fs::copy(&src, &dst).with_context(|| {
                format!("Failed to copy binary: {} -> {}", src.display(), dst.display())
            })?;
        }
    }

    // Update the active version file
    set_active_version(version)?;

    Ok(())
}

/// Install a specific version of Stratum
pub fn install_version(options: InstallVersionOptions) -> Result<()> {
    // Normalize version (remove 'v' prefix if present for storage)
    let version = options.version.trim_start_matches('v').to_string();

    // Validate tier
    let valid_tiers = ["core", "data", "gui", "full"];
    if !valid_tiers.contains(&options.tier.as_str()) {
        bail!(
            "Invalid tier '{}'. Valid tiers are: {}",
            options.tier,
            valid_tiers.join(", ")
        );
    }

    println!("Stratum Version Installer\n");

    // Check if version is already installed
    if is_version_installed(&version)? {
        println!("Version {} is already installed.", version);

        if options.activate {
            let active = get_active_version()?;
            if active.as_deref() == Some(&version) {
                println!("Version {} is already the active version.", version);
                return Ok(());
            }

            if !options.yes && !confirm(&format!("Switch to version {}?", version)) {
                println!("Operation cancelled.");
                return Ok(());
            }

            activate_version_symlinks(&version)?;
            println!("\nSwitched to version {}.", version);
        }
        return Ok(());
    }

    // Fetch the specific release
    println!("Fetching release information for version {}...", version);
    let release = fetch_release(&version)?;

    println!("Found version: {}", release.version);
    println!("Published: {}", release.published_at);
    println!("Tier: {}", options.tier);

    // Detect platform
    let target = detect_target()?;
    println!("Target platform: {}", target);

    // Find the appropriate asset
    let archive_name = format!("stratum-{}-{}-{}.tar.gz", release.version, options.tier, target);
    let checksum_name = format!("{}.sha256", archive_name);

    let archive_asset = release
        .assets
        .iter()
        .find(|a| a.name == archive_name)
        .with_context(|| format!(
            "Release asset not found: {}\nAvailable assets: {}",
            archive_name,
            release.assets.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")
        ))?;

    let checksum_asset = release.assets.iter().find(|a| a.name == checksum_name);

    // Confirm installation
    if !options.yes {
        let size_mb = archive_asset.size as f64 / 1_048_576.0;
        println!("\nDownload size: {:.1} MB", size_mb);

        if !confirm(&format!("Install Stratum {}?", version)) {
            println!("Installation cancelled.");
            return Ok(());
        }
    }

    // Create temp directory for download
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let archive_path = temp_dir.path().join(&archive_name);
    let checksum_path = temp_dir.path().join(&checksum_name);

    // Download archive
    println!("\nDownloading {}...", archive_name);
    download_file(&archive_asset.download_url, &archive_path)?;

    // Download and verify checksum if available
    if let Some(checksum_asset) = checksum_asset {
        println!("Downloading checksum...");
        download_file(&checksum_asset.download_url, &checksum_path)?;

        print!("Verifying checksum... ");
        std::io::stdout().flush().ok();

        if !verify_checksum(&archive_path, &checksum_path)? {
            bail!("Checksum verification failed! The download may be corrupted.");
        }
        println!("OK");
    } else {
        println!("Warning: No checksum available for verification");
    }

    // Create version directory
    let versions_dir = get_versions_dir()?;
    let version_dir = versions_dir.join(&version);
    fs::create_dir_all(&version_dir).context("Failed to create version directory")?;

    // Extract archive
    println!("Extracting to {}...", version_dir.display());
    let extract_dir = temp_dir.path().join("extract");
    fs::create_dir_all(&extract_dir)?;
    extract_tarball(&archive_path, &extract_dir)?;

    // Copy extracted files to version directory
    copy_dir_all(&extract_dir, &version_dir)?;

    // Write version-specific metadata
    let meta_path = version_dir.join(".install-meta");
    let meta = InstallMeta {
        version: version.clone(),
        tier: options.tier.clone(),
        target,
        installed_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        installer_version: "1.0.0".to_string(),
    };
    meta.write(&meta_path)?;

    println!("\nVersion {} installed successfully!", version);

    // Activate if requested
    if options.activate {
        println!("Activating version {}...", version);
        activate_version_symlinks(&version)?;

        // Also update the main .install-meta for compatibility
        let stratum_home = get_stratum_home()?;
        let main_meta_path = stratum_home.join(".install-meta");
        meta.write(&main_meta_path)?;

        println!("Version {} is now active.", version);
    } else {
        println!("\nTo activate this version, run:");
        println!("  stratum self use {}", version);
    }

    Ok(())
}

/// Switch to a different installed version
pub fn use_version(version: &str) -> Result<()> {
    // Normalize version
    let version = version.trim_start_matches('v');

    println!("Switching to Stratum version {}...\n", version);

    // Check if version is installed
    if !is_version_installed(version)? {
        let installed = get_installed_versions()?;

        if installed.is_empty() {
            println!("No versions installed. Install a version first:");
            println!("  stratum self install {}", version);
        } else {
            println!("Version {} is not installed.", version);
            println!("\nInstalled versions:");
            for (v, _) in &installed {
                println!("  - {}", v);
            }
            println!("\nTo install version {}:", version);
            println!("  stratum self install {}", version);
        }
        return Ok(());
    }

    // Check if already active
    let active = get_active_version()?;
    if active.as_deref() == Some(version) {
        println!("Version {} is already the active version.", version);
        return Ok(());
    }

    // Activate the version
    activate_version_symlinks(version)?;

    // Update the main .install-meta
    let versions_dir = get_versions_dir()?;
    let version_meta_path = versions_dir.join(version).join(".install-meta");
    if version_meta_path.exists() {
        let content = fs::read_to_string(&version_meta_path)?;
        let meta = InstallMeta::parse(&content);

        let stratum_home = get_stratum_home()?;
        let main_meta_path = stratum_home.join(".install-meta");
        meta.write(&main_meta_path)?;
    }

    println!("Switched to version {}.", version);
    println!("\nTo verify, run: stratum --version");

    Ok(())
}

/// List installed versions and optionally available releases
pub fn list_versions(show_available: bool) -> Result<()> {
    println!("Stratum Versions\n");

    let active_version = get_active_version()?;
    let installed = get_installed_versions()?;

    // Show installed versions
    if installed.is_empty() {
        println!("No versions installed.");
        println!("\nTo install a version:");
        println!("  stratum self install <version>");
    } else {
        println!("Installed:");
        for (version, meta) in &installed {
            let is_active = active_version.as_deref() == Some(version);
            let marker = if is_active { " (active)" } else { "" };
            let tier_info = meta.as_ref()
                .map(|m| format!(" [{}]", m.tier))
                .unwrap_or_default();

            println!("  {} {}{}{}",
                if is_active { "*" } else { " " },
                version,
                tier_info,
                marker
            );
        }
    }

    // Show available versions from GitHub
    if show_available {
        println!("\nFetching available releases...");

        match fetch_available_releases(10) {
            Ok(releases) => {
                if releases.is_empty() {
                    println!("\nNo releases available on GitHub.");
                } else {
                    println!("\nAvailable (latest 10 releases):");
                    for release in releases {
                        let installed_marker = if is_version_installed(&release.version)? {
                            " [installed]"
                        } else {
                            ""
                        };

                        let date = if !release.published_at.is_empty() {
                            // Parse and format the date nicely
                            release.published_at.split('T').next().unwrap_or("")
                        } else {
                            ""
                        };

                        println!("    {} ({}){}", release.version, date, installed_marker);
                    }
                }
            }
            Err(e) => {
                println!("\nCould not fetch available releases: {}", e);
            }
        }
    } else {
        println!("\nUse --available to see releases from GitHub.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_install_meta() {
        let content = r#"version=0.1.0
tier=full
target=aarch64-apple-darwin
installed_at=2026-01-09T20:45:00Z
installer_version=1.0.0"#;

        let meta = InstallMeta::parse(content);
        assert_eq!(meta.version, "0.1.0");
        assert_eq!(meta.tier, "full");
        assert_eq!(meta.target, "aarch64-apple-darwin");
        assert_eq!(meta.installed_at, "2026-01-09T20:45:00Z");
        assert_eq!(meta.installer_version, "1.0.0");
    }

    #[test]
    fn test_parse_install_meta_empty() {
        let meta = InstallMeta::parse("");
        assert!(meta.version.is_empty());
        assert!(meta.tier.is_empty());
    }

    #[test]
    fn test_detect_target() {
        // This should work on any supported platform
        let result = detect_target();
        assert!(result.is_ok());

        let target = result.unwrap();
        assert!(!target.is_empty());

        // Should contain expected parts
        assert!(
            target.contains("apple-darwin")
                || target.contains("unknown-linux")
                || target.contains("pc-windows")
        );
    }

    #[test]
    fn test_get_stratum_home_default() {
        // Save current value and clear env var for test
        let saved = std::env::var("STRATUM_HOME").ok();
        std::env::remove_var("STRATUM_HOME");

        let result = get_stratum_home();
        assert!(result.is_ok());

        let home = result.unwrap();
        assert!(home.to_string_lossy().ends_with(".stratum"));

        // Restore
        if let Some(val) = saved {
            std::env::set_var("STRATUM_HOME", val);
        }
    }

    #[test]
    fn test_get_stratum_home_env_override() {
        // Save current value
        let saved = std::env::var("STRATUM_HOME").ok();

        std::env::set_var("STRATUM_HOME", "/custom/path");

        let result = get_stratum_home();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/custom/path"));

        // Restore
        if let Some(val) = saved {
            std::env::set_var("STRATUM_HOME", val);
        } else {
            std::env::remove_var("STRATUM_HOME");
        }
    }

    #[test]
    fn test_install_method_display() {
        assert_eq!(InstallMethod::Script.to_string(), "install script");
        assert_eq!(InstallMethod::Homebrew.to_string(), "Homebrew");
        assert_eq!(InstallMethod::Cargo.to_string(), "cargo install");
    }

    #[test]
    fn test_get_versions_dir() {
        // Save current value
        let saved = std::env::var("STRATUM_HOME").ok();

        std::env::set_var("STRATUM_HOME", "/test/stratum");

        let result = get_versions_dir();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/test/stratum/versions"));

        // Restore
        if let Some(val) = saved {
            std::env::set_var("STRATUM_HOME", val);
        } else {
            std::env::remove_var("STRATUM_HOME");
        }
    }

    #[test]
    fn test_get_installed_versions_empty() {
        // Save current value
        let saved = std::env::var("STRATUM_HOME").ok();

        // Use a temp directory that doesn't exist
        let temp_dir = tempfile::tempdir().unwrap();
        let stratum_home = temp_dir.path().join("stratum_test");
        std::env::set_var("STRATUM_HOME", stratum_home.to_str().unwrap());

        let result = get_installed_versions();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        // Restore
        if let Some(val) = saved {
            std::env::set_var("STRATUM_HOME", val);
        } else {
            std::env::remove_var("STRATUM_HOME");
        }
    }

    #[test]
    fn test_is_version_installed() {
        // Save current value
        let saved = std::env::var("STRATUM_HOME").ok();

        // Use a temp directory
        let temp_dir = tempfile::tempdir().unwrap();
        let stratum_home = temp_dir.path().join("stratum_test");
        std::env::set_var("STRATUM_HOME", stratum_home.to_str().unwrap());

        // Version doesn't exist
        let result = is_version_installed("1.0.0");
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // Create the version directory
        let versions_dir = stratum_home.join("versions");
        fs::create_dir_all(versions_dir.join("1.0.0")).unwrap();

        // Now it should exist
        let result = is_version_installed("1.0.0");
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Restore
        if let Some(val) = saved {
            std::env::set_var("STRATUM_HOME", val);
        } else {
            std::env::remove_var("STRATUM_HOME");
        }
    }

    #[test]
    fn test_install_version_options_defaults() {
        let options = InstallVersionOptions {
            version: "1.0.0".to_string(),
            tier: "full".to_string(),
            yes: false,
            activate: true,
        };

        assert_eq!(options.version, "1.0.0");
        assert_eq!(options.tier, "full");
        assert!(!options.yes);
        assert!(options.activate);
    }

    #[test]
    fn test_get_user_data_paths() {
        // Save current value
        let saved = std::env::var("STRATUM_HOME").ok();

        std::env::set_var("STRATUM_HOME", "/test/stratum");

        let result = get_user_data_paths();
        assert!(result.is_ok());

        let paths = result.unwrap();
        // Should include packages, cache, history, lsp-cache, versions
        assert!(paths.len() >= 5);

        // Verify expected paths
        let path_strings: Vec<_> = paths.iter().map(|(p, _)| p.to_string_lossy().to_string()).collect();
        assert!(path_strings.iter().any(|p| p.contains("packages")));
        assert!(path_strings.iter().any(|p| p.contains("cache")));
        assert!(path_strings.iter().any(|p| p.contains("history") || p.contains("repl")));
        assert!(path_strings.iter().any(|p| p.contains("lsp")));
        assert!(path_strings.iter().any(|p| p.contains("versions")));

        // Restore
        if let Some(val) = saved {
            std::env::set_var("STRATUM_HOME", val);
        } else {
            std::env::remove_var("STRATUM_HOME");
        }
    }

    #[test]
    fn test_get_workshop_ide_paths() {
        let paths = get_workshop_ide_paths();

        // On macOS, should have Application Support, Preferences, Caches, etc.
        // On Linux, should have XDG paths
        // Platform-specific, so just verify it doesn't panic
        #[cfg(target_os = "macos")]
        {
            assert!(!paths.is_empty());
            let path_strings: Vec<_> = paths.iter().map(|(p, _)| p.to_string_lossy().to_string()).collect();
            assert!(path_strings.iter().any(|p| p.contains("Library")));
        }

        #[cfg(target_os = "linux")]
        {
            assert!(!paths.is_empty());
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            assert!(paths.is_empty());
        }
    }

    #[test]
    fn test_remove_path_with_warning_nonexistent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let nonexistent = temp_dir.path().join("does_not_exist");

        // Should return false for nonexistent path
        let result = remove_path_with_warning(&nonexistent, "test", false);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_remove_path_with_warning_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        fs::write(&file_path, "test content").unwrap();

        assert!(file_path.exists());

        // Should successfully remove file
        let result = remove_path_with_warning(&file_path, "test file", false);
        assert!(result.is_ok());
        assert!(result.unwrap());
        assert!(!file_path.exists());
    }

    #[test]
    fn test_remove_path_with_warning_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir_path = temp_dir.path().join("test_dir");
        fs::create_dir(&dir_path).unwrap();
        fs::write(dir_path.join("file.txt"), "content").unwrap();

        assert!(dir_path.exists());

        // Should successfully remove directory and contents
        let result = remove_path_with_warning(&dir_path, "test directory", false);
        assert!(result.is_ok());
        assert!(result.unwrap());
        assert!(!dir_path.exists());
    }

    #[test]
    fn test_remove_path_with_warning_dry_run() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        fs::write(&file_path, "test content").unwrap();

        assert!(file_path.exists());

        // Dry run should NOT remove file
        let result = remove_path_with_warning(&file_path, "test file", true);
        assert!(result.is_ok());
        assert!(result.unwrap()); // Returns true because it would have been removed
        assert!(file_path.exists()); // But file still exists
    }

    #[test]
    fn test_remove_excessive_blank_lines() {
        let input = "line1\n\n\n\n\nline2\n";
        let result = remove_excessive_blank_lines(input);
        // Should collapse multiple blank lines to at most 1
        // "line1\n\nline2\n" -> 2 newlines between lines (1 blank line)
        assert_eq!(result, "line1\n\nline2\n");
    }

    #[test]
    fn test_remove_excessive_blank_lines_preserves_content() {
        let input = "line1\nline2\nline3\n";
        let result = remove_excessive_blank_lines(input);
        assert!(result.contains("line1"));
        assert!(result.contains("line2"));
        assert!(result.contains("line3"));
    }

    #[test]
    fn test_get_shell_profile_paths() {
        let paths = get_shell_profile_paths();

        // Should include common shell profiles
        let path_strings: Vec<_> = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();

        // Check for bash profiles
        assert!(path_strings.iter().any(|p| p.contains("bashrc") || p.contains("bash_profile")));

        // Check for zsh profiles
        assert!(path_strings.iter().any(|p| p.contains("zshrc") || p.contains("zprofile")));

        // Check for fish config
        assert!(path_strings.iter().any(|p| p.contains("fish")));
    }

    #[test]
    fn test_get_completion_paths() {
        let paths = get_completion_paths();

        // Should include completion paths for bash, zsh, and fish
        let path_strings: Vec<_> = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();

        assert!(path_strings.iter().any(|p| p.contains("bash-completion")));
        assert!(path_strings.iter().any(|p| p.contains("zfunc") || p.contains("zsh")));
        assert!(path_strings.iter().any(|p| p.contains("fish")));
    }
}
