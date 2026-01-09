# Stratum Language Support for VS Code

Provides comprehensive language support for [Stratum](https://github.com/horizon-analytic-studios/stratum), a Goldilocks programming language - easier than Rust, more structured than Python.

## Features

### Language Intelligence
- **Syntax highlighting** - Full TextMate grammar for `.strat` files
- **IntelliSense** - Smart completions, hover info, signature help
- **Go to definition** - Jump to function and type definitions
- **Find references** - Find all usages of a symbol
- **Rename symbol** - Refactor names across files
- **Code formatting** - Auto-format on save
- **Diagnostics** - Real-time errors and warnings
- **Code actions** - Quick fixes and refactorings
- **Document outline** - Navigate symbols in the current file

### Debugging
- **Breakpoints** - Set breakpoints in your code
- **Step debugging** - Step into, over, and out
- **Variable inspection** - View local and global variables
- **Stack frames** - Navigate the call stack
- **Debug console** - Evaluate expressions during debugging

### Task Integration
- **Auto-detected tasks** - Automatically discovers `stratum.toml` projects
- **Run tasks** - Execute your Stratum programs
- **Build tasks** - Compile with debug or release mode
- **Test tasks** - Run your test suites
- **Format tasks** - Format source files
- **Problem matchers** - Parse compiler errors for quick navigation

### Code Snippets
17+ snippets for common patterns - see the Snippets section below.

## Installation

### Via Stratum CLI (Recommended)

The easiest way to install the extension is using the Stratum CLI:

```bash
# Install the extension
stratum extension install

# Check if installed
stratum extension list

# Uninstall
stratum extension uninstall
```

### From VSIX File

Download or build the VSIX file and install it:

```bash
# Install from a specific VSIX file
stratum extension install --vsix /path/to/stratum.vsix

# Or use VS Code directly
code --install-extension stratum.vsix
```

### Building from Source

```bash
cd editors/vscode/stratum
npm install
npm run package   # Creates stratum.vsix
```

## Requirements

The Stratum CLI must be installed and available in your PATH:

```bash
# Build from source
cargo install --path crates/stratum-cli

# Or install from a release
# (Download from GitHub releases)
```

The extension uses:
- `stratum lsp` for language intelligence
- `stratum dap` for debugging
- `stratum run`, `stratum build`, etc. for tasks

## Extension Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `stratum.server.path` | `"stratum"` | Path to the Stratum executable |
| `stratum.server.args` | `["lsp"]` | Arguments for the language server |
| `stratum.trace.server` | `"off"` | Trace communication with language server (`off`, `messages`, `verbose`) |
| `stratum.format.onSave` | `true` | Format files on save |

## Debugging

### Quick Start

1. Open a `.strat` file
2. Set breakpoints by clicking in the gutter
3. Press F5 or use **Run > Start Debugging**
4. Select "Stratum Debug" configuration

### Debug Configuration

The extension provides a default debug configuration. You can customize it in `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "stratum",
      "request": "launch",
      "name": "Debug Current File",
      "program": "${file}",
      "stopOnEntry": false,
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

## Tasks

The extension automatically provides tasks for projects with a `stratum.toml` manifest:

| Task | Description |
|------|-------------|
| **Stratum: Run** | Run `main.strat` |
| **Stratum: Build** | Build with debug settings |
| **Stratum: Build (Release)** | Build with optimizations |
| **Stratum: Test** | Run tests |
| **Stratum: Format** | Format all source files |

Access tasks via **Terminal > Run Task** or the Command Palette.

## Snippets

| Prefix | Description |
|--------|-------------|
| `fx` | Function definition |
| `fxr` | Function with return type |
| `fxa` | Async function |
| `let` | Variable declaration |
| `if` / `ife` | If / if-else statement |
| `for` / `forr` | For loop / for range |
| `while` | While loop |
| `match` | Match expression |
| `struct` | Struct definition |
| `enum` | Enum definition |
| `interface` | Interface definition |
| `impl` | Implementation block |
| `try` | Try-catch block |
| `test` | Test function |
| `pipe` | Pipeline expression |
| `data` | DataFrame creation |

## Commands

| Command | Description |
|---------|-------------|
| **Stratum: Restart Language Server** | Restart the language server if it becomes unresponsive |

## Troubleshooting

### Language server not starting

1. Ensure `stratum` is in your PATH: `which stratum`
2. Check the extension settings for the correct path
3. Look at the Output panel (View > Output > Stratum Language Server)

### Debugging not working

1. Ensure `stratum dap` works from the command line
2. Check that breakpoints are set on executable lines
3. Look at the Debug Console for error messages

### Tasks not appearing

1. Ensure your project has a `stratum.toml` file
2. Run **Tasks: Refresh** from the Command Palette
3. Check that the manifest is valid

## Development

```bash
cd editors/vscode/stratum
npm install
npm run compile    # Compile TypeScript
npm run watch      # Watch mode
npm run lint       # Run linter
npm run test       # Run tests
npm run package    # Build VSIX
```

To test the extension:
1. Open this folder in VS Code
2. Press F5 to launch Extension Development Host
3. Open a `.strat` file

## License

MIT
