# Troubleshooting

This guide covers common issues encountered when installing, upgrading, or running Stratum.

## Installation Issues

### Permission Denied Errors

**Symptom:** Installation fails with "Permission denied" or "EACCES" errors.

**Solutions:**

1. **Use a user-writable location** (recommended):
   ```bash
   curl -fsSL https://get.stratum-lang.dev | sh -s -- --prefix=$HOME/.stratum
   ```

2. **Fix directory permissions:**
   ```bash
   # If using /usr/local/stratum
   sudo chown -R $(whoami) /usr/local/stratum
   ```

3. **Use sudo for system-wide installation:**
   ```bash
   curl -fsSL https://get.stratum-lang.dev | sudo sh
   ```

### PATH Not Updated

**Symptom:** After installation, `stratum: command not found`.

**Solutions:**

1. **Restart your terminal** or source your shell configuration:
   ```bash
   # Bash
   source ~/.bashrc

   # Zsh
   source ~/.zshrc

   # Fish
   source ~/.config/fish/config.fish
   ```

2. **Manually add to PATH:**
   ```bash
   # For ~/.stratum installation, add to your shell config:
   export PATH="$HOME/.stratum/bin:$PATH"

   # For /usr/local/stratum:
   export PATH="/usr/local/stratum/bin:$PATH"
   ```

3. **Check installation location:**
   ```bash
   # Find where stratum is installed
   find / -name "stratum" -type f 2>/dev/null
   ```

### Shell Completions Not Working

**Symptom:** Tab completion doesn't work for `stratum` commands.

**Solutions:**

1. **Regenerate completions:**
   ```bash
   # Bash
   stratum completions bash > ~/.local/share/bash-completion/completions/stratum

   # Zsh
   stratum completions zsh > ~/.zfunc/_stratum

   # Fish
   stratum completions fish > ~/.config/fish/completions/stratum.fish
   ```

2. **Zsh: Ensure fpath is configured:**
   ```zsh
   # Add to ~/.zshrc
   fpath=(~/.zfunc $fpath)
   autoload -Uz compinit && compinit
   ```

3. **Bash: Ensure bash-completion is installed:**
   ```bash
   # macOS
   brew install bash-completion@2

   # Ubuntu/Debian
   sudo apt install bash-completion
   ```

### Version Conflicts

**Symptom:** Multiple versions installed, wrong version being used.

**Solutions:**

1. **Check which version is active:**
   ```bash
   stratum --version
   which stratum
   ```

2. **List installed versions:**
   ```bash
   stratum self list
   ```

3. **Switch to specific version:**
   ```bash
   stratum self use 1.2.0
   ```

4. **Check for conflicting installations:**
   ```bash
   # Look for stratum in common locations
   ls -la /usr/local/bin/stratum
   ls -la /usr/bin/stratum
   ls -la ~/.stratum/bin/stratum
   ls -la ~/.cargo/bin/stratum  # If installed via cargo
   ```

### Download or Checksum Failures

**Symptom:** Installation fails during download or with checksum mismatch.

**Solutions:**

1. **Check network connectivity:**
   ```bash
   curl -I https://github.com/horizon-analytic/stratum/releases
   ```

2. **Try a different mirror or direct download:**
   ```bash
   # Download manually
   wget https://github.com/horizon-analytic/stratum/releases/latest/download/stratum-macos-universal-VERSION.tar.gz

   # Verify checksum
   sha256sum stratum-macos-universal-VERSION.tar.gz
   ```

3. **Clear cached downloads:**
   ```bash
   rm -rf /tmp/stratum-install-*
   ```

---

## Platform-Specific Issues

### macOS: Gatekeeper Warnings

**Symptom:** "stratum cannot be opened because it is from an unidentified developer" or "stratum is damaged and can't be opened."

**Solutions:**

1. **Allow in System Preferences:**
   - Go to System Preferences > Security & Privacy > General
   - Click "Open Anyway" next to the blocked application message

2. **Remove quarantine attribute:**
   ```bash
   xattr -d com.apple.quarantine /path/to/stratum
   ```

3. **For Homebrew installations,** this shouldn't occur as bottles are signed and notarized.

### macOS: Rosetta 2 Required (Apple Silicon)

**Symptom:** On M1/M2/M3 Mac, "Bad CPU type in executable" error.

**Solutions:**

1. **Use native ARM64 binary:** The installer should auto-detect and use the correct binary. If not:
   ```bash
   curl -fsSL https://get.stratum-lang.dev | sh -s -- --arch=aarch64
   ```

2. **Install Rosetta 2** (for x86_64 binaries):
   ```bash
   softwareupdate --install-rosetta
   ```

### Linux: Missing Dependencies

**Symptom:** "error while loading shared libraries" or similar.

**Solutions:**

1. **Check glibc version:**
   ```bash
   ldd --version
   # Requires glibc 2.17 or newer
   ```

2. **Install required libraries:**
   ```bash
   # Ubuntu/Debian
   sudo apt install libc6 libssl3 ca-certificates

   # Fedora/RHEL
   sudo dnf install glibc openssl ca-certificates
   ```

3. **Use musl build for Alpine:**
   ```bash
   curl -fsSL https://get.stratum-lang.dev | sh -s -- --musl
   ```

