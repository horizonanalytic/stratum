#!/bin/sh
# Stratum Programming Language Installer
#
# Usage:
#   curl --proto '=https' --tlsv1.2 -sSf https://get.stratum-lang.dev/install.sh | sh
#
# Or with options:
#   curl --proto '=https' --tlsv1.2 -sSf https://get.stratum-lang.dev/install.sh | sh -s -- --tier=full
#
# Environment Variables:
#   STRATUM_HOME     - Installation directory (default: ~/.stratum)
#   STRATUM_VERSION  - Version to install (default: latest)
#   STRATUM_TIER     - Installation tier: core, data, gui, full (default: full)
#
# This script wraps all code in functions with main() called at the end
# to prevent partial download execution (security best practice).

set -eu

# ============================================================================
# Configuration
# ============================================================================

STRATUM_VERSION="${STRATUM_VERSION:-latest}"
STRATUM_HOME="${STRATUM_HOME:-$HOME/.stratum}"
STRATUM_TIER="${STRATUM_TIER:-full}"
STRATUM_BASE_URL="${STRATUM_BASE_URL:-https://github.com/horizon-analytic/stratum/releases/download}"

# Installation state
_QUIET=0
_YES=0
_NO_PATH=0
_NO_COMPLETIONS=0
_FORCE=0
_DRY_RUN=0

# Rollback tracking
_ROLLBACK_DIRS=""
_ROLLBACK_FILES=""
_ROLLBACK_PROFILE_BACKUP=""
_ROLLBACK_ENABLED=0
_INSTALL_SUCCEEDED=0

# Progress indicator state
_SPINNER_PID=""

# Detected environment
_PLATFORM=""
_ARCH=""
_LIBC=""
_TARGET=""
_SHELL_NAME=""
_SHELL_PROFILE=""
_SHELLS_DETECTED=""
_EXISTING_INSTALL=""
_EXISTING_VERSION=""

# Minimum requirements
MIN_GLIBC_VERSION="2.17"

# ============================================================================
# Output Utilities
# ============================================================================

# ANSI color codes (disabled if not a TTY)
if [ -t 1 ]; then
    BOLD='\033[1m'
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    CYAN='\033[0;36m'
    RESET='\033[0m'
else
    BOLD=''
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    CYAN=''
    RESET=''
fi

say() {
    if [ "$_QUIET" -eq 0 ]; then
        printf '%b\n' "$1"
    fi
}

say_verbose() {
    if [ "$_QUIET" -eq 0 ]; then
        printf '%b\n' "${CYAN}$1${RESET}"
    fi
}

say_success() {
    printf '%b\n' "${GREEN}✓${RESET} $1"
}

say_warning() {
    printf '%b\n' "${YELLOW}⚠${RESET} $1" >&2
}

say_error() {
    printf '%b\n' "${RED}✗${RESET} $1" >&2
}

err() {
    say_error "$1"
    exit 1
}

# ============================================================================
# Progress Indicators
# ============================================================================

# Spinner characters for progress indication
SPINNER_CHARS='⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏'
SPINNER_CHARS_FALLBACK='|/-\'

# Start a spinner in the background
# Usage: start_spinner "message"
start_spinner() {
    local message="$1"

    if [ "$_QUIET" -eq 1 ] || ! [ -t 1 ]; then
        return
    fi

    # Choose spinner characters based on terminal capability
    local chars="$SPINNER_CHARS_FALLBACK"
    if printf '%s' "$SPINNER_CHARS" | head -c1 2>/dev/null | grep -q '^.' 2>/dev/null; then
        chars="$SPINNER_CHARS"
    fi

    # Start spinner in background
    (
        local i=0
        local len=${#chars}
        while true; do
            local char
            # Get character at position i
            char="$(printf '%s' "$chars" | cut -c$((i % len + 1)))"
            printf '\r%b %s' "${CYAN}${char}${RESET}" "$message"
            i=$((i + 1))
            sleep 0.1
        done
    ) &
    _SPINNER_PID=$!
}

# Stop the spinner
# Usage: stop_spinner [success|fail|clear]
stop_spinner() {
    local status="${1:-clear}"

    if [ -n "$_SPINNER_PID" ]; then
        kill "$_SPINNER_PID" 2>/dev/null || true
        wait "$_SPINNER_PID" 2>/dev/null || true
        _SPINNER_PID=""
    fi

    if [ "$_QUIET" -eq 1 ] || ! [ -t 1 ]; then
        return
    fi

    # Clear the spinner line
    printf '\r\033[K'

    # Optionally print a status indicator
    case "$status" in
        success)
            # Status message printed separately
            ;;
        fail)
            # Error message printed separately
            ;;
        clear)
            # Just clear, no message
            ;;
    esac
}

# Progress bar for downloads (shows percentage)
# Usage: show_download_progress current_bytes total_bytes
show_download_progress() {
    local current="$1"
    local total="$2"

    if [ "$_QUIET" -eq 1 ] || ! [ -t 1 ]; then
        return
    fi

    if [ "$total" -eq 0 ]; then
        return
    fi

    local percent=$((current * 100 / total))
    local bar_width=30
    local filled=$((percent * bar_width / 100))
    local empty=$((bar_width - filled))

    # Build progress bar
    local bar=""
    local i=0
    while [ $i -lt $filled ]; do
        bar="${bar}█"
        i=$((i + 1))
    done
    i=0
    while [ $i -lt $empty ]; do
        bar="${bar}░"
        i=$((i + 1))
    done

    # Format size
    local current_mb=$((current / 1024 / 1024))
    local total_mb=$((total / 1024 / 1024))

    printf '\r  [%s] %3d%% (%d/%d MB)' "$bar" "$percent" "$current_mb" "$total_mb"
}

# Complete the progress bar
finish_progress_bar() {
    if [ "$_QUIET" -eq 1 ] || ! [ -t 1 ]; then
        return
    fi
    printf '\r\033[K'
}

