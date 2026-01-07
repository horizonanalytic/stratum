# Stratum Language Support for VS Code

Provides language support for [Stratum](https://github.com/horizon-analytic-studios/stratum), a Goldilocks programming language - easier than Rust, more structured than Python.

## Features

- Syntax highlighting
- IntelliSense (completions, hover, signatures)
- Go to definition
- Find references
- Rename symbol
- Code formatting
- Diagnostics (errors and warnings)
- Code actions and quick fixes
- Document outline
- Snippets

## Requirements

The Stratum CLI must be installed and available in your PATH. The extension uses the built-in language server (`stratum lsp`).

### Installing Stratum

```bash
# Build from source
cargo install --path crates/stratum-cli
```

## Extension Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `stratum.server.path` | `"stratum"` | Path to the Stratum executable |
| `stratum.server.args` | `["lsp"]` | Arguments for the language server |
| `stratum.trace.server` | `"off"` | Trace communication with language server |
| `stratum.format.onSave` | `true` | Format files on save |

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

## Commands

- **Stratum: Restart Language Server** - Restart the language server

## Development

```bash
cd editors/vscode/stratum
npm install
npm run compile
```

To test the extension:
1. Open this folder in VS Code
2. Press F5 to launch Extension Development Host
3. Open a `.strat` file

## License

MIT
