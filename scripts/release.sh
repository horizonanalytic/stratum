#!/usr/bin/env bash
#
# Stratum Release Script
# ======================
# Local script to build and publish releases without requiring GitHub Actions.
# Reads version from Cargo.toml (single source of truth).
#
# Usage:
#   ./scripts/release.sh [OPTIONS]
#
# Examples:
#   ./scripts/release.sh                  # Full release (version from Cargo.toml)
#   ./scripts/release.sh --dry-run        # Build only, don't publish
#   ./scripts/release.sh --local          # Build for current platform only
#   ./scripts/release.sh --local --dry-run --skip-tests  # Quick local test
#
# Prerequisites (macOS):
#   1. Install "Developer ID Application" and "Developer ID Installer" certificates
#   2. Store notarization credentials in keychain:
#      xcrun notarytool store-credentials stratum-notarization \
#        --apple-id YOUR_APPLE_ID \
#        --team-id YOUR_TEAM_ID \
#        --password YOUR_APP_SPECIFIC_PASSWORD
#
# Environment Variables:
#   SIGNING_IDENTITY         - Override auto-detected signing identity
#   NOTARIZATION_PROFILE     - Keychain profile for notarization (default: auto-detect)
#   GITHUB_TOKEN             - GitHub token for creating releases
#   SKIP_NOTARIZE            - Set to 1 to skip notarization
#   SKIP_SIGN                - Set to 1 to skip code signing
#   S3_BUCKET                - S3 bucket for uploads (default: horizon-analytic-public)
#   HORIZON_API_URL          - Horizon Analytic backend URL (default: https://api.horizonanalytic.com)
#   HORIZON_ADMIN_API_KEY    - Admin API key for backend registration

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BINARY_NAME="stratum"
BUILD_DIR="$PROJECT_ROOT/target/release-build"
ARTIFACTS_DIR="$BUILD_DIR/artifacts"

# S3 and Backend Configuration
S3_BUCKET="${S3_BUCKET:-horizon-analytic-public}"
S3_PREFIX="packages/horizon-stratum"
HORIZON_API_URL="${HORIZON_API_URL:-https://api.horizonanalytic.com}"
PRODUCT_SLUG="horizon-stratum"

# Parse arguments
DRY_RUN=false
LOCAL_ONLY=false
SKIP_TESTS=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --local)
            LOCAL_ONLY=true
            shift
            ;;
        --skip-tests)
            SKIP_TESTS=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Build and publish a Stratum release. Version is read from Cargo.toml."
            echo ""
            echo "Options:"
            echo "  --dry-run      Build only, don't publish to GitHub"
            echo "  --local        Build for current platform only"
            echo "  --skip-tests   Skip running tests"
            echo "  --help, -h     Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  SIGNING_IDENTITY      Override signing identity"
            echo "  NOTARIZATION_PROFILE  Keychain profile for notarization"
            echo "  GITHUB_TOKEN          GitHub token for releases"
            echo "  SKIP_SIGN=1           Skip code signing"
            echo "  SKIP_NOTARIZE=1       Skip notarization"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Read version from Cargo.toml
VERSION=$(grep -m1 '^version' "$PROJECT_ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')

if [[ -z "$VERSION" ]]; then
    echo -e "${RED}Could not read version from Cargo.toml${NC}"
    exit 1
fi

# Validate version format
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-.*)?$ ]]; then
    echo -e "${RED}Invalid version format in Cargo.toml: $VERSION${NC}"
    echo "Expected format: X.Y.Z or X.Y.Z-prerelease"
    exit 1
fi

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo ""
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}  $1${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

# Detect platform
detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Darwin)
            case "$arch" in
                arm64)  echo "aarch64-apple-darwin" ;;
                x86_64) echo "x86_64-apple-darwin" ;;
                *)      log_error "Unsupported macOS architecture: $arch"; exit 1 ;;
            esac
            ;;
        Linux)
            case "$arch" in
                x86_64)  echo "x86_64-unknown-linux-gnu" ;;
                aarch64) echo "aarch64-unknown-linux-gnu" ;;
                *)       log_error "Unsupported Linux architecture: $arch"; exit 1 ;;
            esac
            ;;
        *)
            log_error "Unsupported OS: $os"
            exit 1
            ;;
    esac
}

# Check required tools
check_requirements() {
    log_step "Checking Requirements"

    local missing=()

    command -v cargo >/dev/null 2>&1 || missing+=("cargo")
    command -v git >/dev/null 2>&1 || missing+=("git")
    command -v tar >/dev/null 2>&1 || missing+=("tar")
    command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1 || missing+=("sha256sum or shasum")

    if [[ "$(uname -s)" == "Darwin" ]]; then
        command -v lipo >/dev/null 2>&1 || missing+=("lipo")
        command -v codesign >/dev/null 2>&1 || missing+=("codesign")
        command -v pkgbuild >/dev/null 2>&1 || missing+=("pkgbuild")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing[*]}"
        exit 1
    fi

    # Check for cross if not local-only
    if [[ "$LOCAL_ONLY" == "false" ]]; then
        if ! command -v cross >/dev/null 2>&1; then
            log_warn "cross not installed. Installing for cross-compilation..."
            cargo install cross --git https://github.com/cross-rs/cross
        fi
    fi

    log_success "All requirements satisfied"
}

# Run tests
run_tests() {
    if [[ "$SKIP_TESTS" == "true" ]]; then
        log_warn "Skipping tests (--skip-tests)"
        return
    fi

    log_step "Running Tests"

    cd "$PROJECT_ROOT"

    log_info "Running cargo test..."
    cargo test --all

    log_info "Checking formatting..."
    cargo fmt --all -- --check

    log_info "Running clippy..."
    cargo clippy --all -- -D warnings

    log_success "All tests passed"
}

