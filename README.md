# Stratum

A "Goldilocks" programming language: easier than Rust, more structured than Python.

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)
[![Build Status](https://github.com/horizonanalytic/stratum/actions/workflows/ci.yml/badge.svg)](https://github.com/horizonanalytic/stratum/actions)

## Overview

Stratum is designed for developers who want:
- **Approachable complexity** - Learn in weeks, not months
- **Hybrid execution** - You control what gets compiled vs interpreted
- **Ultra-fast data** - Built-in Arrow-backed DataFrame with SIMD acceleration
- **Native GUI** - First-class declarative UI components
- **Bundled IDE** - "Stratum Workshop" ships with the language

## Quick Example

```stratum
fx greet(name: String) -> String {
    "Hello, {name}!"
}

fx main() {
    let message = greet("Stratum")
    print(message)
}
```

## Key Features

| Feature | Design |
|---------|--------|
| Functions | `fx` keyword (math-friendly, like f(x)) |
| Types | Static with full inference |
| Nullability | `?` suffix (`User?` for nullable) |
| Errors | Exceptions by default, optional `Result` |
| Memory | Invisible automatic reference counting |
| Data | Pipeline operator `\|>` for data flow |

## File Extensions

- `.strat` - Source files
- `.stratum` - Compiled binaries

## Project Status

**Status:** In Development (Phase 1: Project Setup)

See [planning/00-overview.md](planning/00-overview.md) for development roadmap.

## Building from Source

```bash
# Clone the repository
git clone git@github.com:horizonanalytic/stratum.git
cd stratum

# Build (requires Rust 1.75+)
cargo build --all

# Run tests
cargo test --all
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
