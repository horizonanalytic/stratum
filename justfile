# Stratum Development Commands

# Build all crates
build:
    cargo build --all

# Run all tests
test:
    cargo test --all

# Check without building
check:
    cargo check --all

# Lint with clippy
lint:
    cargo clippy --all -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Format check (CI)
fmt-check:
    cargo fmt --all -- --check

# Run the CLI
run *ARGS:
    cargo run --bin stratum -- {{ARGS}}

# Clean build artifacts
clean:
    cargo clean

# Full CI check (format, lint, test)
ci: fmt-check lint test
