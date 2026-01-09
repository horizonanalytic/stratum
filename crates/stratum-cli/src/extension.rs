//! Implementation of the `stratum extension` command for managing VS Code extension.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Install the Stratum VS Code extension
pub fn install_extension(vsix_path: Option<PathBuf>) -> Result<()> {
    // Determine VSIX path
    let path = match vsix_path {
        Some(p) => p,
        None => find_bundled_vsix()?,
    };

    if !path.exists() {
        return Err(anyhow::anyhow!(
            "VSIX file not found at '{}'\n\n\
            You can either:\n\
            - Provide a path with: stratum extension install --vsix /path/to/stratum.vsix\n\
            - Download the VSIX from a GitHub release\n\
            - Build it from source: cd editors/vscode/stratum && npm run package",
            path.display()
        ));
    }

    // Check for VS Code CLI
    check_vscode_cli()?;

    println!("Installing VS Code extension from '{}'...", path.display());

    let output = Command::new("code")
        .args(["--install-extension", path.to_str().unwrap(), "--force"])
        .output()
        .context("Failed to run VS Code CLI")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to install extension: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        println!("{}", stdout.trim());
    }

    println!("Stratum VS Code extension installed successfully!");
    println!("\nRestart VS Code to activate the extension.");

    Ok(())
}

/// List installed VS Code extensions (filtering for stratum)
pub fn list_extensions() -> Result<()> {
    check_vscode_cli()?;

    let output = Command::new("code")
        .args(["--list-extensions", "--show-versions"])
        .output()
        .context("Failed to list extensions")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to list extensions: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stratum_extensions: Vec<&str> = stdout
        .lines()
        .filter(|line| line.to_lowercase().contains("stratum"))
        .collect();

    if stratum_extensions.is_empty() {
        println!("Stratum extension is not installed.");
        println!("\nInstall it with: stratum extension install");
    } else {
        println!("Installed Stratum extensions:");
        for ext in stratum_extensions {
            println!("  {}", ext);
        }
    }

    Ok(())
}

/// Uninstall the Stratum VS Code extension
pub fn uninstall_extension() -> Result<()> {
    check_vscode_cli()?;

    println!("Uninstalling Stratum VS Code extension...");

    // Try both possible extension IDs
    let extension_ids = [
        "horizon-analytic-studios.stratum",
        "cache.stratum",
    ];

    let mut uninstalled = false;
    for ext_id in extension_ids {
        let output = Command::new("code")
            .args(["--uninstall-extension", ext_id])
            .output()
            .context("Failed to run VS Code CLI")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.contains("not installed") {
                uninstalled = true;
                println!("Uninstalled: {}", ext_id);
            }
        }
    }

    if uninstalled {
        println!("\nStratum extension uninstalled successfully!");
        println!("Restart VS Code to complete the uninstallation.");
    } else {
        println!("Stratum extension was not installed.");
    }

    Ok(())
}

/// Check if VS Code CLI is available
fn check_vscode_cli() -> Result<()> {
    Command::new("code")
        .arg("--version")
        .output()
        .map_err(|_| {
            anyhow::anyhow!(
                "VS Code CLI (code) not found in PATH.\n\n\
                Make sure VS Code is installed and the 'code' command is available.\n\
                On macOS: Open VS Code, then Cmd+Shift+P -> 'Shell Command: Install code command'\n\
                On Windows: VS Code installer adds 'code' to PATH by default\n\
                On Linux: The 'code' command is usually available after installing VS Code"
            )
        })?;
    Ok(())
}

/// Find the bundled VSIX file in common locations
fn find_bundled_vsix() -> Result<PathBuf> {
    // Try several common locations
    let candidates = [
        // Next to the stratum executable
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("stratum.vsix"))),
        // In the current directory
        Some(PathBuf::from("stratum.vsix")),
        // In a share directory (Unix-style installation)
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("../share/stratum/stratum.vsix"))),
        // User's home directory
        dirs::data_local_dir().map(|d| d.join("stratum/stratum.vsix")),
        // For development: in the editors directory relative to cargo manifest
        option_env!("CARGO_MANIFEST_DIR")
            .map(|d| PathBuf::from(d).join("../../editors/vscode/stratum/stratum.vsix")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // Return a sensible default path for the error message
    Ok(PathBuf::from("stratum.vsix"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_bundled_vsix_returns_path() {
        // Should always return a path (even if file doesn't exist)
        let result = find_bundled_vsix();
        assert!(result.is_ok());
    }
}
