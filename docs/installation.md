# Installation Guide

This guide covers all installation methods for Stratum, including system requirements, platform-specific instructions, and upgrade procedures.

## System Requirements

### Minimum Requirements

| Component | Requirement |
|-----------|-------------|
| **OS** | macOS 11+, Linux (glibc 2.17+), or Alpine Linux |
| **Architecture** | x86_64 or ARM64 (aarch64) |
| **Disk Space** | 15 MB (Core) to 120 MB (Full) |
| **RAM** | 512 MB minimum, 2 GB recommended |

### Platform Support

| Platform | Architecture | Status |
|----------|--------------|--------|
| macOS 11+ (Big Sur) | Apple Silicon (arm64) | Fully supported |
| macOS 11+ (Big Sur) | Intel (x86_64) | Fully supported |
| Linux (glibc 2.17+) | x86_64 | Fully supported |
| Linux (glibc 2.17+) | ARM64 | Fully supported |
| Alpine Linux (musl) | x86_64 | Fully supported |
| Windows | x86_64 | Coming soon (Phase 14) |

## Installation Tiers

Stratum offers tiered installation to minimize download size for your use case:

| Tier | Size | Includes |
|------|------|----------|
| **Core** | ~15 MB | CLI, REPL, compiler, type checker, bytecode VM |
| **Data** | ~45 MB | Core + DataFrame, Arrow integration, SQL support |
| **GUI** | ~80 MB | Data + GUI framework (iced), native widgets |
| **Full** | ~120 MB | GUI + Workshop IDE, LSP, DAP debugger |

**Recommended:** Most users should install the **Data** tier (default), which includes the powerful DataFrame operations that make Stratum ideal for data work.

---

## Quick Install (Interactive)

The recommended way to install Stratum is using the interactive installer:

```bash
curl -fsSL https://get.stratum-lang.dev | sh
```

This will:
1. Detect your platform and architecture
2. Prompt you to select an installation tier
3. Download and extract the appropriate binaries
4. Configure your PATH
5. Install shell completions for your shell(s)

### Non-Interactive Installation

For scripted or automated installations:

```bash
# Install with all defaults (Data tier, default location)
curl -fsSL https://get.stratum-lang.dev | sh -s -- --yes

# Install specific tier
curl -fsSL https://get.stratum-lang.dev | sh -s -- --yes --tier=full

# Custom installation directory
curl -fsSL https://get.stratum-lang.dev | sh -s -- --yes --prefix=/opt/stratum

# Skip PATH modification (configure manually)
curl -fsSL https://get.stratum-lang.dev | sh -s -- --yes --no-path

# Skip shell completions
curl -fsSL https://get.stratum-lang.dev | sh -s -- --yes --no-completions

# Silent mode (minimal output)
curl -fsSL https://get.stratum-lang.dev | sh -s -- --yes --quiet
```

---

## Homebrew (macOS / Linux)

Stratum is available via Homebrew for both macOS and Linux.

### Add the Tap

```bash
brew tap horizon-analytic/stratum
```

### Install

```bash
# Install default tier (Data)
brew install stratum

# Install with GUI framework
brew install stratum --with-gui

# Install full version (includes Workshop IDE)
brew install stratum --with-full
```

### Upgrade

```bash
brew update
brew upgrade stratum
```

### Uninstall

```bash
brew uninstall stratum
```

---

## Docker

Official Docker images are available for containerized workflows. Images support both amd64 and arm64 architectures.

```bash
# Run a Stratum script
docker run --rm -v $(pwd):/app ghcr.io/horizon-analytic/stratum run /app/script.strat

# Start interactive REPL
docker run --rm -it ghcr.io/horizon-analytic/stratum repl

# Evaluate an expression
docker run --rm ghcr.io/horizon-analytic/stratum eval "1 + 2 * 3"

# Use specific version
docker run --rm -it ghcr.io/horizon-analytic/stratum:1.0.0 repl
```

### Available Tags

| Tag | Description |
|-----|-------------|
| `stratum:latest` | Latest stable, Data tier (recommended) |
| `stratum:core` | Core tier only (Alpine-based, ~50 MB) |
| `stratum:data` | Data tier (same as core, ~50 MB) |
| `stratum:full` | Full tier with LSP server (Debian-based, ~100 MB) |
| `stratum:X.Y.Z` | Specific version |
| `stratum:X.Y.Z-full` | Specific version with LSP |

### Using as Base Image

```dockerfile
FROM ghcr.io/horizon-analytic/stratum:data

WORKDIR /app
COPY . .

CMD ["stratum", "run", "main.strat"]
```

### LSP Server in Docker

The `full` image includes the LSP server for editor integration:

```bash
# Start LSP server (connect from editor)
docker run --rm -i ghcr.io/horizon-analytic/stratum:full lsp --stdio
```

---

## Linux Package Managers

### Debian / Ubuntu (.deb)

