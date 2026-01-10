#!/bin/sh
# Stratum Programming Language Uninstaller
#
# Usage:
#   # Download and run (for broken installations):
#   curl -fsSL https://raw.githubusercontent.com/horizon-analytic/stratum/main/scripts/uninstall.sh | sh
#
#   # With options:
#   curl -fsSL ... | sh -s -- --purge --yes
#
#   # Or run locally:
#   ./uninstall.sh [OPTIONS]
#
# Options:
#   --yes, -y         Skip confirmation prompts
#   --purge           Remove all user data, including config and Workshop IDE data
#   --dry-run         Show what would be removed without making changes
#   --quiet, -q       Suppress non-essential output
#   --help, -h        Show this help message
#
# This script works even when the stratum binary is corrupted or missing.
# It mirrors the functionality of 'stratum self uninstall'.
#
# This script wraps all code in functions with main() called at the end
# to prevent partial download execution (security best practice).

set -eu

# ============================================================================
# Configuration
# ============================================================================

STRATUM_HOME="${STRATUM_HOME:-$HOME/.stratum}"

# Command-line options
_YES=0
_PURGE=0
_DRY_RUN=0
_QUIET=0

# Detected state
_PLATFORM=""
_INSTALL_METHOD=""

# Statistics
_REMOVED_COUNT=0
_WARNING_COUNT=0

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
    printf '%b\n' "${GREEN}[ok]${RESET} $1"
}

say_warning() {
    printf '%b\n' "${YELLOW}[!]${RESET} $1" >&2
    _WARNING_COUNT=$((_WARNING_COUNT + 1))
}

say_error() {
    printf '%b\n' "${RED}[x]${RESET} $1" >&2
}

err() {
    say_error "$1"
    exit 1
}

# Prompt for confirmation
confirm() {
    local message="$1"

    if [ "$_YES" -eq 1 ]; then
        return 0
    fi

    if [ ! -t 0 ]; then
        # Non-interactive, default to no
        return 1
    fi

    printf '%b' "${BOLD}$message${RESET} [y/N] "
    read -r response
    case "$response" in
        [Yy]|[Yy][Ee][Ss])
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

# ============================================================================
# Platform Detection
# ============================================================================

detect_platform() {
    case "$(uname -s)" in
        Darwin)
            _PLATFORM="macos"
            ;;
        Linux)
            _PLATFORM="linux"
            ;;
        FreeBSD)
            _PLATFORM="freebsd"
            ;;
        *)
            _PLATFORM="unknown"
            ;;
    esac
}

# ============================================================================
# Installation Method Detection
# ============================================================================

detect_install_method() {
    _INSTALL_METHOD=""

    # Check 1: Script installation (has .install-meta file)
    if [ -f "$STRATUM_HOME/.install-meta" ]; then
        _INSTALL_METHOD="script"
        return
    fi

    # Check 2: Homebrew
    if command -v brew >/dev/null 2>&1; then
        if brew list stratum >/dev/null 2>&1; then
            _INSTALL_METHOD="homebrew"
            return
        fi
    fi

    # Check 3: apt/dpkg (Debian/Ubuntu)
    if command -v dpkg >/dev/null 2>&1; then
        if dpkg -s stratum >/dev/null 2>&1; then
            _INSTALL_METHOD="apt"
            return
        fi
    fi

    # Check 4: rpm (Fedora/RHEL)
    if command -v rpm >/dev/null 2>&1; then
        if rpm -q stratum >/dev/null 2>&1; then
            _INSTALL_METHOD="rpm"
            return
        fi
    fi

    # Check 5: Cargo install
    if command -v cargo >/dev/null 2>&1; then
        if cargo install --list 2>/dev/null | grep -q "^stratum-cli"; then
            _INSTALL_METHOD="cargo"
            return
        fi
    fi

    # Check 6: Binary exists in STRATUM_HOME
    if [ -f "$STRATUM_HOME/bin/stratum" ]; then
        _INSTALL_METHOD="script"
        return
    fi

    # Check 7: Binary in PATH
    if command -v stratum >/dev/null 2>&1; then
        _INSTALL_METHOD="unknown"
        return
    fi

    # No installation found
    _INSTALL_METHOD="none"
}

# ============================================================================
# Path Helpers
# ============================================================================

# Get shell profile paths to check
get_shell_profile_paths() {
    echo "$HOME/.bashrc"
    echo "$HOME/.bash_profile"
    echo "$HOME/.profile"
    echo "$HOME/.zshrc"
    echo "$HOME/.zprofile"
    echo "$HOME/.config/fish/conf.d/stratum.fish"
    echo "$HOME/.config/fish/config.fish"
}