# Build for a specific target
build_target() {
    local target="$1"
    local use_cross="${2:-false}"

    log_info "Building for $target..."

    cd "$PROJECT_ROOT"

    if [[ "$use_cross" == "true" ]]; then
        cross build --release --target "$target" -p stratum-cli
    else
        cargo build --release --target "$target" -p stratum-cli
    fi

    # Create artifact directory
    local artifact_dir="$ARTIFACTS_DIR/$target"
    mkdir -p "$artifact_dir"
    mkdir -p "$artifact_dir/completions"

    # Copy binary
    cp "target/$target/release/$BINARY_NAME" "$artifact_dir/"

    # Generate completions (may fail for cross-compiled binaries)
    if [[ "$use_cross" == "false" ]]; then
        "$artifact_dir/$BINARY_NAME" completions bash > "$artifact_dir/completions/stratum.bash" 2>/dev/null || true
        "$artifact_dir/$BINARY_NAME" completions zsh > "$artifact_dir/completions/_stratum" 2>/dev/null || true
        "$artifact_dir/$BINARY_NAME" completions fish > "$artifact_dir/completions/stratum.fish" 2>/dev/null || true
    fi

    log_success "Built $target"
}

# Build all targets
build_all() {
    log_step "Building Release Binaries"

    mkdir -p "$BUILD_DIR"
    mkdir -p "$ARTIFACTS_DIR"

    local current_target
    current_target=$(detect_platform)

    if [[ "$LOCAL_ONLY" == "true" ]]; then
        log_info "Building for local platform only: $current_target"
        build_target "$current_target" false
    else
        # Native builds
        log_info "Building native targets..."

        if [[ "$(uname -s)" == "Darwin" ]]; then
            # macOS: build both architectures
            build_target "aarch64-apple-darwin" false
            build_target "x86_64-apple-darwin" false
        else
            # Linux: build native
            build_target "$current_target" false
        fi

        # Cross-compiled builds
        log_info "Building cross-compiled targets..."

        if [[ "$(uname -s)" == "Darwin" ]]; then
            # From macOS, cross-compile to Linux
            build_target "x86_64-unknown-linux-gnu" true
            build_target "aarch64-unknown-linux-gnu" true
            build_target "x86_64-unknown-linux-musl" true
        else
            # From Linux, cross-compile to macOS (requires osxcross)
            if command -v x86_64-apple-darwin-gcc >/dev/null 2>&1; then
                log_warn "macOS cross-compilation not available. Skipping."
            fi
        fi
    fi

    log_success "All builds complete"
}

# Create macOS universal binary
create_universal_binary() {
    if [[ "$(uname -s)" != "Darwin" ]]; then
        return
    fi

    log_step "Creating macOS Universal Binary"

    local universal_dir="$ARTIFACTS_DIR/universal"
    mkdir -p "$universal_dir/completions"

    # Check if both architectures are available
    if [[ -f "$ARTIFACTS_DIR/aarch64-apple-darwin/$BINARY_NAME" ]] && \
       [[ -f "$ARTIFACTS_DIR/x86_64-apple-darwin/$BINARY_NAME" ]]; then

        lipo -create \
            "$ARTIFACTS_DIR/aarch64-apple-darwin/$BINARY_NAME" \
            "$ARTIFACTS_DIR/x86_64-apple-darwin/$BINARY_NAME" \
            -output "$universal_dir/$BINARY_NAME"

        # Copy completions from arm64 build
        cp -r "$ARTIFACTS_DIR/aarch64-apple-darwin/completions/"* "$universal_dir/completions/" 2>/dev/null || true

        log_info "Universal binary info:"
        lipo -info "$universal_dir/$BINARY_NAME"

        log_success "Created universal binary"
    else
        log_warn "Both architectures not available, skipping universal binary"
    fi
}

# Auto-detect signing identity (prioritizes Developer ID Application)
detect_signing_identity() {
    if [[ -n "${SIGNING_IDENTITY:-}" ]]; then
        echo "$SIGNING_IDENTITY"
        return
    fi

    # Priority: Developer ID Application > Apple Distribution > Apple Development
    local identity

    # Try Developer ID Application first (for distribution outside App Store)
    identity=$(security find-identity -v -p codesigning 2>/dev/null | \
        grep "Developer ID Application" | \
        head -1 | \
        sed 's/.*"\(.*\)".*/\1/')

    if [[ -n "$identity" ]]; then
        echo "$identity"
        return
    fi

    # Fall back to Apple Distribution (App Store)
    identity=$(security find-identity -v -p codesigning 2>/dev/null | \
        grep "Apple Distribution" | \
        head -1 | \
        sed 's/.*"\(.*\)".*/\1/')

    if [[ -n "$identity" ]]; then
        echo "$identity"
        return
    fi

    # Fall back to Apple Development (testing only)
    identity=$(security find-identity -v -p codesigning 2>/dev/null | \
        grep "Apple Development" | \
        head -1 | \
        sed 's/.*"\(.*\)".*/\1/')

    echo "$identity"
}