# ============================================================================
# Rollback System
# ============================================================================

# Track a directory for rollback
track_directory() {
    local dir="$1"
    if [ -z "$_ROLLBACK_DIRS" ]; then
        _ROLLBACK_DIRS="$dir"
    else
        _ROLLBACK_DIRS="$_ROLLBACK_DIRS:$dir"
    fi
}

# Track a file for rollback (will be deleted on rollback)
track_file() {
    local file="$1"
    if [ -z "$_ROLLBACK_FILES" ]; then
        _ROLLBACK_FILES="$file"
    else
        _ROLLBACK_FILES="$_ROLLBACK_FILES:$file"
    fi
}

# Backup a file before modification (for rollback)
backup_for_rollback() {
    local file="$1"
    if [ -f "$file" ]; then
        local backup="${file}.stratum-backup-$$"
        cp "$file" "$backup"
        if [ -z "$_ROLLBACK_PROFILE_BACKUP" ]; then
            _ROLLBACK_PROFILE_BACKUP="$file:$backup"
        else
            _ROLLBACK_PROFILE_BACKUP="$_ROLLBACK_PROFILE_BACKUP|$file:$backup"
        fi
    fi
}

# Perform rollback on failure
rollback() {
    if [ "$_ROLLBACK_ENABLED" -eq 0 ] || [ "$_INSTALL_SUCCEEDED" -eq 1 ]; then
        return
    fi

    say ""
    say_warning "Installation failed. Rolling back changes..."

    # Stop any running spinner
    stop_spinner clear

    # Restore backed up files
    if [ -n "$_ROLLBACK_PROFILE_BACKUP" ]; then
        echo "$_ROLLBACK_PROFILE_BACKUP" | tr '|' '\n' | while read -r pair; do
            local original="${pair%%:*}"
            local backup="${pair#*:}"
            if [ -f "$backup" ]; then
                mv "$backup" "$original"
                say_verbose "  Restored: $original"
            fi
        done
    fi

    # Remove created files
    if [ -n "$_ROLLBACK_FILES" ]; then
        echo "$_ROLLBACK_FILES" | tr ':' '\n' | while read -r file; do
            if [ -f "$file" ]; then
                rm -f "$file"
                say_verbose "  Removed file: $file"
            fi
        done
    fi

    # Remove created directories (in reverse order, only if empty or stratum-created)
    if [ -n "$_ROLLBACK_DIRS" ]; then
        echo "$_ROLLBACK_DIRS" | tr ':' '\n' | sort -r | while read -r dir; do
            if [ -d "$dir" ]; then
                # Only remove if it's empty or we created it
                rmdir "$dir" 2>/dev/null || rm -rf "$dir" 2>/dev/null || true
                say_verbose "  Removed directory: $dir"
            fi
        done
    fi

    say_success "Rollback complete. Your system is unchanged."
}

# Clean up backup files on success
cleanup_backups() {
    if [ -n "$_ROLLBACK_PROFILE_BACKUP" ]; then
        echo "$_ROLLBACK_PROFILE_BACKUP" | tr '|' '\n' | while read -r pair; do
            local backup="${pair#*:}"
            rm -f "$backup" 2>/dev/null || true
        done
    fi
}

# Enable rollback and set up trap
enable_rollback() {
    _ROLLBACK_ENABLED=1
    trap rollback EXIT INT TERM
}

# Disable rollback (on success)
disable_rollback() {
    _INSTALL_SUCCEEDED=1
    cleanup_backups
    trap - EXIT INT TERM
}

# ============================================================================
# Platform Detection
# ============================================================================

# Detect the operating system
detect_os() {
    local os
    os="$(uname -s)"

    case "$os" in
        Linux)
            _PLATFORM="linux"
            ;;
        Darwin)
            _PLATFORM="macos"
            ;;
        FreeBSD)
            _PLATFORM="freebsd"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            err "Windows is not yet supported. See Phase 14 of our roadmap."
            ;;
        *)
            err "Unsupported operating system: $os"
            ;;
    esac
}

# Detect the CPU architecture
detect_arch() {
    local arch
    arch="$(uname -m)"

    case "$arch" in
        x86_64|amd64)
            _ARCH="x86_64"
            ;;
        arm64|aarch64)
            _ARCH="aarch64"
            ;;
        armv7l|armv8l)
            _ARCH="armv7"
            ;;
        i386|i686)
            err "32-bit x86 is not supported. Stratum requires a 64-bit system."
            ;;
        *)
            err "Unsupported architecture: $arch"
            ;;
    esac

    # macOS: Check if running under Rosetta 2
    if [ "$_PLATFORM" = "macos" ] && [ "$_ARCH" = "x86_64" ]; then
        if sysctl -n sysctl.proc_translated 2>/dev/null | grep -q "1"; then
            say_warning "Running under Rosetta 2. Installing native ARM64 binary for better performance."
            _ARCH="aarch64"
        fi
    fi
}

# Detect the C library (Linux only)
detect_libc() {
    if [ "$_PLATFORM" != "linux" ]; then
        _LIBC=""
        return
    fi

    # Method 1: Check ldd version output
    if command -v ldd >/dev/null 2>&1; then
        if ldd --version 2>&1 | grep -qi musl; then
            _LIBC="musl"
            return
        fi
    fi

    # Method 2: Check for musl in the dynamic linker
    if [ -f /lib/ld-musl-x86_64.so.1 ] || [ -f /lib/ld-musl-aarch64.so.1 ]; then
        _LIBC="musl"
        return
    fi

    # Method 3: Check /proc/self/exe linkage
    if command -v readlink >/dev/null 2>&1; then
        local exe_link
        exe_link="$(readlink /proc/self/exe 2>/dev/null || true)"
        if echo "$exe_link" | grep -qi musl; then
            _LIBC="musl"
            return
        fi
    fi

    # Default to glibc
    _LIBC="gnu"
}