# Get shell completion paths
get_completion_paths() {
    # User-level completions
    echo "$HOME/.local/share/bash-completion/completions/stratum"
    echo "$HOME/.zfunc/_stratum"
    echo "$HOME/.config/fish/completions/stratum.fish"
    # System-level completions
    echo "/usr/local/share/zsh/site-functions/_stratum"
}

# Get user data paths (for --purge)
get_user_data_paths() {
    echo "$STRATUM_HOME/packages|installed packages"
    echo "$STRATUM_HOME/cache|build cache"
    echo "$STRATUM_HOME/history|REPL history"
    echo "$STRATUM_HOME/.repl_history|REPL history (legacy)"
    echo "$STRATUM_HOME/lsp-cache|LSP cache"
    echo "$STRATUM_HOME/versions|installed versions"
    echo "$STRATUM_HOME/.active-version|active version marker"
}

# Get Workshop IDE paths (platform-specific, for --purge)
get_workshop_ide_paths() {
    case "$_PLATFORM" in
        macos)
            echo "$HOME/Library/Application Support/Stratum Workshop|Workshop IDE settings"
            echo "$HOME/Library/Preferences/dev.stratum-lang.workshop.plist|Workshop IDE preferences"
            echo "$HOME/Library/Caches/dev.stratum-lang.workshop|Workshop IDE cache"
            echo "$HOME/Library/Logs/Stratum Workshop|Workshop IDE logs"
            echo "$HOME/Library/Saved Application State/dev.stratum-lang.workshop.savedState|Workshop IDE saved state"
            ;;
        linux)
            local config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
            local data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
            local cache_home="${XDG_CACHE_HOME:-$HOME/.cache}"

            echo "$config_home/stratum-workshop|Workshop IDE settings"
            echo "$data_home/stratum-workshop|Workshop IDE data"
            echo "$cache_home/stratum-workshop|Workshop IDE cache"
            ;;
    esac
}

# ============================================================================
# Removal Functions
# ============================================================================

# Remove a file or directory
remove_item() {
    local path="$1"
    local desc="${2:-}"

    if [ ! -e "$path" ] && [ ! -L "$path" ]; then
        return 0
    fi

    if [ "$_DRY_RUN" -eq 1 ]; then
        say "  [DRY RUN] Would remove: $path"
        return 0
    fi

    if [ -d "$path" ]; then
        if rm -rf "$path" 2>/dev/null; then
            _REMOVED_COUNT=$((_REMOVED_COUNT + 1))
            say_verbose "  Removed directory: $path"
        else
            say_warning "Failed to remove directory: $path (permission denied?)"
        fi
    else
        if rm -f "$path" 2>/dev/null; then
            _REMOVED_COUNT=$((_REMOVED_COUNT + 1))
            say_verbose "  Removed file: $path"
        else
            say_warning "Failed to remove file: $path (permission denied?)"
        fi
    fi
}

# Clean stratum entries from a shell profile
clean_shell_profile() {
    local profile="$1"

    if [ ! -f "$profile" ]; then
        return 0
    fi

    local content
    content="$(cat "$profile")"

    # Check if file contains any Stratum-related entries
    if ! echo "$content" | grep -qi "stratum"; then
        return 0
    fi

    if [ "$_DRY_RUN" -eq 1 ]; then
        say "  [DRY RUN] Would clean: $profile"
        return 0
    fi

    # Create temporary file for filtered content
    local temp_file
    temp_file="$(mktemp)"

    # Filter out Stratum-related lines
    # Handles:
    # - export STRATUM_HOME=...
    # - export PATH="$STRATUM_HOME/bin:$PATH" or similar
    # - source completions
    # - # Stratum comments
    # - Fish-specific: set -x STRATUM_HOME ...
    local stratum_bin="$STRATUM_HOME/bin"
    local prev_blank=0

    echo "$content" | while IFS= read -r line || [ -n "$line" ]; do
        local line_lower
        line_lower="$(echo "$line" | tr '[:upper:]' '[:lower:]')"
        local line_trimmed
        line_trimmed="$(echo "$line" | sed 's/^[[:space:]]*//' | sed 's/[[:space:]]*$//')"

        local skip=0

        # Skip comment lines about Stratum
        if echo "$line_trimmed" | grep -q '^#' && echo "$line_lower" | grep -q "stratum"; then
            skip=1
        fi

        # Skip STRATUM_HOME exports
        if echo "$line_lower" | grep -q "stratum_home"; then
            skip=1
        fi

        # Skip PATH modifications containing stratum bin
        if echo "$line_lower" | grep -q "$(echo "$stratum_bin" | tr '[:upper:]' '[:lower:]')"; then
            skip=1
        fi

        # Skip fish set commands for stratum
        if echo "$line_trimmed" | grep -q '^set ' && echo "$line_lower" | grep -q "stratum"; then
            skip=1
        fi

        # Skip source commands for stratum completions
        if echo "$line_trimmed" | grep -qE '^(source |\. )' && echo "$line_lower" | grep -q "stratum"; then
            skip=1
        fi

        # Skip fpath modifications for stratum completions
        if echo "$line_lower" | grep -q "fpath" && echo "$line_lower" | grep -q "stratum"; then
            skip=1
        fi

        # Skip PATH lines containing stratum
        if echo "$line_lower" | grep -q "export path=.*stratum"; then
            skip=1
        fi

        if [ "$skip" -eq 0 ]; then
            # Handle consecutive blank lines (allow max 1)
            if [ -z "$line_trimmed" ]; then
                if [ "$prev_blank" -eq 0 ]; then
                    echo "$line"
                    prev_blank=1
                fi
            else
                echo "$line"
                prev_blank=0
            fi
        fi
    done > "$temp_file"

    # Check if content changed
    if ! cmp -s "$profile" "$temp_file"; then
        if mv "$temp_file" "$profile" 2>/dev/null; then
            _REMOVED_COUNT=$((_REMOVED_COUNT + 1))
            say_verbose "  Cleaned profile: $profile"
        else
            say_warning "Failed to update profile: $profile (permission denied?)"
            rm -f "$temp_file"
        fi
    else
        rm -f "$temp_file"
    fi
}