```bash
# Download the package
wget https://github.com/horizon-analytic/stratum/releases/latest/download/stratum_VERSION_amd64.deb

# Install
sudo dpkg -i stratum_VERSION_amd64.deb

# Or use apt (resolves dependencies automatically)
sudo apt install ./stratum_VERSION_amd64.deb
```

### Fedora / RHEL (.rpm)

```bash
# Download the package
wget https://github.com/horizon-analytic/stratum/releases/latest/download/stratum-VERSION-1.x86_64.rpm

# Install
sudo rpm -i stratum-VERSION-1.x86_64.rpm

# Or use dnf
sudo dnf install ./stratum-VERSION-1.x86_64.rpm
```

---

## macOS .pkg Installer

A signed and notarized .pkg installer is available for macOS:

```bash
# Download
wget https://github.com/horizon-analytic/stratum/releases/latest/download/stratum-VERSION-macos.pkg

# Install (opens GUI installer)
open stratum-VERSION-macos.pkg

# Or install via command line
sudo installer -pkg stratum-VERSION-macos.pkg -target /
```

---

## Manual Installation (Tarball)

For manual installation or custom setups:

```bash
# Download appropriate tarball
# macOS Universal:
wget https://github.com/horizon-analytic/stratum/releases/latest/download/stratum-macos-universal-VERSION.tar.gz

# Linux x86_64:
wget https://github.com/horizon-analytic/stratum/releases/latest/download/stratum-linux-x86_64-VERSION.tar.gz

# Linux ARM64:
wget https://github.com/horizon-analytic/stratum/releases/latest/download/stratum-linux-aarch64-VERSION.tar.gz

# Linux musl (Alpine):
wget https://github.com/horizon-analytic/stratum/releases/latest/download/stratum-linux-x86_64-musl-VERSION.tar.gz

# Extract
tar -xzf stratum-*.tar.gz

# Move to desired location
sudo mv stratum /usr/local/bin/

# Verify installation
stratum --version
```

---

## Shell Completions

Shell completions are installed automatically by the interactive installer. To install manually:

### Bash

```bash
stratum completions bash > ~/.local/share/bash-completion/completions/stratum
```

Or add to your `.bashrc`:

```bash
eval "$(stratum completions bash)"
```

### Zsh

```bash
stratum completions zsh > ~/.zfunc/_stratum
```

Ensure `~/.zfunc` is in your `fpath` (add to `.zshrc`):

```zsh
fpath=(~/.zfunc $fpath)
autoload -Uz compinit && compinit
```

### Fish

```bash
stratum completions fish > ~/.config/fish/completions/stratum.fish
```

---

## Upgrading

### Using the CLI

```bash
# Check for updates
stratum self update --check

# Update to latest version
stratum self update

# Update to specific version
stratum self update 1.2.0

# Upgrade tier (e.g., from Data to Full)
stratum self update --tier=full
```

### Version Management

Stratum supports multiple versions side-by-side:

```bash
# Install a specific version
stratum self install 1.1.0

# List installed versions
stratum self list

# Switch active version
stratum self use 1.1.0

# Use specific version for a single command
stratum +1.1.0 run script.strat
```

### Upgrading via Package Managers

```bash
# Homebrew
brew update && brew upgrade stratum

# apt
sudo apt update && sudo apt upgrade stratum

# dnf
sudo dnf upgrade stratum
```

---

## Uninstallation

### Using the CLI

```bash
# Remove Stratum (preserves user configuration)
stratum self uninstall

# Remove everything including configuration
stratum self uninstall --purge
```

### Package Managers

```bash
# Homebrew
brew uninstall stratum

# apt
sudo apt remove stratum      # Keep config
sudo apt purge stratum       # Remove config

# dnf
sudo dnf remove stratum
```

### Manual Uninstallation

If the `stratum` binary is unavailable:

```bash
curl -fsSL https://get.stratum-lang.dev/uninstall.sh | sh
```

For details on what gets removed, see [Troubleshooting: Uninstallation](troubleshooting.md#uninstallation).

---

## Configuration

Stratum stores configuration in:

| Platform | Location |
|----------|----------|
| macOS/Linux | `~/.stratum/config.toml` |
| Windows | `%APPDATA%\stratum\config.toml` |

### Default Configuration

```toml
# ~/.stratum/config.toml

[general]
# Default tier for new installations
tier = "data"

[repl]
# REPL history file location
history_file = "~/.stratum/history"
# Maximum history entries
history_size = 10000

[compiler]
# Default optimization level (0-3)
opt_level = 2
# Enable debug symbols
debug = false

[formatter]
# Indentation style
indent = 4
# Line width
line_width = 100
```

---

## Verifying Installation

After installation, verify everything is working:

```bash
# Check version
stratum --version

# Run a quick test
stratum -c 'print("Hello, Stratum!")'

# Start the REPL
stratum repl

# Check installed tier
stratum self info
```

---

## Next Steps

- [Quick Start Guide](index.md) - Write your first Stratum program
- [Standard Library Reference](stdlib/index.md) - Explore built-in functionality
- [Troubleshooting](troubleshooting.md) - Common issues and solutions