# Get the glibc version (Linux only)
get_glibc_version() {
    if [ "$_LIBC" != "gnu" ]; then
        echo ""
        return
    fi

    # Try ldd --version first
    if command -v ldd >/dev/null 2>&1; then
        local version
        version="$(ldd --version 2>&1 | head -n1 | grep -oE '[0-9]+\.[0-9]+' | head -n1)"
        if [ -n "$version" ]; then
            echo "$version"
            return
        fi
    fi

    # Fallback: check libc.so directly
    if [ -f /lib/x86_64-linux-gnu/libc.so.6 ]; then
        /lib/x86_64-linux-gnu/libc.so.6 2>&1 | grep -oE 'version [0-9]+\.[0-9]+' | grep -oE '[0-9]+\.[0-9]+'
        return
    fi

    echo ""
}

# Compare version strings (returns 0 if version1 >= version2)
version_gte() {
    local v1="$1"
    local v2="$2"

    # Split on dots and compare numerically
    local v1_major v1_minor v2_major v2_minor
    v1_major="${v1%%.*}"
    v1_minor="${v1#*.}"
    v2_major="${v2%%.*}"
    v2_minor="${v2#*.}"

    if [ "$v1_major" -gt "$v2_major" ]; then
        return 0
    elif [ "$v1_major" -eq "$v2_major" ] && [ "$v1_minor" -ge "$v2_minor" ]; then
        return 0
    fi
    return 1
}

# Build the target triple
build_target() {
    case "$_PLATFORM" in
        macos)
            _TARGET="${_ARCH}-apple-darwin"
            ;;
        linux)
            if [ "$_LIBC" = "musl" ]; then
                _TARGET="${_ARCH}-unknown-linux-musl"
            else
                _TARGET="${_ARCH}-unknown-linux-gnu"
            fi
            ;;
        freebsd)
            _TARGET="${_ARCH}-unknown-freebsd"
            ;;
    esac
}

# Run all platform detection
detect_platform() {
    say_verbose "Detecting platform..."

    detect_os
    detect_arch
    detect_libc
    build_target

    say_verbose "  OS: $_PLATFORM"
    say_verbose "  Architecture: $_ARCH"
    if [ -n "$_LIBC" ]; then
        say_verbose "  C Library: $_LIBC"
    fi
    say_verbose "  Target: $_TARGET"
}

# ============================================================================
# Shell Detection
# ============================================================================

# Get the name of a shell from its path
shell_name() {
    basename "$1" 2>/dev/null || echo ""
}

# Detect the user's current/default shell
detect_current_shell() {
    # First try $SHELL environment variable
    if [ -n "${SHELL:-}" ]; then
        _SHELL_NAME="$(shell_name "$SHELL")"
        return
    fi

    # Fallback: check passwd entry
    if command -v getent >/dev/null 2>&1; then
        local shell_path
        shell_path="$(getent passwd "$(whoami)" 2>/dev/null | cut -d: -f7)"
        if [ -n "$shell_path" ]; then
            _SHELL_NAME="$(shell_name "$shell_path")"
            return
        fi
    fi

    # Last resort: assume bash
    _SHELL_NAME="bash"
}

# Detect all installed shells (for completion installation)
detect_installed_shells() {
    _SHELLS_DETECTED=""

    # Check for bash
    if [ -f "$HOME/.bashrc" ] || [ -f "$HOME/.bash_profile" ]; then
        _SHELLS_DETECTED="$_SHELLS_DETECTED bash"
    elif command -v bash >/dev/null 2>&1; then
        _SHELLS_DETECTED="$_SHELLS_DETECTED bash"
    fi

    # Check for zsh
    if [ -f "$HOME/.zshrc" ] || [ -f "$HOME/.zprofile" ]; then
        _SHELLS_DETECTED="$_SHELLS_DETECTED zsh"
    elif command -v zsh >/dev/null 2>&1; then
        _SHELLS_DETECTED="$_SHELLS_DETECTED zsh"
    fi

    # Check for fish
    if [ -d "$HOME/.config/fish" ] || command -v fish >/dev/null 2>&1; then
        _SHELLS_DETECTED="$_SHELLS_DETECTED fish"
    fi

    # Trim leading space
    _SHELLS_DETECTED="$(echo "$_SHELLS_DETECTED" | sed 's/^ *//')"
}

# Detect the appropriate shell profile file to modify
detect_shell_profile() {
    case "$_SHELL_NAME" in
        bash)
            # Prefer .bashrc for interactive shells, .bash_profile for login
            if [ -f "$HOME/.bashrc" ]; then
                _SHELL_PROFILE="$HOME/.bashrc"
            elif [ -f "$HOME/.bash_profile" ]; then
                _SHELL_PROFILE="$HOME/.bash_profile"
            elif [ -f "$HOME/.profile" ]; then
                _SHELL_PROFILE="$HOME/.profile"
            else
                # Create .bashrc if nothing exists
                _SHELL_PROFILE="$HOME/.bashrc"
            fi
            ;;
        zsh)
            if [ -f "$HOME/.zshrc" ]; then
                _SHELL_PROFILE="$HOME/.zshrc"
            elif [ -f "$HOME/.zprofile" ]; then
                _SHELL_PROFILE="$HOME/.zprofile"
            else
                _SHELL_PROFILE="$HOME/.zshrc"
            fi
            ;;
        fish)
            # Fish uses conf.d for modular configuration
            _SHELL_PROFILE="$HOME/.config/fish/conf.d/stratum.fish"
            ;;
        *)
            # Fallback to .profile for unknown shells
            _SHELL_PROFILE="$HOME/.profile"
            ;;
    esac
}

# Run all shell detection
detect_shells() {
    say_verbose "Detecting shell environment..."

    detect_current_shell
    detect_installed_shells
    detect_shell_profile

    say_verbose "  Current shell: $_SHELL_NAME"
    say_verbose "  Detected shells: $_SHELLS_DETECTED"
    say_verbose "  Profile file: $_SHELL_PROFILE"
}