# ============================================================================
# Main Uninstall Logic
# ============================================================================

show_what_will_be_removed() {
    say "\nThe following will be removed:\n"

    # Core installation
    say "  Core installation:"
    for item in bin lib share .install-meta; do
        local path="$STRATUM_HOME/$item"
        if [ -e "$path" ]; then
            say "    - $path"
        fi
    done

    # Shell profiles (that contain stratum entries)
    local has_profile_entries=0
    for profile in $(get_shell_profile_paths); do
        if [ -f "$profile" ] && grep -qi "stratum" "$profile" 2>/dev/null; then
            if [ "$has_profile_entries" -eq 0 ]; then
                say "\n  Shell profile modifications (PATH, STRATUM_HOME):"
                has_profile_entries=1
            fi
            say "    - $profile"
        fi
    done

    # Shell completions
    local has_completions=0
    for comp in $(get_completion_paths); do
        if [ -e "$comp" ]; then
            if [ "$has_completions" -eq 0 ]; then
                say "\n  Shell completions:"
                has_completions=1
            fi
            say "    - $comp"
        fi
    done

    # Purge-only items
    if [ "$_PURGE" -eq 1 ]; then
        # User data
        local has_user_data=0
        for entry in $(get_user_data_paths); do
            local path="${entry%%|*}"
            local desc="${entry##*|}"
            if [ -e "$path" ]; then
                if [ "$has_user_data" -eq 0 ]; then
                    say "\n  User data (--purge):"
                    has_user_data=1
                fi
                say "    - $path ($desc)"
            fi
        done

        # Config file
        if [ -f "$STRATUM_HOME/config.toml" ]; then
            if [ "$has_user_data" -eq 0 ]; then
                say "\n  User data (--purge):"
            fi
            say "    - $STRATUM_HOME/config.toml (user configuration)"
        fi

        # Workshop IDE data
        local has_workshop=0
        for entry in $(get_workshop_ide_paths); do
            local path="${entry%%|*}"
            local desc="${entry##*|}"
            if [ -e "$path" ]; then
                if [ "$has_workshop" -eq 0 ]; then
                    say "\n  Workshop IDE data:"
                    has_workshop=1
                fi
                say "    - $path ($desc)"
            fi
        done
    else
        # Show preserved items
        local has_preserved=0

        if [ -f "$STRATUM_HOME/config.toml" ]; then
            if [ "$has_preserved" -eq 0 ]; then
                say "\n  Note: The following will be preserved:"
                has_preserved=1
            fi
            say "    - config.toml (user configuration)"
        fi

        for entry in $(get_user_data_paths); do
            local path="${entry%%|*}"
            local desc="${entry##*|}"
            if [ -e "$path" ]; then
                if [ "$has_preserved" -eq 0 ]; then
                    say "\n  Note: The following will be preserved:"
                    has_preserved=1
                fi
                local name
                name="$(basename "$path")"
                say "    - $name ($desc)"
            fi
        done

        if [ "$has_preserved" -eq 1 ]; then
            say "        Use --purge to remove all user data"
        fi

        # Workshop IDE preserved
        local has_workshop_preserved=0
        for entry in $(get_workshop_ide_paths); do
            local path="${entry%%|*}"
            local desc="${entry##*|}"
            if [ -e "$path" ]; then
                if [ "$has_workshop_preserved" -eq 0 ]; then
                    say "\n  Note: Workshop IDE data will be preserved:"
                    has_workshop_preserved=1
                fi
                say "    - $path ($desc)"
            fi
        done
    fi

    say ""
}