### Linux: AppArmor/SELinux Blocking Execution

**Symptom:** Permission denied even with correct file permissions.

**Solutions:**

1. **Check SELinux status:**
   ```bash
   getenforce
   ```

2. **Allow execution:**
   ```bash
   # SELinux
   sudo chcon -t bin_t /path/to/stratum

   # Or set permissive for troubleshooting
   sudo setenforce 0
   ```

### Container Environment Detection

**Symptom:** Interactive installer hangs or behaves unexpectedly in containers.

**Solutions:**

1. **Use non-interactive mode:**
   ```bash
   curl -fsSL https://get.stratum-lang.dev | sh -s -- --yes --tier=data
   ```

2. **In Dockerfile:**
   ```dockerfile
   FROM debian:bookworm-slim
   RUN apt-get update && apt-get install -y curl ca-certificates \
       && curl -fsSL https://get.stratum-lang.dev | sh -s -- --yes --quiet \
       && rm -rf /var/lib/apt/lists/*
   ```

3. **Or use official Docker image:**
   ```dockerfile
   FROM ghcr.io/horizon-analytic/stratum:data
   ```

---

## Runtime Issues

### REPL Not Starting

**Symptom:** `stratum repl` exits immediately or shows error.

**Solutions:**

1. **Check terminal supports interactive input:**
   ```bash
   # Verify TTY
   tty
   ```

2. **Check for corrupt history:**
   ```bash
   rm ~/.stratum/history
   ```

3. **Run with verbose output:**
   ```bash
   STRATUM_LOG=debug stratum repl
   ```

### Workshop IDE Not Launching

**Symptom:** `stratum workshop` fails to open.

**Solutions:**

1. **Verify GUI tier is installed:**
   ```bash
   stratum self info
   # Should show "gui" or "full" tier
   ```

2. **Upgrade to GUI tier:**
   ```bash
   stratum self update --tier=gui
   ```

3. **Check display server (Linux):**
   ```bash
   echo $DISPLAY  # X11
   echo $WAYLAND_DISPLAY  # Wayland
   ```

4. **macOS: Grant accessibility permissions:**
   - System Preferences > Security & Privacy > Privacy > Accessibility
   - Add Stratum Workshop or Terminal

### LSP Not Connecting to Editor

**Symptom:** No IntelliSense or diagnostics in VS Code/editor.

**Solutions:**

1. **Verify LSP is installed:**
   ```bash
   which stratum-lsp
   stratum-lsp --version
   ```

2. **Check VS Code extension is installed:**
   ```bash
   stratum extension list
   ```

3. **Reinstall extension:**
   ```bash
   stratum extension uninstall
   stratum extension install
   ```

4. **Check Output panel** in VS Code for Stratum Language Server logs.

---

## Uninstallation

### What Gets Removed

When uninstalling Stratum, the following are removed by default:

| Component | Location | Removed |
|-----------|----------|---------|
| Binaries | `/usr/local/stratum/bin/` or `~/.stratum/bin/` | Yes |
| Standard Library | `<install>/lib/` | Yes |
| Shell Completions | Various (see below) | Yes |
| PATH entries | Shell config files | Yes |
| **User Configuration** | `~/.stratum/config.toml` | Only with `--purge` |
| **Installed Packages** | `~/.stratum/packages/` | Only with `--purge` |
| **REPL History** | `~/.stratum/history` | Only with `--purge` |

### Shell Completion Locations

| Shell | Completion File |
|-------|-----------------|
| Bash | `~/.local/share/bash-completion/completions/stratum` |
| Zsh | `~/.zfunc/_stratum` |
| Fish | `~/.config/fish/completions/stratum.fish` |

### Manual Cleanup

If automatic uninstall doesn't work:

```bash
# Remove binaries
rm -rf ~/.stratum
rm -rf /usr/local/stratum

# Remove shell completions
rm ~/.local/share/bash-completion/completions/stratum
rm ~/.zfunc/_stratum
rm ~/.config/fish/completions/stratum.fish

# Remove PATH entries from shell configs
# Edit ~/.bashrc, ~/.zshrc, ~/.config/fish/config.fish
# Remove lines containing STRATUM_HOME or stratum/bin

# macOS: Remove .pkg receipt
sudo pkgutil --forget dev.stratum-lang.stratum
```

### Orphaned Files After Failed Install

If installation failed partway:

```bash
# Find stratum-related files
find ~ -name "*stratum*" -type f 2>/dev/null
find /usr/local -name "*stratum*" 2>/dev/null
find /tmp -name "*stratum*" 2>/dev/null

# Clean up
rm -rf /tmp/stratum-install-*
```

---

## Getting Help

If these solutions don't resolve your issue:

1. **Check existing issues:** [GitHub Issues](https://github.com/horizon-analytic/stratum/issues)

2. **Open a new issue** with:
   - Your OS and version (`uname -a`)
   - Stratum version (`stratum --version`)
   - Full error message
   - Steps to reproduce

3. **Enable debug logging:**
   ```bash
   STRATUM_LOG=debug stratum <command>
   ```

4. **Generate diagnostic report:**
   ```bash
   stratum self diagnose > stratum-diagnostics.txt
   ```