# Sign macOS binary
sign_macos_binary() {
    if [[ "$(uname -s)" != "Darwin" ]]; then
        return
    fi

    if [[ "${SKIP_SIGN:-}" == "1" ]]; then
        log_warn "Skipping code signing (SKIP_SIGN=1)"
        return
    fi

    log_step "Signing macOS Binary"

    local universal_dir="$ARTIFACTS_DIR/universal"

    if [[ ! -f "$universal_dir/$BINARY_NAME" ]]; then
        log_warn "Universal binary not found, skipping signing"
        return
    fi

    # Auto-detect signing identity
    local identity
    identity=$(detect_signing_identity)

    if [[ -z "$identity" ]]; then
        log_warn "No signing identity found, skipping code signing"
        log_info "Install a Developer ID Application certificate from Apple Developer"
        return
    fi

    log_info "Signing with: $identity"

    codesign --force --options runtime \
        --sign "$identity" \
        --timestamp \
        "$universal_dir/$BINARY_NAME"

    # Verify signature
    log_info "Verifying signature..."
    codesign --verify --deep --strict --verbose=2 "$universal_dir/$BINARY_NAME" 2>&1 | head -5

    log_success "Binary signed successfully"
}

# Notarize macOS binary
notarize_macos_binary() {
    if [[ "$(uname -s)" != "Darwin" ]]; then
        return
    fi

    if [[ "${SKIP_NOTARIZE:-}" == "1" ]]; then
        log_warn "Skipping notarization (SKIP_NOTARIZE=1)"
        return
    fi

    log_step "Notarizing macOS Binary"

    local universal_dir="$ARTIFACTS_DIR/universal"

    if [[ ! -f "$universal_dir/$BINARY_NAME" ]]; then
        log_warn "Universal binary not found, skipping notarization"
        return
    fi

    # Use keychain profile (preferred) or fall back to horizon-analytic profile
    local profile="${NOTARIZATION_PROFILE:-}"

    # Try common profile names if not specified
    if [[ -z "$profile" ]]; then
        # Check if stratum-notarization profile exists
        if xcrun notarytool history --keychain-profile stratum-notarization >/dev/null 2>&1; then
            profile="stratum-notarization"
        # Fall back to horizon-analytic profile
        elif xcrun notarytool history --keychain-profile horizon-analytic-notarization >/dev/null 2>&1; then
            profile="horizon-analytic-notarization"
        fi
    fi

    if [[ -z "$profile" ]]; then
        log_warn "No notarization keychain profile found, skipping notarization"
        log_info "To set up notarization, run:"
        log_info "  xcrun notarytool store-credentials stratum-notarization \\"
        log_info "    --apple-id YOUR_APPLE_ID \\"
        log_info "    --team-id YOUR_TEAM_ID \\"
        log_info "    --password YOUR_APP_SPECIFIC_PASSWORD"
        return
    fi

    log_info "Using keychain profile: $profile"

    # Create a zip for notarization
    local notarize_zip="$BUILD_DIR/stratum-notarize.zip"
    ditto -c -k --keepParent "$universal_dir/$BINARY_NAME" "$notarize_zip"

    log_info "Submitting for notarization (this may take a few minutes)..."

    xcrun notarytool submit "$notarize_zip" \
        --keychain-profile "$profile" \
        --wait

    log_success "Binary notarized successfully"
}