perform_uninstall() {
    say "\nUninstalling Stratum...\n"

    # Clean shell profiles
    printf "Cleaning shell profiles... "
    local profiles_cleaned=0
    for profile in $(get_shell_profile_paths); do
        if [ -f "$profile" ] && grep -qi "stratum" "$profile" 2>/dev/null; then
            clean_shell_profile "$profile"
            profiles_cleaned=$((profiles_cleaned + 1))
        fi
    done
    if [ "$profiles_cleaned" -eq 0 ]; then
        say "no changes needed"
    else
        say "done ($profiles_cleaned files updated)"
    fi

    # Remove shell completions
    printf "Removing shell completions... "
    local completions_removed=0
    for comp in $(get_completion_paths); do
        if [ -e "$comp" ]; then
            remove_item "$comp"
            completions_removed=$((completions_removed + 1))
        fi
    done
    if [ "$completions_removed" -eq 0 ]; then
        say "none found"
    else
        say "done ($completions_removed files removed)"
    fi

    # Remove core installation (but preserve user data if not purging)
    printf "Removing core installation... "
    for item in bin lib share .install-meta; do
        local path="$STRATUM_HOME/$item"
        if [ -e "$path" ]; then
            remove_item "$path"
        fi
    done
    say "done"

    # Remove user data if purging
    if [ "$_PURGE" -eq 1 ]; then
        printf "Removing user data... "
        for entry in $(get_user_data_paths); do
            local path="${entry%%|*}"
            if [ -e "$path" ]; then
                remove_item "$path"
            fi
        done

        # Config file
        if [ -f "$STRATUM_HOME/config.toml" ]; then
            remove_item "$STRATUM_HOME/config.toml"
        fi
        say "done"

        # Workshop IDE data
        printf "Removing Workshop IDE data... "
        local workshop_removed=0
        for entry in $(get_workshop_ide_paths); do
            local path="${entry%%|*}"
            if [ -e "$path" ]; then
                remove_item "$path"
                workshop_removed=$((workshop_removed + 1))
            fi
        done
        if [ "$workshop_removed" -eq 0 ]; then
            say "none found"
        else
            say "done"
        fi
    fi

    # Try to remove STRATUM_HOME if it's empty (or only has preserved user data)
    if [ "$_PURGE" -eq 1 ]; then
        if [ -d "$STRATUM_HOME" ]; then
            # Check if directory is empty
            if [ -z "$(ls -A "$STRATUM_HOME" 2>/dev/null)" ]; then
                remove_item "$STRATUM_HOME"
            fi
        fi
    fi
}

show_completion_summary() {
    say ""

    if [ "$_DRY_RUN" -eq 1 ]; then
        say "${BOLD}Dry run complete.${RESET}"
        say "No files were modified. Run without --dry-run to perform uninstall."
    else
        if [ "$_WARNING_COUNT" -gt 0 ]; then
            say "${YELLOW}Uninstall completed with $_WARNING_COUNT warning(s).${RESET}"
        else
            say "${GREEN}Stratum has been successfully uninstalled.${RESET}"
        fi

        if [ "$_PURGE" -eq 0 ]; then
            say "\nUser data was preserved in $STRATUM_HOME"
            say "To remove it: rm -rf $STRATUM_HOME"
        fi

        say "\nPlease reload your shell configuration:"
        say "  ${CYAN}source ~/.bashrc${RESET}  # for Bash"
        say "  ${CYAN}source ~/.zshrc${RESET}   # for Zsh"
        say "  ${CYAN}exec fish${RESET}         # for Fish"
        say "\nOr start a new terminal session."
    fi
}

# ============================================================================
# Main Entry Point
# ============================================================================