# ============================================================================
# Existing Installation Detection
# ============================================================================

# Check for existing Stratum installation
check_existing_installation() {
    say_verbose "Checking for existing installation..."

    _EXISTING_INSTALL=""
    _EXISTING_VERSION=""

    # Check 1: STRATUM_HOME directory
    if [ -d "$STRATUM_HOME" ] && [ -f "$STRATUM_HOME/bin/stratum" ]; then
        _EXISTING_INSTALL="$STRATUM_HOME"
        if [ -x "$STRATUM_HOME/bin/stratum" ]; then
            _EXISTING_VERSION="$("$STRATUM_HOME/bin/stratum" --version 2>/dev/null | head -n1 || echo "unknown")"
        fi
    fi

    # Check 2: stratum in PATH
    if [ -z "$_EXISTING_INSTALL" ]; then
        local stratum_path
        stratum_path="$(command -v stratum 2>/dev/null || true)"
        if [ -n "$stratum_path" ]; then
            # Resolve symlinks to find actual installation
            if command -v realpath >/dev/null 2>&1; then
                stratum_path="$(realpath "$stratum_path")"
            elif command -v readlink >/dev/null 2>&1; then
                stratum_path="$(readlink -f "$stratum_path" 2>/dev/null || echo "$stratum_path")"
            fi
            _EXISTING_INSTALL="$(dirname "$(dirname "$stratum_path")")"
            _EXISTING_VERSION="$(stratum --version 2>/dev/null | head -n1 || echo "unknown")"
        fi
    fi

    # Check 3: Homebrew installation
    if [ -z "$_EXISTING_INSTALL" ] && command -v brew >/dev/null 2>&1; then
        if brew list stratum >/dev/null 2>&1; then
            _EXISTING_INSTALL="homebrew"
            _EXISTING_VERSION="$(brew info stratum 2>/dev/null | head -n1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "unknown")"
        fi
    fi

    # Check 4: System package manager installation
    if [ -z "$_EXISTING_INSTALL" ]; then
        if command -v dpkg >/dev/null 2>&1 && dpkg -l stratum >/dev/null 2>&1; then
            _EXISTING_INSTALL="apt"
            _EXISTING_VERSION="$(dpkg -l stratum 2>/dev/null | grep '^ii' | awk '{print $3}' || echo "unknown")"
        elif command -v rpm >/dev/null 2>&1 && rpm -q stratum >/dev/null 2>&1; then
            _EXISTING_INSTALL="rpm"
            _EXISTING_VERSION="$(rpm -q stratum 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "unknown")"
        fi
    fi

    if [ -n "$_EXISTING_INSTALL" ]; then
        say_verbose "  Found existing installation: $_EXISTING_INSTALL"
        say_verbose "  Version: $_EXISTING_VERSION"
    else
        say_verbose "  No existing installation found"
    fi
}

# ============================================================================
# System Requirements Verification
# ============================================================================

# Check all system requirements
verify_system_requirements() {
    say_verbose "Verifying system requirements..."

    local errors=0

    # Check 1: Required commands
    for cmd in curl tar; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            # curl fallback to wget
            if [ "$cmd" = "curl" ] && command -v wget >/dev/null 2>&1; then
                say_verbose "  curl not found, will use wget"
                continue
            fi
            say_error "Required command not found: $cmd"
            errors=$((errors + 1))
        fi
    done

    # Check 2: glibc version (Linux only)
    if [ "$_PLATFORM" = "linux" ] && [ "$_LIBC" = "gnu" ]; then
        local glibc_version
        glibc_version="$(get_glibc_version)"
        if [ -n "$glibc_version" ]; then
            say_verbose "  glibc version: $glibc_version"
            if ! version_gte "$glibc_version" "$MIN_GLIBC_VERSION"; then
                say_error "glibc version $glibc_version is too old. Minimum required: $MIN_GLIBC_VERSION"
                say_error "Consider using the musl build or upgrading your system."
                errors=$((errors + 1))
            fi
        else
            say_warning "Could not detect glibc version. Proceeding anyway..."
        fi
    fi

    # Check 3: Disk space (rough estimate based on tier)
    local required_mb
    case "$STRATUM_TIER" in
        core) required_mb=20 ;;
        data) required_mb=60 ;;
        gui) required_mb=100 ;;
        full) required_mb=150 ;;
        *) required_mb=150 ;;
    esac

    local available_mb
    if command -v df >/dev/null 2>&1; then
        # Get available space in the home directory (in MB)
        available_mb="$(df -m "$HOME" 2>/dev/null | tail -1 | awk '{print $4}')"
        if [ -n "$available_mb" ] && [ "$available_mb" -lt "$required_mb" ]; then
            say_error "Insufficient disk space. Available: ${available_mb}MB, Required: ~${required_mb}MB"
            errors=$((errors + 1))
        else
            say_verbose "  Disk space: ${available_mb}MB available (need ~${required_mb}MB)"
        fi
    fi

    # Check 4: Write permissions
    local install_parent
    install_parent="$(dirname "$STRATUM_HOME")"
    if [ -d "$install_parent" ]; then
        if ! [ -w "$install_parent" ]; then
            say_error "No write permission to: $install_parent"
            say_error "Choose a different installation directory or fix permissions."
            errors=$((errors + 1))
        fi
    fi

    # Check 5: /tmp is executable (for downloads)
    if [ -d "/tmp" ]; then
        local test_script="/tmp/stratum_exec_test_$$"
        if printf '#!/bin/sh\nexit 0\n' > "$test_script" 2>/dev/null; then
            if ! chmod +x "$test_script" 2>/dev/null || ! "$test_script" 2>/dev/null; then
                say_warning "/tmp appears to be mounted noexec. Using alternative temp directory."
                export TMPDIR="$HOME/.stratum-tmp"
                mkdir -p "$TMPDIR"
            fi
            rm -f "$test_script" 2>/dev/null || true
        fi
    fi

    if [ "$errors" -gt 0 ]; then
        err "System requirements check failed with $errors error(s)."
    fi

    say_verbose "  All system requirements satisfied"
}

