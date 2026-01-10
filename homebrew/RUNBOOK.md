# Homebrew Deployment Runbook

This runbook describes how to deploy new versions of Stratum to Homebrew.

## Prerequisites

Before your first deployment, complete this one-time setup:

### 1. Create the Homebrew Tap Repository

```bash
# Create a new repository on GitHub named: homebrew-stratum
# Under the organization: horizon-analytic

# Clone and set up the tap repository
git clone https://github.com/horizon-analytic/homebrew-stratum.git
cd homebrew-stratum

# Copy the formula and README from the main repo
cp /path/to/stratum/homebrew/Formula/stratum.rb Formula/
cp /path/to/stratum/homebrew/README.md .

git add .
git commit -m "Initial formula setup"
git push
```

### 2. Set Up GitHub Secrets

Add the following secret to the main Stratum repository (Settings > Secrets and variables > Actions):

| Secret Name | Description |
|-------------|-------------|
| `HOMEBREW_TAP_TOKEN` | A GitHub Personal Access Token with `repo` scope for the homebrew-stratum repository |

To create the token:
1. Go to GitHub Settings > Developer settings > Personal access tokens > Fine-grained tokens
2. Create a new token with:
   - Repository access: Only select repositories > `horizon-analytic/homebrew-stratum`
   - Permissions: Contents (Read and write)
3. Copy the token and add it as `HOMEBREW_TAP_TOKEN` secret

---

## Release Process

### Using the Release Script (Recommended)

The `scripts/release.sh` script handles the complete release process. It reads the version from `Cargo.toml` (single source of truth).

```bash
# Full release (builds for all platforms, signs, notarizes, publishes)
./scripts/release.sh

# Local build only (current platform, for testing)
./scripts/release.sh --local

# Dry run (build but don't publish)
./scripts/release.sh --dry-run

# Quick local test
./scripts/release.sh --local --skip-tests --dry-run
```

The script will:
1. Read version from `Cargo.toml`
2. Run tests (unless `--skip-tests`)
3. Build for all platforms (or just local with `--local`)
4. Create macOS universal binary
5. Sign with Developer ID certificates (Application + Installer)
6. Notarize with Apple (using keychain profile)
7. Create .pkg installer, .deb, .rpm, and tarballs
8. Publish to GitHub Releases (unless `--dry-run`)

### Prerequisites for macOS Signing

1. **Developer ID Certificates** (from Apple Developer portal):
   - Developer ID Application (for signing binaries)
   - Developer ID Installer (for signing .pkg files)

2. **Notarization credentials** stored in keychain:
   ```bash
   xcrun notarytool store-credentials stratum-notarization \
     --apple-id YOUR_APPLE_ID \
     --team-id YOUR_TEAM_ID \
     --password YOUR_APP_SPECIFIC_PASSWORD
   ```

### Step-by-Step Release

1. **Update version in Cargo.toml**
   ```bash
   # Edit Cargo.toml and update [workspace.package] version
   vim Cargo.toml
   ```

2. **Update CHANGELOG.md**

3. **Commit version bump**
   ```bash
   git add Cargo.toml CHANGELOG.md
   git commit -m "Bump version to X.Y.Z"
   ```

4. **Run release script**
   ```bash
   ./scripts/release.sh
   ```

5. **Create Git tag** (if not done by script)
   ```bash
   VERSION=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
   git tag -a "v$VERSION" -m "Release v$VERSION"
   git push origin "v$VERSION"
   ```

6. **Verify**
   ```bash
   brew update
   brew upgrade stratum
   stratum --version
   ```

---

## Automated Homebrew Update

The Homebrew formula is automatically updated when you publish a GitHub Release.

### How It Works

1. Publishing a release triggers `homebrew-update.yml` workflow
2. Workflow downloads the release tarball and calculates SHA256
3. Updates the formula in `horizon-analytic/homebrew-stratum`
4. The `homebrew-bottles.yml` workflow builds pre-compiled bottles

### Verify Automation

After publishing:
- Check Actions tab for workflow status
- Verify the homebrew-stratum repository was updated
- Test: `brew update && brew upgrade stratum`

---

## Manual Deployment

Use this process if automation fails or for emergency updates.

### Steps

1. **Calculate SHA256 Hash**
   ```bash
   VERSION="0.2.0"
   curl -sL "https://github.com/horizon-analytic/stratum/archive/refs/tags/v${VERSION}.tar.gz" \
     | shasum -a 256
   ```

2. **Update the Formula**
   ```bash
   cd /path/to/homebrew-stratum

   # Edit Formula/stratum.rb
   # Update the URL to new version
   # Update sha256 to the hash calculated above
   ```

   Example changes in `stratum.rb`:
   ```ruby
   url "https://github.com/horizon-analytic/stratum/archive/refs/tags/v0.2.0.tar.gz"
   sha256 "abc123..."  # Your calculated hash
   ```

3. **Test Locally**
   ```bash
   # Install from local formula
   brew install --build-from-source ./Formula/stratum.rb

   # Verify
   stratum --version
   stratum completions bash > /dev/null  # Should succeed
   ```