show_help() {
    cat << 'EOF'
Stratum Uninstaller

USAGE:
    uninstall.sh [OPTIONS]

OPTIONS:
    --yes, -y         Skip confirmation prompts
    --purge           Remove all user data, including config and Workshop IDE data
    --dry-run         Show what would be removed without making changes
    --quiet, -q       Suppress non-essential output
    --help, -h        Show this help message

EXAMPLES:
    # Interactive uninstall (preserves user data)
    ./uninstall.sh

    # Non-interactive uninstall
    ./uninstall.sh --yes

    # Complete removal including all user data
    ./uninstall.sh --purge --yes

    # See what would be removed
    ./uninstall.sh --dry-run

ENVIRONMENT:
    STRATUM_HOME      Installation directory (default: ~/.stratum)

This script works even when the stratum binary is corrupted or missing.
It mirrors the functionality of 'stratum self uninstall'.
EOF
}

parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            --yes|-y)
                _YES=1
                ;;
            --purge)
                _PURGE=1
                ;;
            --dry-run)
                _DRY_RUN=1
                ;;
            --quiet|-q)
                _QUIET=1
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                say_error "Unknown option: $1"
                say "Use --help for usage information."
                exit 1
                ;;
        esac
        shift
    done
}

main() {
    parse_args "$@"

    say "${BOLD}Stratum Uninstaller${RESET}\n"

    # Detect platform
    detect_platform

    # Detect installation method
    detect_install_method

    case "$_INSTALL_METHOD" in
        homebrew)
            say "Stratum was installed via Homebrew."
            say "Please use: ${CYAN}brew uninstall stratum${RESET}"
            say "\nNote: User data in ~/.stratum will not be removed."
            say "To remove it manually: rm -rf ~/.stratum"
            if [ "$_PURGE" -eq 1 ]; then
                case "$_PLATFORM" in
                    macos)
                        say "\nTo also remove Workshop IDE data on macOS:"
                        say "  rm -rf ~/Library/Application\\ Support/Stratum\\ Workshop"
                        say "  rm -rf ~/Library/Caches/dev.stratum-lang.workshop"
                        ;;
                    linux)
                        say "\nTo also remove Workshop IDE data on Linux:"
                        say "  rm -rf ~/.config/stratum-workshop"
                        say "  rm -rf ~/.local/share/stratum-workshop"
                        say "  rm -rf ~/.cache/stratum-workshop"
                        ;;
                esac
            fi
            exit 0
            ;;
        apt)
            say "Stratum was installed via apt/dpkg."
            say "Please use: ${CYAN}sudo apt remove stratum${RESET}"
            if [ "$_PURGE" -eq 1 ]; then
                say "         or: ${CYAN}sudo apt purge stratum${RESET}"
                say "\nNote: To also remove user data:"
                say "  rm -rf ~/.stratum"
                say "  rm -rf ~/.config/stratum-workshop"
                say "  rm -rf ~/.local/share/stratum-workshop"
            fi
            exit 0
            ;;
        rpm)
            say "Stratum was installed via rpm."
            say "Please use: ${CYAN}sudo dnf remove stratum${RESET}"
            if [ "$_PURGE" -eq 1 ]; then
                say "\nNote: To also remove user data:"
                say "  rm -rf ~/.stratum"
                say "  rm -rf ~/.config/stratum-workshop"
                say "  rm -rf ~/.local/share/stratum-workshop"
            fi
            exit 0
            ;;
        cargo)
            say "Stratum was installed via cargo install."
            say "Please use: ${CYAN}cargo uninstall stratum-cli${RESET}"
            say "\nNote: User data in ~/.stratum will not be removed."
            say "To remove it manually: rm -rf ~/.stratum"
            exit 0
            ;;
        none)
            say_warning "No Stratum installation found."
            say "Checked locations:"
            say "  - $STRATUM_HOME"
            say "  - Homebrew, apt, rpm, cargo"
            say "  - PATH"
            say "\nIf you have a custom installation, set STRATUM_HOME and run again:"
            say "  STRATUM_HOME=/path/to/stratum ./uninstall.sh"
            exit 1
            ;;
        script|unknown)
            # Continue with uninstall
            ;;
    esac

    # Show what will be removed
    show_what_will_be_removed

    # Confirm uninstall
    local confirm_message
    if [ "$_PURGE" -eq 1 ]; then
        confirm_message="This will remove Stratum and ALL user data including Workshop IDE settings. Continue?"
    else
        confirm_message="This will remove Stratum. User configuration will be preserved. Continue?"
    fi

    if ! confirm "$confirm_message"; then
        say "\nUninstall cancelled."
        exit 0
    fi

    # Perform uninstall
    perform_uninstall

    # Show summary
    show_completion_summary
}

# Only run main if this script is being executed directly (not sourced)
# This allows for testing individual functions
if [ "${STRATUM_UNINSTALL_SOURCED:-0}" -eq 0 ]; then
    main "$@"
fi
