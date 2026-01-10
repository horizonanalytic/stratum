# Troubleshooting

This guide covers common issues encountered when installing or running Stratum.

## Installation Issues

### "command not found" After Installation

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
   # Add to your shell config (~/.bashrc, ~/.zshrc, etc.):
   export PATH="/path/to/stratum:$PATH"
   ```

3. **Verify where stratum is installed:**
   ```bash
   # Check common locations
   ls -la /usr/local/bin/stratum
   ls -la ~/.cargo/bin/stratum
   which stratum
   ```

### Permission Denied Errors

**Symptom:** Installation fails with "Permission denied" errors.

**Solutions:**

1. **Install to user directory:**
   ```bash
   # When building from source
   cargo install --path crates/stratum-cli
   # Installs to ~/.cargo/bin/
   ```

2. **Use sudo for system-wide installation:**
   ```bash
   sudo mv stratum /usr/local/bin/
   ```

3. **Fix directory permissions:**
   ```bash
   sudo chown -R $(whoami) /usr/local/bin/stratum
   ```

### Shell Completions Not Working

**Symptom:** Tab completion doesn't work for `stratum` commands.

**Solutions:**

1. **Generate completions:**
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

### Build from Source Failures

**Symptom:** `cargo build` fails with errors.

**Solutions:**

1. **Ensure Rust 1.75+ is installed:**
   ```bash
   rustc --version
   # Should be 1.75.0 or higher
   ```

2. **Update Rust:**
   ```bash
   rustup update
   ```

3. **Clean and rebuild:**
   ```bash
   cargo clean
   cargo build --release
   ```

4. **Check for missing system dependencies:**
   ```bash
   # Ubuntu/Debian
   sudo apt install build-essential pkg-config libssl-dev

   # macOS (with Xcode command line tools)
   xcode-select --install
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

### macOS: "Bad CPU type in executable" (Apple Silicon)

**Symptom:** On M1/M2/M3 Mac, "Bad CPU type in executable" error.

**Solutions:**

1. **Download the correct binary:** Ensure you downloaded the ARM64/aarch64 version for Apple Silicon Macs.

2. **Build from source:** Building from source will automatically compile for your architecture:
   ```bash
   cargo build --release
   ```

3. **Install Rosetta 2** (for x86_64 binaries):
   ```bash
   softwareupdate --install-rosetta
   ```

### Linux: Missing Shared Libraries

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

### Linux: SELinux Blocking Execution

**Symptom:** Permission denied even with correct file permissions.

**Solutions:**

1. **Check SELinux status:**
   ```bash
   getenforce
   ```

2. **Allow execution:**
   ```bash
   sudo chcon -t bin_t /path/to/stratum
   ```

### Windows: Execution Policy

**Symptom:** PowerShell blocks running stratum.

**Solutions:**

1. **Run from Command Prompt** instead of PowerShell.

2. **Or adjust execution policy:**
   ```powershell
   Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
   ```

---

## Runtime Issues

### REPL Not Starting

**Symptom:** `stratum repl` exits immediately or shows error.

**Solutions:**

1. **Check terminal supports interactive input:**
   ```bash
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

1. **Check display server (Linux):**
   ```bash
   echo $DISPLAY  # X11
   echo $WAYLAND_DISPLAY  # Wayland
   ```

2. **macOS: Grant accessibility permissions:**
   - System Preferences > Security & Privacy > Privacy > Accessibility
   - Add Terminal or your terminal emulator

3. **Install required GUI libraries (Linux):**
   ```bash
   # Ubuntu/Debian
   sudo apt install libgtk-3-0 libwebkit2gtk-4.0-37
   ```

### Script Execution Errors

**Symptom:** Script fails with unexpected errors.

**Solutions:**

1. **Check syntax:**
   ```bash
   stratum fmt --check script.strat
   ```

2. **Run with debug output:**
   ```bash
   STRATUM_LOG=debug stratum run script.strat
   ```

3. **Test in REPL:** Copy problematic code sections to the REPL to isolate issues.

---

## Uninstallation

### Manual Cleanup

To completely remove Stratum:

```bash
# Remove binary
rm /usr/local/bin/stratum
# Or if installed via cargo:
rm ~/.cargo/bin/stratum

# Remove configuration and data
rm -rf ~/.stratum

# Remove shell completions
rm -f ~/.local/share/bash-completion/completions/stratum
rm -f ~/.zfunc/_stratum
rm -f ~/.config/fish/completions/stratum.fish

# Remove PATH entries from shell configs
# Edit ~/.bashrc, ~/.zshrc, or ~/.config/fish/config.fish
# Remove lines containing stratum
```

---

## Getting Help

If these solutions don't resolve your issue:

1. **Check existing issues:** [GitHub Issues](https://github.com/horizonanalytic/stratum/issues)

2. **Open a new issue** with:
   - Your OS and version (`uname -a` or Windows version)
   - Stratum version (`stratum --version`)
   - Full error message
   - Steps to reproduce

3. **Enable debug logging:**
   ```bash
   STRATUM_LOG=debug stratum <command>
   ```