4. **Push Changes**
   ```bash
   git add Formula/stratum.rb
   git commit -m "stratum 0.2.0"
   git push
   ```

5. **Verify Public Install**
   ```bash
   brew update
   brew upgrade stratum
   ```

---

## Troubleshooting

### Build Failures

**Symptom:** Formula fails to build
```bash
# Get verbose output
brew install --verbose --debug stratum

# Common fixes:
# 1. Ensure Rust toolchain works
rustc --version
cargo --version

# 2. Check for missing dependencies
brew doctor
```

**Symptom:** SHA256 mismatch
```bash
# Re-download and hash
VERSION="0.2.0"
curl -sL "https://github.com/horizon-analytic/stratum/archive/refs/tags/v${VERSION}.tar.gz" \
  -o stratum.tar.gz
shasum -a 256 stratum.tar.gz
```

### Completion Issues

**Symptom:** Shell completions not working after install
```bash
# Verify completions were installed
ls $(brew --prefix)/share/zsh/site-functions/_stratum
ls $(brew --prefix)/etc/bash_completion.d/stratum
ls $(brew --prefix)/share/fish/vendor_completions.d/stratum.fish

# Regenerate if missing
stratum completions bash > $(brew --prefix)/etc/bash_completion.d/stratum
stratum completions zsh > $(brew --prefix)/share/zsh/site-functions/_stratum
stratum completions fish > $(brew --prefix)/share/fish/vendor_completions.d/stratum.fish
```

### Rollback

To revert to a previous version:

```bash
# Find previous version
brew log stratum

# Checkout specific formula version
cd $(brew --repository horizon-analytic/homebrew-stratum)
git log --oneline Formula/stratum.rb
git checkout <commit-hash> -- Formula/stratum.rb

# Reinstall
brew reinstall stratum
```

---

## Pre-Release Testing

Before releasing a new version, test the formula locally:

```bash
# Build from source tarball
VERSION="0.2.0"
brew install --build-from-source \
  "https://github.com/horizon-analytic/stratum/archive/refs/tags/v${VERSION}.tar.gz"

# Run formula tests
brew test stratum

# Audit the formula
brew audit --strict stratum
```

---

## Checklist

### Before Release
- [ ] All tests pass (`cargo test`)
- [ ] Clippy clean (`cargo clippy`)
- [ ] Version bumped in `Cargo.toml`
- [ ] CHANGELOG updated
- [ ] Git tag created and pushed

### After Release
- [ ] GitHub Release published
- [ ] Homebrew workflow succeeded
- [ ] `brew update && brew upgrade stratum` works
- [ ] `stratum --version` shows correct version
- [ ] Shell completions work

### Quarterly Maintenance
- [ ] Review and update formula dependencies
- [ ] Check for Homebrew policy changes
- [ ] Verify formula passes `brew audit --strict`
- [ ] Consider submitting to homebrew-core if eligible

---

## Bottle Building

Bottles are pre-built binary packages that allow users to install Stratum without compiling from source.

### Automated Bottle Building

The `homebrew-bottles.yml` workflow automatically builds bottles when a release is published:

1. **Platforms built:**
   - macOS ARM64 (arm64_sonoma) - Apple Silicon Macs
   - macOS x86_64 (sonoma) - Intel Macs
   - Linux x86_64 (x86_64_linux) - Linux systems

2. **Process:**
   - Builds Stratum on each platform using `brew install --build-bottle`
   - Creates bottle archives using `brew bottle`
   - Uploads bottles to the GitHub Release as assets
   - Updates the formula with bottle SHA256 hashes

3. **Verification:**
   After bottles are built, verify they work:
   ```bash
   # This should download the pre-built bottle instead of compiling
   brew install horizon-analytic/stratum/stratum

   # Verify bottle was used (check for "Pouring" in output)
   brew info stratum
   ```

### Manual Bottle Building

If automation fails, build bottles manually:

```bash
# Install from source with bottle flag
brew install --build-bottle horizon-analytic/stratum/stratum

# Create the bottle
cd $(brew --cache)
brew bottle --json horizon-analytic/stratum/stratum

# Upload the resulting .tar.gz to the GitHub Release
# Then update the formula with the SHA256 hash
```

### Tiered Bottle Considerations

Note: Bottles are built with the default tier (Data). Users who want different tiers
(GUI or Full) will need to build from source:

```bash
# This will compile from source with GUI support
brew install stratum --with-gui
```

---

## Submitting to Homebrew Core

Once Stratum meets the requirements (50+ stars, stable release, active maintenance), submit to homebrew-core:

1. Fork homebrew-core
2. Add formula to `Formula/s/stratum.rb`
3. Run `brew audit --new --strict stratum`
4. Submit PR with clear description
5. Address reviewer feedback

Note: homebrew-core formulas are auto-updated by Homebrew's CI, so manual updates are rarely needed after acceptance.

---

## Contacts

- **Formula Issues:** Open an issue on horizon-analytic/homebrew-stratum
- **Stratum Issues:** Open an issue on horizon-analytic/stratum
- **Homebrew Help:** https://docs.brew.sh/