# Create macOS .pkg installer
create_macos_pkg() {
    if [[ "$(uname -s)" != "Darwin" ]]; then
        return
    fi

    log_step "Creating macOS Installer Package"

    local universal_dir="$ARTIFACTS_DIR/universal"
    local pkg_dir="$BUILD_DIR/pkg"

    if [[ ! -f "$universal_dir/$BINARY_NAME" ]]; then
        log_warn "Universal binary not found, skipping .pkg creation"
        return
    fi

    mkdir -p "$pkg_dir/root/usr/local/bin"
    mkdir -p "$pkg_dir/root/usr/local/share/bash-completion/completions"
    mkdir -p "$pkg_dir/root/usr/local/share/zsh/site-functions"
    mkdir -p "$pkg_dir/root/usr/local/share/fish/vendor_completions.d"
    mkdir -p "$pkg_dir/resources"

    # Copy binary
    cp "$universal_dir/$BINARY_NAME" "$pkg_dir/root/usr/local/bin/"
    chmod 755 "$pkg_dir/root/usr/local/bin/$BINARY_NAME"

    # Copy completions
    if [[ -d "$universal_dir/completions" ]]; then
        cp "$universal_dir/completions/stratum.bash" "$pkg_dir/root/usr/local/share/bash-completion/completions/stratum" 2>/dev/null || true
        cp "$universal_dir/completions/_stratum" "$pkg_dir/root/usr/local/share/zsh/site-functions/_stratum" 2>/dev/null || true
        cp "$universal_dir/completions/stratum.fish" "$pkg_dir/root/usr/local/share/fish/vendor_completions.d/stratum.fish" 2>/dev/null || true
    fi

    # Create welcome and conclusion HTML
    cat > "$pkg_dir/resources/welcome.html" << EOF
<!DOCTYPE html>
<html>
<head><title>Welcome</title></head>
<body>
<h1>Stratum Programming Language</h1>
<p>Version $VERSION</p>
<p>This installer will install the Stratum CLI and shell completions.</p>
</body>
</html>
EOF

    cat > "$pkg_dir/resources/conclusion.html" << EOF
<!DOCTYPE html>
<html>
<head><title>Installation Complete</title></head>
<body>
<h1>Installation Complete</h1>
<p>Stratum has been installed successfully!</p>
<p>Open a new terminal and run <code>stratum --help</code> to get started.</p>
</body>
</html>
EOF

    # Create component package
    pkgbuild \
        --root "$pkg_dir/root" \
        --identifier dev.stratum-lang.stratum \
        --version "$VERSION" \
        --install-location "/" \
        "$pkg_dir/stratum-component.pkg"

    # Create distribution file
    cat > "$pkg_dir/distribution.xml" << EOF
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
  <title>Stratum Programming Language</title>
  <organization>dev.stratum-lang</organization>
  <domains enable_localSystem="true"/>
  <options customize="never" require-scripts="false" hostArchitectures="x86_64,arm64"/>
  <welcome file="welcome.html"/>
  <conclusion file="conclusion.html"/>
  <pkg-ref id="dev.stratum-lang.stratum"/>
  <choices-outline>
    <line choice="default">
      <line choice="dev.stratum-lang.stratum"/>
    </line>
  </choices-outline>
  <choice id="default"/>
  <choice id="dev.stratum-lang.stratum" visible="false">
    <pkg-ref id="dev.stratum-lang.stratum"/>
  </choice>
  <pkg-ref id="dev.stratum-lang.stratum" version="$VERSION" onConclusion="none">stratum-component.pkg</pkg-ref>
</installer-gui-script>
EOF

    # Build product archive
    local pkg_name="stratum-${VERSION}-macos.pkg"
    productbuild \
        --distribution "$pkg_dir/distribution.xml" \
        --resources "$pkg_dir/resources" \
        --package-path "$pkg_dir" \
        "$ARTIFACTS_DIR/$pkg_name"

    # Try to sign the package with Developer ID Installer certificate
    # Note: Don't use -p codesigning as it filters out Installer certificates
    local installer_identity
    installer_identity=$(security find-identity -v 2>/dev/null | \
        grep "Developer ID Installer" | \
        head -1 | \
        sed 's/.*"\(.*\)".*/\1/')

    if [[ -n "$installer_identity" ]]; then
        log_info "Signing installer package with: $installer_identity"
        local signed_pkg="stratum-${VERSION}-macos-signed.pkg"
        productsign \
            --sign "$installer_identity" \
            "$ARTIFACTS_DIR/$pkg_name" \
            "$ARTIFACTS_DIR/$signed_pkg"
        mv "$ARTIFACTS_DIR/$signed_pkg" "$ARTIFACTS_DIR/$pkg_name"
    else
        log_warn "No 'Developer ID Installer' certificate found, package will be unsigned"
        log_info "Note: .pkg files require a separate 'Developer ID Installer' certificate"
        log_info "The binary inside is still signed with Developer ID Application"
        log_info "Users will see a Gatekeeper warning but can still install via System Settings"
    fi

    # Notarize the package using keychain profile
    if [[ "${SKIP_NOTARIZE:-}" != "1" ]]; then
        local profile="${NOTARIZATION_PROFILE:-}"

        if [[ -z "$profile" ]]; then
            if xcrun notarytool history --keychain-profile stratum-notarization >/dev/null 2>&1; then
                profile="stratum-notarization"
            elif xcrun notarytool history --keychain-profile horizon-analytic-notarization >/dev/null 2>&1; then
                profile="horizon-analytic-notarization"
            fi
        fi

        if [[ -n "$profile" ]]; then
            log_info "Notarizing installer package with profile: $profile"
            xcrun notarytool submit "$ARTIFACTS_DIR/$pkg_name" \
                --keychain-profile "$profile" \
                --wait

            log_info "Stapling notarization ticket..."
            xcrun stapler staple "$ARTIFACTS_DIR/$pkg_name"
        else
            log_warn "No notarization profile found, skipping package notarization"
        fi
    fi

    log_success "Created $pkg_name"
}

# Create Linux .deb package
create_linux_deb() {
    if [[ "$(uname -s)" != "Linux" ]]; then
        # On macOS, we can still create .deb if we have dpkg-deb
        if ! command -v dpkg-deb >/dev/null 2>&1; then
            log_warn "dpkg-deb not available, skipping .deb creation"
            return
        fi
    fi

    local linux_binary="$ARTIFACTS_DIR/x86_64-unknown-linux-gnu/$BINARY_NAME"
    if [[ ! -f "$linux_binary" ]]; then
        log_warn "Linux x86_64 binary not found, skipping .deb creation"
        return
    fi

    log_step "Creating Linux .deb Package"

    local deb_dir="$BUILD_DIR/deb"
    mkdir -p "$deb_dir/DEBIAN"
    mkdir -p "$deb_dir/usr/bin"
    mkdir -p "$deb_dir/usr/share/bash-completion/completions"
    mkdir -p "$deb_dir/usr/share/zsh/vendor-completions"
    mkdir -p "$deb_dir/usr/share/fish/vendor_completions.d"
    mkdir -p "$deb_dir/usr/share/doc/stratum"

    # Copy binary
    cp "$linux_binary" "$deb_dir/usr/bin/"
    chmod 755 "$deb_dir/usr/bin/$BINARY_NAME"

    # Copy completions
    local completions_dir="$ARTIFACTS_DIR/x86_64-unknown-linux-gnu/completions"
    if [[ -d "$completions_dir" ]]; then
        cp "$completions_dir/stratum.bash" "$deb_dir/usr/share/bash-completion/completions/stratum" 2>/dev/null || true
        cp "$completions_dir/_stratum" "$deb_dir/usr/share/zsh/vendor-completions/_stratum" 2>/dev/null || true
        cp "$completions_dir/stratum.fish" "$deb_dir/usr/share/fish/vendor_completions.d/stratum.fish" 2>/dev/null || true
    fi

    # Calculate installed size
    local installed_size
    installed_size=$(du -sk "$deb_dir" | cut -f1)

    # Create control file
    cat > "$deb_dir/DEBIAN/control" << EOF
Package: stratum
Version: $VERSION
Section: devel
Priority: optional
Architecture: amd64
Installed-Size: $installed_size
Maintainer: Horizon Analytic Studios <support@horizonanalytic.com>
Description: Stratum programming language
 Stratum is a Goldilocks programming language with native data operations
 and GUI support. It offers a learning curve between Python and Rust,
 with built-in DataFrame support, Arrow integration, and a bundled IDE.
Homepage: https://stratum-lang.dev
EOF

    # Create copyright file
    cat > "$deb_dir/usr/share/doc/stratum/copyright" << EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: stratum
Source: https://github.com/horizon-analytic/stratum

Files: *
Copyright: 2024-2026 Horizon Analytic Studios, LLC
License: MIT or Apache-2.0
EOF

    # Build the .deb
    local deb_name="stratum_${VERSION}_amd64.deb"
    dpkg-deb --build "$deb_dir" "$ARTIFACTS_DIR/$deb_name"

    log_success "Created $deb_name"
}

