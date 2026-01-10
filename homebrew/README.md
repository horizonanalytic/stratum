# Homebrew Tap for Stratum

This is the official [Homebrew](https://brew.sh/) tap for [Stratum](https://stratum-lang.dev), a Goldilocks programming language with native data operations and GUI support.

## Installation

```bash
# Add the tap
brew tap horizon-analytic/stratum

# Install Stratum (Data tier - default)
brew install stratum

# Or install directly
brew install horizon-analytic/stratum/stratum
```

### Installation Tiers

Stratum supports tiered installation to balance features vs. binary size:

| Tier | Command | Size | Includes |
|------|---------|------|----------|
| **Data** | `brew install stratum` | ~45 MB | CLI, REPL, DataFrame, Arrow, SQL |
| **GUI** | `brew install stratum --with-gui` | ~80 MB | Data + GUI framework |
| **Full** | `brew install stratum --with-full` | ~120 MB | GUI + Workshop IDE, LSP |

```bash
# Install with GUI framework
brew install stratum --with-gui

# Install with everything (Workshop IDE, LSP)
brew install stratum --with-full
```

To change tiers after installation:
```bash
brew reinstall stratum --with-full
```

## Updating

```bash
brew update
brew upgrade stratum
```

## Uninstalling

```bash
brew uninstall stratum
brew untap horizon-analytic/stratum  # Optional: remove the tap
```

## Development Build

To install from the latest main branch:

```bash
brew install --HEAD stratum
```

## Included Components

All tiers include:
- `stratum` CLI with REPL, compiler, and runtime
- Type checker and formatter
- DataFrame, Arrow, and SQL support
- Shell completions (bash, zsh, fish)

**GUI tier** adds:
- GUI framework for native desktop applications

**Full tier** adds:
- Workshop IDE (integrated development environment)
- LSP server for editor integration

## Shell Completions

Shell completions are automatically installed. To verify they're working:

**Bash** (add to `~/.bashrc` if not using bash-completion):
```bash
source $(brew --prefix)/etc/bash_completion.d/stratum
```

**Zsh** (completions are auto-loaded):
```zsh
# Verify with:
type _stratum
```

**Fish** (completions are auto-loaded):
```fish
# Verify with:
complete -c stratum
```

## Troubleshooting

### Formula won't install
```bash
brew update
brew doctor
```

### Completions not working
Ensure your shell is configured to use Homebrew completions:

```bash
# For bash (in ~/.bashrc):
if type brew &>/dev/null; then
  HOMEBREW_PREFIX="$(brew --prefix)"
  if [[ -r "${HOMEBREW_PREFIX}/etc/profile.d/bash_completion.sh" ]]; then
    source "${HOMEBREW_PREFIX}/etc/profile.d/bash_completion.sh"
  fi
fi

# For zsh (in ~/.zshrc):
if type brew &>/dev/null; then
  FPATH="$(brew --prefix)/share/zsh/site-functions:${FPATH}"
  autoload -Uz compinit && compinit
fi
```

## License

MIT License - see the main [Stratum repository](https://github.com/horizon-analytic/stratum) for details.