# ============================================================================
# Download Utilities
# ============================================================================

# Download a file using curl or wget
# Usage: download url output [show_progress]
download() {
    local url="$1"
    local output="$2"
    local show_progress="${3:-0}"

    say_verbose "Downloading: $url"

    if command -v curl >/dev/null 2>&1; then
        if [ "$show_progress" -eq 1 ] && [ -t 1 ] && [ "$_QUIET" -eq 0 ]; then
            # Use curl's progress bar
            curl --proto '=https' --tlsv1.2 -fL "$url" -o "$output" --progress-bar 2>&1 | \
                while IFS= read -r line; do
                    printf '\r  %s' "$line"
                done
            printf '\r\033[K'
        else
            curl --proto '=https' --tlsv1.2 -sSfL "$url" -o "$output"
        fi
    elif command -v wget >/dev/null 2>&1; then
        if [ "$show_progress" -eq 1 ] && [ -t 1 ] && [ "$_QUIET" -eq 0 ]; then
            # wget shows progress by default, format it nicely
            wget --https-only "$url" -O "$output" 2>&1 | \
                while IFS= read -r line; do
                    case "$line" in
                        *%*) printf '\r  %s' "$line" ;;
                    esac
                done
            printf '\r\033[K'
        else
            wget --https-only --quiet "$url" -O "$output"
        fi
    else
        err "Neither curl nor wget found. Cannot download files."
    fi
}

# Download with progress indicator (wrapper for visual feedback)
download_with_progress() {
    local url="$1"
    local output="$2"
    local description="${3:-Downloading}"

    if [ -t 1 ] && [ "$_QUIET" -eq 0 ]; then
        say "  $description..."
        download "$url" "$output" 1
    else
        download "$url" "$output" 0
    fi
}

# Download and verify checksum
download_verified() {
    local url="$1"
    local output="$2"
    local checksum_url="$3"

    # Download the file with progress
    download_with_progress "$url" "$output" "Downloading Stratum"

    # Download checksum file (silent, it's small)
    say_verbose "  Fetching checksum..."
    local checksum_file="${output}.sha256"
    download "$checksum_url" "$checksum_file" 0

    # Verify
    start_spinner "Verifying checksum..."
    local expected_checksum
    expected_checksum="$(cat "$checksum_file" | awk '{print $1}')"
    local actual_checksum

    if command -v sha256sum >/dev/null 2>&1; then
        actual_checksum="$(sha256sum "$output" | awk '{print $1}')"
    elif command -v shasum >/dev/null 2>&1; then
        actual_checksum="$(shasum -a 256 "$output" | awk '{print $1}')"
    else
        stop_spinner clear
        say_warning "Cannot verify checksum: sha256sum/shasum not found"
        rm -f "$checksum_file"
        return 0
    fi

    stop_spinner clear

    if [ "$expected_checksum" != "$actual_checksum" ]; then
        rm -f "$output" "$checksum_file"
        err "Checksum verification failed for $output"
    fi

    rm -f "$checksum_file"
    say_success "Checksum verified"
}

# ============================================================================
# Interactive Prompts
# ============================================================================

# Check if we're running in an interactive terminal
is_interactive() {
    [ -t 0 ] && [ -t 1 ] && [ "$_YES" -eq 0 ]
}

# Prompt for yes/no confirmation
confirm() {
    local prompt="$1"
    local default="${2:-y}"

    if [ "$_YES" -eq 1 ]; then
        return 0
    fi

    if ! is_interactive; then
        # Non-interactive: use default
        [ "$default" = "y" ]
        return $?
    fi

    local yn
    if [ "$default" = "y" ]; then
        printf '%s [Y/n] ' "$prompt"
    else
        printf '%s [y/N] ' "$prompt"
    fi

    # Read from /dev/tty to handle piped scripts
    read -r yn </dev/tty || yn="$default"

    case "$yn" in
        [Yy]*) return 0 ;;
        [Nn]*) return 1 ;;
        "") [ "$default" = "y" ] ;;
        *) return 1 ;;
    esac
}

# Prompt for tier selection
select_tier() {
    if ! is_interactive; then
        return
    fi

    say ""
    say "${BOLD}Select installation tier:${RESET}"
    say "  ${CYAN}1)${RESET} Core      - CLI, REPL, type checker, VM (~15 MB)"
    say "  ${CYAN}2)${RESET} Data      - Core + DataFrame, Arrow, SQL (~45 MB)"
    say "  ${CYAN}3)${RESET} GUI       - Data + GUI framework (~80 MB)"
    say "  ${CYAN}4)${RESET} Full      - GUI + Workshop IDE, LSP (~120 MB) ${GREEN}[Recommended]${RESET}"
    say ""

    local choice
    printf "Enter choice [1-4, default=4]: "
    read -r choice </dev/tty || choice="4"

    case "$choice" in
        1) STRATUM_TIER="core" ;;
        2) STRATUM_TIER="data" ;;
        3) STRATUM_TIER="gui" ;;
        4|"") STRATUM_TIER="full" ;;
        *)
            say_warning "Invalid choice, using 'full'"
            STRATUM_TIER="full"
            ;;
    esac
}

# Prompt for installation directory
select_install_dir() {
    if ! is_interactive; then
        return
    fi

    say ""
    printf "Installation directory [%s]: " "$STRATUM_HOME"
    local dir
    read -r dir </dev/tty || dir=""

    if [ -n "$dir" ]; then
        STRATUM_HOME="$dir"
    fi
}

# ============================================================================
# Installation
# ============================================================================