# Create Linux .rpm package
create_linux_rpm() {
    if ! command -v rpmbuild >/dev/null 2>&1; then
        log_warn "rpmbuild not available, skipping .rpm creation"
        return
    fi

    local linux_binary="$ARTIFACTS_DIR/x86_64-unknown-linux-gnu/$BINARY_NAME"
    if [[ ! -f "$linux_binary" ]]; then
        log_warn "Linux x86_64 binary not found, skipping .rpm creation"
        return
    fi

    log_step "Creating Linux .rpm Package"

    # Set up rpmbuild directories
    local rpm_dir="$BUILD_DIR/rpmbuild"
    mkdir -p "$rpm_dir"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
    mkdir -p "$rpm_dir/BUILDROOT/stratum-${VERSION}-1.x86_64"

    local rpm_root="$rpm_dir/BUILDROOT/stratum-${VERSION}-1.x86_64"
    mkdir -p "$rpm_root/usr/bin"
    mkdir -p "$rpm_root/usr/share/bash-completion/completions"
    mkdir -p "$rpm_root/usr/share/zsh/site-functions"
    mkdir -p "$rpm_root/usr/share/fish/vendor_completions.d"

    # Copy binary
    cp "$linux_binary" "$rpm_root/usr/bin/"
    chmod 755 "$rpm_root/usr/bin/$BINARY_NAME"

    # Copy completions
    local completions_dir="$ARTIFACTS_DIR/x86_64-unknown-linux-gnu/completions"
    if [[ -d "$completions_dir" ]]; then
        cp "$completions_dir/stratum.bash" "$rpm_root/usr/share/bash-completion/completions/stratum" 2>/dev/null || true
        cp "$completions_dir/_stratum" "$rpm_root/usr/share/zsh/site-functions/_stratum" 2>/dev/null || true
        cp "$completions_dir/stratum.fish" "$rpm_root/usr/share/fish/vendor_completions.d/stratum.fish" 2>/dev/null || true
    fi

    # Create spec file
    cat > "$rpm_dir/SPECS/stratum.spec" << EOF
Name:           stratum
Version:        $VERSION
Release:        1%{?dist}
Summary:        Stratum programming language

License:        MIT or Apache-2.0
URL:            https://stratum-lang.dev

%description
Stratum is a Goldilocks programming language with native data operations
and GUI support. It offers a learning curve between Python and Rust,
with built-in DataFrame support, Arrow integration, and a bundled IDE.

%files
%{_bindir}/stratum
%{_datadir}/bash-completion/completions/stratum
%{_datadir}/zsh/site-functions/_stratum
%{_datadir}/fish/vendor_completions.d/stratum.fish

%changelog
* $(date +"%a %b %d %Y") Horizon Analytic Studios <support@horizonanalytic.com> - ${VERSION}-1
- Release ${VERSION}
EOF

    # Build the RPM
    rpmbuild --define "_topdir $rpm_dir" -bb "$rpm_dir/SPECS/stratum.spec"

    # Copy to artifacts
    local rpm_name="stratum-${VERSION}-1.x86_64.rpm"
    cp "$rpm_dir/RPMS/x86_64/$rpm_name" "$ARTIFACTS_DIR/"

    log_success "Created $rpm_name"
}

