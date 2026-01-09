# Changelog

All notable changes to the Stratum VS Code extension will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-01-08

### Added

- **Language Server Protocol (LSP) Integration**
  - IntelliSense with smart completions
  - Hover information for symbols
  - Signature help for function calls
  - Go to definition
  - Find all references
  - Rename symbol across files
  - Real-time diagnostics and error reporting
  - Code actions and quick fixes
  - Document outline and symbols

- **Debug Adapter Protocol (DAP) Support**
  - Breakpoint support
  - Step into, step over, step out
  - Variable inspection in debug panel
  - Stack frame navigation
  - Thread support

- **Task Provider**
  - Auto-detection of `stratum.toml` projects
  - Run, build, test, and format tasks
  - Debug and release build configurations
  - Problem matchers for compiler output

- **Syntax Highlighting**
  - Full TextMate grammar for `.strat` files
  - Support for all language constructs

- **Code Snippets**
  - 17+ snippets for common patterns
  - Function definitions, control flow, data operations

- **Editor Configuration**
  - Auto-closing brackets and quotes
  - Comment toggling support
  - Indentation rules
  - Format on save (configurable)

### Configuration Options

- `stratum.server.path` - Path to Stratum executable
- `stratum.server.args` - Language server arguments
- `stratum.trace.server` - Server communication tracing
- `stratum.format.onSave` - Auto-format on save
