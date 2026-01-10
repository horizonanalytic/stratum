# Stratum Docker Images

Official Docker images for the Stratum programming language.

## Available Images

| Tag | Base | Size (est.) | Features |
|-----|------|-------------|----------|
| `stratum:core` | Alpine 3.19 | ~50 MB | CLI, REPL, compiler, DataFrame, Arrow, SQL |
| `stratum:data` | Alpine 3.19 | ~50 MB | Same as core (alias) |
| `stratum:latest` | Alpine 3.19 | ~50 MB | Same as data (default) |
| `stratum:full` | Debian slim | ~100 MB | Core + LSP server |

## Quick Start

```bash
# Run a Stratum script
docker run --rm -v $(pwd):/app stratum run script.strat

# Interactive REPL
docker run --rm -it stratum repl

# Evaluate an expression
docker run --rm stratum eval "1 + 2 * 3"

# Check version
docker run --rm stratum --version
```

## Building Locally

From the repository root:

```bash
# Build core image
docker build -f docker/Dockerfile.core -t stratum:core .

# Build full image (includes LSP)
docker build -f docker/Dockerfile.full -t stratum:full .

# Tag aliases
docker tag stratum:core stratum:data
docker tag stratum:core stratum:latest
```

## Multi-Architecture Builds

Build for both amd64 and arm64:

```bash
# Create builder (one-time setup)
docker buildx create --name stratum-builder --use

# Build and push multi-arch image
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -f docker/Dockerfile.core \
  -t ghcr.io/horizon-analytic/stratum:latest \
  --push .
```

## Usage Examples

### Running Scripts

```bash
# Mount current directory and run a script
docker run --rm -v $(pwd):/app stratum run main.strat

# Run with arguments
docker run --rm -v $(pwd):/app stratum run main.strat -- arg1 arg2

# Run in interpreted mode
docker run --rm -v $(pwd):/app stratum run --interpret-all main.strat
```

### Interactive REPL

```bash
# Start REPL
docker run --rm -it stratum repl

# REPL with history persistence
docker run --rm -it \
  -v stratum-history:/home/stratum/.stratum \
  stratum repl
```

### Building Projects

```bash
# Initialize a new project
docker run --rm -v $(pwd):/app stratum init myproject

# Build a binary
docker run --rm -v $(pwd):/app stratum build main.strat -o app

# Run tests
docker run --rm -v $(pwd):/app stratum test
```

### Using LSP (Full Image)

```bash
# Start LSP server (for editor integration)
docker run --rm -it -p 9257:9257 stratum:full lsp --stdio
```

### As a Base Image

```dockerfile
# Use Stratum as base image for your project
FROM ghcr.io/horizon-analytic/stratum:data

# Copy your Stratum project
COPY . /app

# Set working directory
WORKDIR /app

# Run your application
CMD ["stratum", "run", "main.strat"]
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `STRATUM_HOME` | Stratum configuration directory | `~/.stratum` |
| `NO_COLOR` | Disable colored output | unset |

## Volume Mounts

| Container Path | Purpose |
|----------------|---------|
| `/app` | Working directory for code |
| `/home/stratum/.stratum` | Configuration and cache |

## Security

- Images run as non-root user (`stratum`, UID 1000)
- Based on minimal base images (Alpine/Debian slim)
- No unnecessary packages or tools installed

## Version Tags

Images are tagged with semantic versions:

- `stratum:1.0.0` - Specific version
- `stratum:1.0` - Latest patch of 1.0.x
- `stratum:1` - Latest minor of 1.x.x
- `stratum:latest` - Latest stable release