# Create release tarballs
create_tarballs() {
    log_step "Creating Release Tarballs"

    local sha_cmd
    if command -v sha256sum >/dev/null 2>&1; then
        sha_cmd="sha256sum"
    else
        sha_cmd="shasum -a 256"
    fi

    # Create tarballs for each platform
    for target_dir in "$ARTIFACTS_DIR"/*; do
        if [[ ! -d "$target_dir" ]]; then
            continue
        fi

        local target
        target=$(basename "$target_dir")

        # Skip non-target directories
        if [[ "$target" == "universal" ]]; then
            target="macos-universal"
        elif [[ ! "$target" =~ ^(aarch64|x86_64) ]]; then
            continue
        fi

        # Determine archive name
        local archive_name
        case "$target" in
            aarch64-apple-darwin)  archive_name="stratum-macos-arm64" ;;
            x86_64-apple-darwin)   archive_name="stratum-macos-x86_64" ;;
            macos-universal)       archive_name="stratum-macos-universal" ;;
            x86_64-unknown-linux-gnu)   archive_name="stratum-linux-x86_64" ;;
            aarch64-unknown-linux-gnu)  archive_name="stratum-linux-aarch64" ;;
            x86_64-unknown-linux-musl)  archive_name="stratum-linux-x86_64-musl" ;;
            *)                     continue ;;
        esac

        log_info "Creating tarball for $archive_name..."

        local tarball_name="${archive_name}-${VERSION}.tar.gz"
        local work_dir="$BUILD_DIR/tarball-$target"

        mkdir -p "$work_dir/stratum-${VERSION}"

        # Copy binary
        cp "$target_dir/$BINARY_NAME" "$work_dir/stratum-${VERSION}/" 2>/dev/null || continue
        chmod 755 "$work_dir/stratum-${VERSION}/$BINARY_NAME"

        # Copy completions
        if [[ -d "$target_dir/completions" ]]; then
            cp -r "$target_dir/completions" "$work_dir/stratum-${VERSION}/"
        fi

        # Create README
        cat > "$work_dir/stratum-${VERSION}/README.txt" << EOF
Stratum Programming Language v${VERSION}
========================================

Installation:
  1. Copy 'stratum' to a directory in your PATH (e.g., /usr/local/bin)
  2. Run 'stratum --help' to verify installation

Shell Completions:
  Bash: Copy completions/stratum.bash to ~/.local/share/bash-completion/completions/stratum
  Zsh:  Copy completions/_stratum to a directory in your fpath
  Fish: Copy completions/stratum.fish to ~/.config/fish/completions/

Getting Started:
  stratum repl    - Start the interactive REPL
  stratum init    - Create a new project
  stratum --help  - Show all commands

Documentation: https://stratum-lang.dev
Repository: https://github.com/horizon-analytic/stratum
EOF

        # Create tarball
        (cd "$work_dir" && tar -czvf "$ARTIFACTS_DIR/$tarball_name" "stratum-${VERSION}")

        # Create checksum
        (cd "$ARTIFACTS_DIR" && $sha_cmd "$tarball_name" > "${tarball_name}.sha256")

        log_success "Created $tarball_name"
    done
}

# Create master checksums file
create_checksums() {
    log_step "Creating Checksums"

    local sha_cmd
    if command -v sha256sum >/dev/null 2>&1; then
        sha_cmd="sha256sum"
    else
        sha_cmd="shasum -a 256"
    fi

    (
        cd "$ARTIFACTS_DIR"
        $sha_cmd *.tar.gz *.deb *.rpm *.pkg 2>/dev/null > checksums.sha256 || true
    )

    if [[ -f "$ARTIFACTS_DIR/checksums.sha256" ]]; then
        log_info "Checksums:"
        cat "$ARTIFACTS_DIR/checksums.sha256"
        log_success "Created checksums.sha256"
    fi
}

# Upload uninstall script to S3
upload_uninstall_script() {
    log_step "Uploading Uninstall Script to S3"

    local uninstall_script="$PROJECT_ROOT/scripts/uninstall.sh"

    if [[ ! -f "$uninstall_script" ]]; then
        log_warn "Uninstall script not found at $uninstall_script, skipping S3 upload"
        return
    fi

    if ! command -v aws >/dev/null 2>&1; then
        log_warn "AWS CLI not installed, skipping S3 upload"
        log_info "Install AWS CLI: brew install awscli"
        return
    fi

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "Would upload: $uninstall_script -> s3://$S3_BUCKET/$S3_PREFIX/uninstall.sh"
        return
    fi

    log_info "Uploading uninstall.sh to S3..."
    aws s3 cp "$uninstall_script" "s3://$S3_BUCKET/$S3_PREFIX/uninstall.sh" \
        --content-type "text/x-shellscript"

    log_success "Uploaded uninstall.sh to s3://$S3_BUCKET/$S3_PREFIX/uninstall.sh"
}

# Upload release artifacts to S3
upload_release_artifacts() {
    log_step "Uploading Release Artifacts to S3"

    if ! command -v aws >/dev/null 2>&1; then
        log_warn "AWS CLI not installed, skipping S3 artifact upload"
        return
    fi

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "Would upload release artifacts to s3://$S3_BUCKET/$S3_PREFIX/$VERSION/"
        return
    fi

    local s3_version_prefix="$S3_PREFIX/$VERSION"

    # Upload tarballs
    for tarball in "$ARTIFACTS_DIR"/*.tar.gz; do
        if [[ -f "$tarball" ]]; then
            local filename
            filename=$(basename "$tarball")
            log_info "Uploading $filename..."
            aws s3 cp "$tarball" "s3://$S3_BUCKET/$s3_version_prefix/$filename"
        fi
    done

    # Upload installers
    for installer in "$ARTIFACTS_DIR"/*.pkg "$ARTIFACTS_DIR"/*.deb "$ARTIFACTS_DIR"/*.rpm; do
        if [[ -f "$installer" ]]; then
            local filename
            filename=$(basename "$installer")
            log_info "Uploading $filename..."
            aws s3 cp "$installer" "s3://$S3_BUCKET/$s3_version_prefix/$filename"
        fi
    done

    # Upload checksums
    if [[ -f "$ARTIFACTS_DIR/checksums.sha256" ]]; then
        log_info "Uploading checksums.sha256..."
        aws s3 cp "$ARTIFACTS_DIR/checksums.sha256" "s3://$S3_BUCKET/$s3_version_prefix/checksums.sha256"
    fi

    log_success "Uploaded release artifacts to s3://$S3_BUCKET/$s3_version_prefix/"
}

# Register release with Horizon Analytic backend
register_with_horizon_backend() {
    log_step "Registering Release with Horizon Analytic Backend"

    if [[ -z "${HORIZON_ADMIN_API_KEY:-}" ]]; then
        log_warn "HORIZON_ADMIN_API_KEY not set, skipping backend registration"
        log_info "Set HORIZON_ADMIN_API_KEY to register releases with the Horizon Analytic backend"
        return
    fi

    if ! command -v curl >/dev/null 2>&1; then
        log_warn "curl not installed, skipping backend registration"
        return
    fi

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "Would register version $VERSION with Horizon Analytic backend"
        return
    fi

    local api_endpoint="$HORIZON_API_URL/api/v1/admin/releases/bulk"
    local s3_version_prefix="$S3_PREFIX/$VERSION"
    local releases_json="[]"

    # Build release entries for each platform
    local releases=()

    # macOS universal
    if [[ -f "$ARTIFACTS_DIR/stratum-macos-universal-${VERSION}.tar.gz" ]]; then
        releases+=("{
            \"platform\": \"macos\",
            \"architecture\": \"universal\",
            \"s3_key\": \"$s3_version_prefix/stratum-macos-universal-${VERSION}.tar.gz\",
            \"filename\": \"stratum-macos-universal-${VERSION}.tar.gz\",
            \"installer_type\": \"tar.gz\"
        }")
    fi

    # macOS arm64
    if [[ -f "$ARTIFACTS_DIR/stratum-macos-arm64-${VERSION}.tar.gz" ]]; then
        releases+=("{
            \"platform\": \"macos\",
            \"architecture\": \"arm64\",
            \"s3_key\": \"$s3_version_prefix/stratum-macos-arm64-${VERSION}.tar.gz\",
            \"filename\": \"stratum-macos-arm64-${VERSION}.tar.gz\",
            \"installer_type\": \"tar.gz\"
        }")
    fi

    # macOS x86_64
    if [[ -f "$ARTIFACTS_DIR/stratum-macos-x86_64-${VERSION}.tar.gz" ]]; then
        releases+=("{
            \"platform\": \"macos\",
            \"architecture\": \"x64\",
            \"s3_key\": \"$s3_version_prefix/stratum-macos-x86_64-${VERSION}.tar.gz\",
            \"filename\": \"stratum-macos-x86_64-${VERSION}.tar.gz\",
            \"installer_type\": \"tar.gz\"
        }")
    fi

    # macOS .pkg installer
    if [[ -f "$ARTIFACTS_DIR/stratum-${VERSION}-macos.pkg" ]]; then
        releases+=("{
            \"platform\": \"macos\",
            \"architecture\": \"universal\",
            \"s3_key\": \"$s3_version_prefix/stratum-${VERSION}-macos.pkg\",
            \"filename\": \"stratum-${VERSION}-macos.pkg\",
            \"installer_type\": \"pkg\"
        }")
    fi

    # Linux x86_64 tarball
    if [[ -f "$ARTIFACTS_DIR/stratum-linux-x86_64-${VERSION}.tar.gz" ]]; then
        releases+=("{
            \"platform\": \"linux\",
            \"architecture\": \"x64\",
            \"s3_key\": \"$s3_version_prefix/stratum-linux-x86_64-${VERSION}.tar.gz\",
            \"filename\": \"stratum-linux-x86_64-${VERSION}.tar.gz\",
            \"installer_type\": \"tar.gz\"
        }")
    fi

    # Linux aarch64 tarball
    if [[ -f "$ARTIFACTS_DIR/stratum-linux-aarch64-${VERSION}.tar.gz" ]]; then
        releases+=("{
            \"platform\": \"linux\",
            \"architecture\": \"arm64\",
            \"s3_key\": \"$s3_version_prefix/stratum-linux-aarch64-${VERSION}.tar.gz\",
            \"filename\": \"stratum-linux-aarch64-${VERSION}.tar.gz\",
            \"installer_type\": \"tar.gz\"
        }")
    fi

    # Linux .deb
    if [[ -f "$ARTIFACTS_DIR/stratum_${VERSION}_amd64.deb" ]]; then
        releases+=("{
            \"platform\": \"linux\",
            \"architecture\": \"x64\",
            \"s3_key\": \"$s3_version_prefix/stratum_${VERSION}_amd64.deb\",
            \"filename\": \"stratum_${VERSION}_amd64.deb\",
            \"installer_type\": \"deb\"
        }")
    fi

    # Linux .rpm
    if [[ -f "$ARTIFACTS_DIR/stratum-${VERSION}-1.x86_64.rpm" ]]; then
        releases+=("{
            \"platform\": \"linux\",
            \"architecture\": \"x64\",
            \"s3_key\": \"$s3_version_prefix/stratum-${VERSION}-1.x86_64.rpm\",
            \"filename\": \"stratum-${VERSION}-1.x86_64.rpm\",
            \"installer_type\": \"rpm\"
        }")
    fi

    if [[ ${#releases[@]} -eq 0 ]]; then
        log_warn "No release artifacts found to register"
        return
    fi

    # Join releases array into JSON array
    local releases_json
    releases_json=$(printf '%s,' "${releases[@]}")
    releases_json="[${releases_json%,}]"

    # Get product ID from backend (or use cached/hardcoded value)
    # For now, we'll query the product by slug
    log_info "Fetching product ID for $PRODUCT_SLUG..."
    local product_response
    product_response=$(curl -s "$HORIZON_API_URL/api/v1/products/$PRODUCT_SLUG" 2>/dev/null || echo '{}')

    local product_id
    product_id=$(echo "$product_response" | grep -o '"id":"[^"]*"' | head -1 | sed 's/"id":"\([^"]*\)"/\1/')

    if [[ -z "$product_id" ]]; then
        log_warn "Could not fetch product ID for $PRODUCT_SLUG, skipping backend registration"
        log_info "Ensure the product exists in the Horizon Analytic backend"
        return
    fi

    log_info "Product ID: $product_id"
    log_info "Registering ${#releases[@]} release(s) for version $VERSION..."

    # Build bulk request payload
    local payload
    payload=$(cat << EOF
{
    "product_id": "$product_id",
    "version": "$VERSION",
    "releases": $releases_json,
    "mark_as_latest": true
}
EOF
)

    # Send registration request
    local response
    response=$(curl -s -w "\n%{http_code}" -X POST "$api_endpoint" \
        -H "Content-Type: application/json" \
        -H "X-Admin-API-Key: $HORIZON_ADMIN_API_KEY" \
        -d "$payload" 2>/dev/null)

    local http_code
    http_code=$(echo "$response" | tail -1)
    local body
    body=$(echo "$response" | sed '$d')

    if [[ "$http_code" == "200" ]] || [[ "$http_code" == "201" ]]; then
        log_success "Registered ${#releases[@]} release(s) with Horizon Analytic backend"
    else
        log_warn "Backend registration returned HTTP $http_code"
        log_info "Response: $body"
        log_info "You may need to register releases manually"
    fi
}

# Publish to GitHub
publish_to_github() {
    if [[ "$DRY_RUN" == "true" ]]; then
        log_warn "Dry run mode, skipping GitHub release"
        return
    fi

    if [[ -z "${GITHUB_TOKEN:-}" ]]; then
        log_warn "GITHUB_TOKEN not set, skipping GitHub release"
        return
    fi

    if ! command -v gh >/dev/null 2>&1; then
        log_warn "GitHub CLI (gh) not installed, skipping GitHub release"
        return
    fi

    log_step "Publishing to GitHub"

    cd "$PROJECT_ROOT"

    # Check if tag exists
    if ! git rev-parse "v${VERSION}" >/dev/null 2>&1; then
        log_info "Creating tag v${VERSION}..."
        git tag -a "v${VERSION}" -m "Release v${VERSION}"
        git push origin "v${VERSION}"
    fi

    # Determine if prerelease
    local prerelease_flag=""
    if [[ "$VERSION" == *"-"* ]]; then
        prerelease_flag="--prerelease"
    fi

    # Create release notes
    local release_notes="$BUILD_DIR/release-notes.md"
    cat > "$release_notes" << EOF
## Installation

### Quick Install (macOS/Linux)
\`\`\`bash
curl -fsSL https://get.stratum-lang.dev | sh
\`\`\`

### Homebrew (macOS/Linux)
\`\`\`bash
brew tap horizon-analytic/stratum
brew install stratum
\`\`\`

### macOS Installer
Download \`stratum-${VERSION}-macos.pkg\` and run the installer.

### Linux (Debian/Ubuntu)
\`\`\`bash
sudo dpkg -i stratum_${VERSION}_amd64.deb
\`\`\`

### Linux (Fedora/RHEL)
\`\`\`bash
sudo rpm -i stratum-${VERSION}-1.x86_64.rpm
\`\`\`

### Manual Installation
Download the appropriate tarball for your platform, extract, and copy the \`stratum\` binary to your PATH.

## Checksums
SHA256 checksums for all files are available in \`checksums.sha256\`.

## Getting Started
\`\`\`bash
stratum repl     # Start the interactive REPL
stratum init     # Create a new project
stratum --help   # Show all commands
\`\`\`
EOF

    # Create release
    log_info "Creating GitHub release v${VERSION}..."

    gh release create "v${VERSION}" \
        --title "Stratum v${VERSION}" \
        --notes-file "$release_notes" \
        $prerelease_flag \
        "$ARTIFACTS_DIR"/*.tar.gz \
        "$ARTIFACTS_DIR"/*.sha256 \
        "$ARTIFACTS_DIR"/*.deb \
        "$ARTIFACTS_DIR"/*.rpm \
        "$ARTIFACTS_DIR"/*.pkg \
        2>/dev/null || true

    log_success "Published release v${VERSION}"
}

# Main
main() {
    echo ""
    echo "╔═══════════════════════════════════════════════════════════════════════════╗"
    echo "║                    Stratum Release Build v${VERSION}                         ║"
    echo "╚═══════════════════════════════════════════════════════════════════════════╝"
    echo ""

    if [[ "$DRY_RUN" == "true" ]]; then
        log_warn "DRY RUN MODE - Will build but not publish"
    fi

    if [[ "$LOCAL_ONLY" == "true" ]]; then
        log_info "LOCAL MODE - Building for current platform only"
    fi

    check_requirements
    run_tests
    build_all

    if [[ "$(uname -s)" == "Darwin" ]] && [[ "$LOCAL_ONLY" == "false" ]]; then
        create_universal_binary
        sign_macos_binary
        notarize_macos_binary
        create_macos_pkg
    fi

    if [[ "$LOCAL_ONLY" == "false" ]]; then
        create_linux_deb
        create_linux_rpm
    fi

    create_tarballs
    create_checksums

    # S3 and Backend Integration
    upload_uninstall_script
    upload_release_artifacts
    register_with_horizon_backend

    publish_to_github

    log_step "Release Complete"

    echo ""
    log_info "Artifacts available in: $ARTIFACTS_DIR"
    echo ""
    ls -la "$ARTIFACTS_DIR"
    echo ""

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "To publish this release, run without --dry-run"
    fi

    log_success "Done!"
}

main