# Create directory structure
create_directories() {
    say_verbose "Creating directory structure..."

    # Track the root directory for rollback (only if we're creating it)
    if [ ! -d "$STRATUM_HOME" ]; then
        track_directory "$STRATUM_HOME"
    fi

    mkdir -p "$STRATUM_HOME/bin"
    mkdir -p "$STRATUM_HOME/lib"
    mkdir -p "$STRATUM_HOME/share/completions"
    mkdir -p "$STRATUM_HOME/share/man/man1"

    say_verbose "  Created: $STRATUM_HOME"
}

# Download and extract Stratum
install_stratum() {
    say "Installing Stratum $STRATUM_VERSION ($STRATUM_TIER tier)..."

    if [ "$_DRY_RUN" -eq 1 ]; then
        say "  [DRY RUN] Would download and install to: $STRATUM_HOME"
        return
    fi

    local version="$STRATUM_VERSION"
    if [ "$version" = "latest" ]; then
        # TODO: Fetch latest version from API
        version="0.1.0"
    fi

    local archive_name="stratum-${version}-${STRATUM_TIER}-${_TARGET}.tar.gz"
    local download_url="${STRATUM_BASE_URL}/v${version}/${archive_name}"
    local checksum_url="${download_url}.sha256"

    local temp_dir
    temp_dir="$(mktemp -d)"
    local archive_path="$temp_dir/$archive_name"

    say_verbose "  Download URL: $download_url"
    say_verbose "  Temp directory: $temp_dir"

    # TODO: Uncomment when releases are available
    # download_verified "$download_url" "$archive_path" "$checksum_url"
    #
    # # Extract with progress
    # start_spinner "Extracting files..."
    # tar -xzf "$archive_path" -C "$STRATUM_HOME"
    # stop_spinner clear
    # say_success "Files extracted"

    # Placeholder: Create empty binaries for testing
    say_warning "Releases not yet available. Creating placeholder installation."

    start_spinner "Creating placeholder files..."
    sleep 0.5  # Simulate some work

    # Track files for rollback
    track_file "$STRATUM_HOME/bin/stratum"
    touch "$STRATUM_HOME/bin/stratum"
    chmod +x "$STRATUM_HOME/bin/stratum"

    # Create a placeholder script that outputs version
    cat > "$STRATUM_HOME/bin/stratum" << 'PLACEHOLDER'
#!/bin/sh
case "$1" in
    --version|-V)
        echo "stratum 0.1.0 (placeholder)"
        ;;
    completions)
        # Placeholder completion generation
        case "$2" in
            bash)
                echo "# Stratum bash completions (placeholder)"
                echo "complete -W 'run build fmt pkg workshop self repl --version --help' stratum"
                ;;
            zsh)
                echo "#compdef stratum"
                echo "# Stratum zsh completions (placeholder)"
                echo "_stratum() { _arguments ':command:(run build fmt pkg workshop self repl)' }"
                echo "_stratum"
                ;;
            fish)
                echo "# Stratum fish completions (placeholder)"
                echo "complete -c stratum -n '__fish_use_subcommand' -a 'run build fmt pkg workshop self repl'"
                ;;
        esac
        ;;
    *)
        echo "Stratum Programming Language (placeholder installation)"
        echo ""
        echo "USAGE:"
        echo "    stratum <COMMAND>"
        echo ""
        echo "COMMANDS:"
        echo "    run       Run a Stratum script"
        echo "    build     Compile a Stratum project"
        echo "    fmt       Format source files"
        echo "    pkg       Package manager"
        echo "    workshop  Open Workshop IDE"
        echo "    self      Manage Stratum installation"
        echo "    repl      Start interactive REPL"
        echo ""
        echo "OPTIONS:"
        echo "    -h, --help       Show help"
        echo "    -V, --version    Show version"
        ;;
esac
PLACEHOLDER
    chmod +x "$STRATUM_HOME/bin/stratum"

    stop_spinner clear
    say_success "Installation files created"

    # Clean up temp directory
    rm -rf "$temp_dir"
}

# Add Stratum to PATH
configure_path() {
    if [ "$_NO_PATH" -eq 1 ]; then
        say_verbose "Skipping PATH configuration (--no-path)"
        return
    fi

    local stratum_bin="$STRATUM_HOME/bin"

    # Check if already in PATH
    case ":$PATH:" in
        *":$stratum_bin:"*)
            say_verbose "Stratum already in PATH"
            return
            ;;
    esac

    say "Configuring PATH..."

    if [ "$_DRY_RUN" -eq 1 ]; then
        say "  [DRY RUN] Would add to $_SHELL_PROFILE"
        return
    fi

    local path_line
    case "$_SHELL_NAME" in
        fish)
            path_line="set -gx PATH \"$stratum_bin\" \$PATH"
            mkdir -p "$(dirname "$_SHELL_PROFILE")"
            ;;
        *)
            path_line="export PATH=\"$stratum_bin:\$PATH\""
            ;;
    esac

    # Check if line already exists
    if [ -f "$_SHELL_PROFILE" ] && grep -qF "$stratum_bin" "$_SHELL_PROFILE"; then
        say_verbose "  PATH entry already in $_SHELL_PROFILE"
        return
    fi

    # Backup the profile file for rollback
    backup_for_rollback "$_SHELL_PROFILE"

    # Add the line
    {
        echo ""
        echo "# Added by Stratum installer"
        echo "$path_line"
    } >> "$_SHELL_PROFILE"

    say_success "Added to $_SHELL_PROFILE"
}

