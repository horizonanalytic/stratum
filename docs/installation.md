# Installation Guide

This guide covers all installation methods for Stratum.

## System Requirements

### Minimum Requirements

| Component | Requirement |
|-----------|-------------|
| **OS** | macOS 11+, Linux (glibc 2.17+), or Windows 10+ |
| **Architecture** | x86_64 or ARM64 (aarch64) |
| **Disk Space** | ~150 MB |
| **RAM** | 512 MB minimum, 2 GB recommended |

### Platform Support

| Platform | Architecture | Status |
|----------|--------------|--------|
| macOS 11+ (Big Sur) | Apple Silicon (arm64) | Fully supported |
| macOS 11+ (Big Sur) | Intel (x86_64) | Fully supported |
| Linux (glibc 2.17+) | x86_64 | Fully supported |
| Linux (glibc 2.17+) | ARM64 | Fully supported |
| Windows 10+ | x86_64 | Fully supported |

---

## Download Pre-built Binaries (Recommended)

Pre-built binaries are available for all supported platforms:

**[Download Stratum](https://horizonanalytic.com/landing/packages/horizon-stratum)**

1. Visit the download page and select the appropriate binary for your platform
2. Extract the archive to your preferred location
3. Add the binary location to your PATH

### macOS / Linux

```bash
# Extract (adjust filename for your platform)
tar -xzf stratum-macos-arm64.tar.gz

# Move to a location in your PATH
sudo mv stratum /usr/local/bin/

# Verify installation
stratum --version
```

### Windows

1. Extract the `.zip` file
2. Move `stratum.exe` to a directory in your PATH (e.g., `C:\Program Files\Stratum\`)
3. Or add the extraction directory to your PATH environment variable

```powershell
# Verify installation
stratum --version
```

---

## Build from Source

Building from source requires Rust 1.75 or later.

### Prerequisites

Install Rust via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Build Steps

```bash
# Clone the repository
git clone https://github.com/horizonanalytic/stratum.git
cd stratum

# Build in release mode
cargo build --release

# The binary will be at target/release/stratum
./target/release/stratum --version

# Optionally install to your Cargo bin directory
cargo install --path crates/stratum-cli
```

### Build Options

```bash
# Build only the CLI (faster, smaller)
cargo build --release --bin stratum

# Build with all features
cargo build --release --all

# Run tests
cargo test --all
```

---

## Shell Completions

Generate shell completions for your preferred shell:

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

### PowerShell

```powershell
stratum completions powershell >> $PROFILE
```

---

## Coming Soon

The following installation methods are planned for future releases:

- **Homebrew** - `brew install stratum` (in progress)
- **Docker** - Official container images
- **Linux packages** - `.deb` and `.rpm` packages

---

## Verifying Installation

After installation, verify everything is working:

```bash
# Check version
stratum --version

# Run a quick test
stratum eval "1 + 2 * 3"

# Start the REPL
stratum repl

# Run a script
stratum run hello.strat
```

---

## Troubleshooting

### "command not found" after installation

Ensure the Stratum binary is in your PATH:

```bash
# Check if stratum is in PATH
which stratum

# If not found, add it to your shell profile
# For bash (~/.bashrc) or zsh (~/.zshrc):
export PATH="/path/to/stratum:$PATH"
```

### Permission denied on macOS

macOS may block unsigned binaries. To allow execution:

```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine /path/to/stratum
```

Or go to **System Preferences > Security & Privacy** and click "Allow Anyway".

### Build failures

If building from source fails:

1. Ensure Rust 1.75+ is installed: `rustc --version`
2. Update Rust: `rustup update`
3. Clean and rebuild: `cargo clean && cargo build --release`

---

## Next Steps

- [Examples](examples/index.md) - Learn from example programs
- [Standard Library Reference](stdlib/index.md) - Explore built-in functionality