# Install shell completions
install_completions() {
    if [ "$_NO_COMPLETIONS" -eq 1 ]; then
        say_verbose "Skipping shell completions (--no-completions)"
        return
    fi

    say "Installing shell completions..."

    if [ "$_DRY_RUN" -eq 1 ]; then
        say "  [DRY RUN] Would install completions for: $_SHELLS_DETECTED"
        return
    fi

    start_spinner "Generating shell completions..."

    for shell in $_SHELLS_DETECTED; do
        case "$shell" in
            bash)
                local bash_comp_dir="$HOME/.local/share/bash-completion/completions"
                mkdir -p "$bash_comp_dir"
                local bash_comp_file="$bash_comp_dir/stratum"
                "$STRATUM_HOME/bin/stratum" completions bash > "$bash_comp_file"
                track_file "$bash_comp_file"
                ;;
            zsh)
                local zsh_comp_dir="$HOME/.zfunc"
                mkdir -p "$zsh_comp_dir"
                local zsh_comp_file="$zsh_comp_dir/_stratum"
                "$STRATUM_HOME/bin/stratum" completions zsh > "$zsh_comp_file"
                track_file "$zsh_comp_file"
                ;;
            fish)
                local fish_comp_dir="$HOME/.config/fish/completions"
                mkdir -p "$fish_comp_dir"
                local fish_comp_file="$fish_comp_dir/stratum.fish"
                "$STRATUM_HOME/bin/stratum" completions fish > "$fish_comp_file"
                track_file "$fish_comp_file"
                ;;
        esac
    done

    stop_spinner clear
    say_success "Shell completions installed for: $_SHELLS_DETECTED"

    # Configure sourcing in shell configs if needed
    configure_completion_sourcing
}

# Check if bash-completion package is available
check_bash_completion() {
    # Check common bash-completion locations
    if [ -f /etc/bash_completion ]; then
        return 0
    fi
    if [ -f /usr/share/bash-completion/bash_completion ]; then
        return 0
    fi
    if [ -f /usr/local/share/bash-completion/bash_completion ]; then
        return 0
    fi
    # macOS Homebrew location
    if [ -f /opt/homebrew/etc/bash_completion ]; then
        return 0
    fi
    if [ -f /usr/local/etc/bash_completion ]; then
        return 0
    fi
    # Check if bash-completion is in brew
    if command -v brew >/dev/null 2>&1; then
        if brew list bash-completion >/dev/null 2>&1 || brew list bash-completion@2 >/dev/null 2>&1; then
            return 0
        fi
    fi
    return 1
}

# Configure shell completion sourcing
configure_completion_sourcing() {
    if [ "$_DRY_RUN" -eq 1 ]; then
        say "  [DRY RUN] Would configure completion sourcing"
        return
    fi

    for shell in $_SHELLS_DETECTED; do
        case "$shell" in
            bash)
                configure_bash_completion_sourcing
                ;;
            zsh)
                configure_zsh_completion_sourcing
                ;;
            fish)
                # Fish auto-loads from ~/.config/fish/completions/
                say_verbose "  Fish completions auto-loaded (no config needed)"
                ;;
        esac
    done
}

# Configure bash completion sourcing if bash-completion not installed
configure_bash_completion_sourcing() {
    local bash_comp_file="$HOME/.local/share/bash-completion/completions/stratum"

    # If bash-completion package is available, completions are auto-loaded
    if check_bash_completion; then
        say_verbose "  Bash-completion package detected (auto-loading enabled)"
        return
    fi

    # Find the appropriate bashrc file
    local bashrc=""
    if [ -f "$HOME/.bashrc" ]; then
        bashrc="$HOME/.bashrc"
    elif [ -f "$HOME/.bash_profile" ]; then
        bashrc="$HOME/.bash_profile"
    else
        bashrc="$HOME/.bashrc"
    fi

    # Check if sourcing already exists
    if [ -f "$bashrc" ] && grep -qF "stratum" "$bashrc" && grep -qF "completion" "$bashrc"; then
        say_verbose "  Bash completion sourcing already configured"
        return
    fi

    say_verbose "  Adding bash completion sourcing to $bashrc"

    # Backup for rollback
    backup_for_rollback "$bashrc"

    # Add completion sourcing
    {
        echo ""
        echo "# Stratum shell completions"
        echo "if [ -f \"$bash_comp_file\" ]; then"
        echo "    . \"$bash_comp_file\""
        echo "fi"
    } >> "$bashrc"

    say_verbose "  Bash completion sourcing configured"
}

# Configure zsh completion sourcing (fpath and compinit)
configure_zsh_completion_sourcing() {
    local zsh_comp_dir="$HOME/.zfunc"

    # Find the appropriate zshrc file
    local zshrc=""
    if [ -f "$HOME/.zshrc" ]; then
        zshrc="$HOME/.zshrc"
    elif [ -f "$HOME/.zprofile" ]; then
        zshrc="$HOME/.zprofile"
    else
        zshrc="$HOME/.zshrc"
    fi

    local needs_fpath=1
    local needs_compinit=1

    # Check if fpath already includes ~/.zfunc
    if [ -f "$zshrc" ]; then
        if grep -qE "fpath.*\.zfunc|fpath.*zfunc" "$zshrc"; then
            needs_fpath=0
            say_verbose "  Zsh fpath already includes ~/.zfunc"
        fi
        # Check if compinit is already called
        if grep -qE "^[^#]*compinit" "$zshrc"; then
            needs_compinit=0
            say_verbose "  Zsh compinit already configured"
        fi
    fi

    # Nothing to do if both are configured
    if [ "$needs_fpath" -eq 0 ] && [ "$needs_compinit" -eq 0 ]; then
        return
    fi

    say_verbose "  Configuring zsh completions in $zshrc"

    # Backup for rollback
    backup_for_rollback "$zshrc"

    # Add fpath and compinit configuration
    if [ "$needs_fpath" -eq 1 ] || [ "$needs_compinit" -eq 1 ]; then
        {
            echo ""
            echo "# Stratum shell completions"
            if [ "$needs_fpath" -eq 1 ]; then
                echo "fpath=(~/.zfunc \$fpath)"
            fi
            if [ "$needs_compinit" -eq 1 ]; then
                echo "autoload -Uz compinit && compinit"
            fi
        } >> "$zshrc"
    fi

    say_verbose "  Zsh completion sourcing configured"
}

# Record installation metadata
record_installation() {
    if [ "$_DRY_RUN" -eq 1 ]; then
        return
    fi

    local meta_file="$STRATUM_HOME/.install-meta"
    track_file "$meta_file"

    {
        echo "version=$STRATUM_VERSION"
        echo "tier=$STRATUM_TIER"
        echo "target=$_TARGET"
        echo "installed_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        echo "installer_version=1.0.0"
    } > "$meta_file"
}

# ============================================================================
# Argument Parsing
# ============================================================================

parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            -y|--yes)
                _YES=1
                ;;
            -q|--quiet)
                _QUIET=1
                ;;
            --tier=*)
                STRATUM_TIER="${1#*=}"
                ;;
            --prefix=*)
                STRATUM_HOME="${1#*=}"
                ;;
            --no-path)
                _NO_PATH=1
                ;;
            --no-completions)
                _NO_COMPLETIONS=1
                ;;
            --force)
                _FORCE=1
                ;;
            --dry-run)
                _DRY_RUN=1
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            -V|--version)
                echo "Stratum installer v1.0.0"
                exit 0
                ;;
            *)
                say_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
        shift
    done

    # Validate tier
    case "$STRATUM_TIER" in
        core|data|gui|full) ;;
        *)
            err "Invalid tier: $STRATUM_TIER. Must be one of: core, data, gui, full"
            ;;
    esac
}

show_help() {
    cat <<EOF
Stratum Programming Language Installer

USAGE:
    install.sh [OPTIONS]

OPTIONS:
    -y, --yes            Automatic yes to prompts
    -q, --quiet          Suppress non-error output
    --tier=<TIER>        Installation tier: core, data, gui, full (default: full)
    --prefix=<PATH>      Installation directory (default: ~/.stratum)
    --no-path            Don't modify PATH
    --no-completions     Don't install shell completions
    --force              Overwrite existing installation
    --dry-run            Show what would be done without making changes
    -h, --help           Show this help message
    -V, --version        Show installer version

ENVIRONMENT VARIABLES:
    STRATUM_HOME         Installation directory (same as --prefix)
    STRATUM_VERSION      Version to install (default: latest)
    STRATUM_TIER         Installation tier (same as --tier)

EXAMPLES:
    # Interactive installation
    curl -sSf https://get.stratum-lang.dev/install.sh | sh

    # Non-interactive with full tier
    curl -sSf https://get.stratum-lang.dev/install.sh | sh -s -- --yes --tier=full

    # Custom installation directory
    curl -sSf https://get.stratum-lang.dev/install.sh | sh -s -- --prefix=/opt/stratum

For more information, visit: https://stratum-lang.dev
EOF
}

# ============================================================================
# Main Entry Point
# ============================================================================

show_banner() {
    if [ "$_QUIET" -eq 1 ]; then
        return
    fi

    say ""
    say "${BOLD}╔═══════════════════════════════════════════╗${RESET}"
    say "${BOLD}║${RESET}      ${CYAN}Stratum Programming Language${RESET}         ${BOLD}║${RESET}"
    say "${BOLD}║${RESET}           Installer v1.0.0               ${BOLD}║${RESET}"
    say "${BOLD}╚═══════════════════════════════════════════╝${RESET}"
    say ""
}

show_summary() {
    say ""
    say "${BOLD}Installation Summary:${RESET}"
    say "  Platform:    $_PLATFORM ($_ARCH)"
    say "  Target:      $_TARGET"
    say "  Tier:        $STRATUM_TIER"
    say "  Directory:   $STRATUM_HOME"
    say "  Shell:       $_SHELL_NAME"
    if [ -n "$_EXISTING_INSTALL" ]; then
        say "  Existing:    $_EXISTING_VERSION ($_EXISTING_INSTALL)"
    fi
    say ""
}

show_completion_message() {
    say ""
    say "${GREEN}╔═══════════════════════════════════════════╗${RESET}"
    say "${GREEN}║${RESET}       ${BOLD}Installation Complete!${RESET}              ${GREEN}║${RESET}"
    say "${GREEN}╚═══════════════════════════════════════════╝${RESET}"
    say ""
    say "To get started, restart your shell or run:"
    say ""
    say "    ${CYAN}source $_SHELL_PROFILE${RESET}"
    say ""
    say "Then try:"
    say ""
    say "    ${CYAN}stratum --version${RESET}"
    say "    ${CYAN}stratum repl${RESET}"
    say ""
    say "Documentation: ${BLUE}https://stratum-lang.dev/docs${RESET}"
    say ""
}

main() {
    parse_args "$@"

    show_banner

    # Detection phase
    detect_platform
    detect_shells
    check_existing_installation
    verify_system_requirements

    # Handle existing installation
    if [ -n "$_EXISTING_INSTALL" ] && [ "$_FORCE" -eq 0 ]; then
        if [ "$_EXISTING_INSTALL" = "homebrew" ]; then
            err "Stratum is installed via Homebrew. Use 'brew upgrade stratum' to update."
        elif [ "$_EXISTING_INSTALL" = "apt" ] || [ "$_EXISTING_INSTALL" = "rpm" ]; then
            err "Stratum is installed via system package manager. Use your package manager to update."
        else
            if ! confirm "Existing installation found ($_EXISTING_VERSION). Overwrite?"; then
                say "Installation cancelled."
                exit 0
            fi
        fi
    fi

    # Interactive prompts
    if is_interactive; then
        select_tier
        select_install_dir
    fi

    # Show summary and confirm
    show_summary

    if is_interactive && ! confirm "Proceed with installation?"; then
        say "Installation cancelled."
        exit 0
    fi

    # Enable rollback for the installation phase
    # If anything fails from here, we'll clean up automatically
    if [ "$_DRY_RUN" -eq 0 ]; then
        enable_rollback
    fi

    # Installation phase
    create_directories
    install_stratum
    configure_path
    install_completions
    record_installation

    # Disable rollback - installation succeeded!
    if [ "$_DRY_RUN" -eq 0 ]; then
        disable_rollback
    fi

    # Done!
    show_completion_message
}

# Execute main function (this line must be last for security)
main "$@"
