#!/usr/bin/env bash
#
# Deploy docs to GitHub Pages
# Creates/updates the gh-pages branch with built documentation
#
# Usage:
#   ./scripts/deploy-docs.sh
#
# Prerequisites:
#   - mdbook installed (cargo install mdbook)
#   - Clean working directory (no uncommitted changes)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DOCS_DIR="$PROJECT_ROOT/docs"
BOOK_DIR="$DOCS_DIR/book"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }

# Ensure we're in the project root
cd "$PROJECT_ROOT"

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    echo "Error: You have uncommitted changes. Please commit or stash them first."
    exit 1
fi

# Build the docs
log_info "Building documentation with mdbook..."
cd "$DOCS_DIR"
mdbook build

# Get current branch to return to later
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

# Create a temporary directory for the docs
TEMP_DIR=$(mktemp -d)
cp -r "$BOOK_DIR"/* "$TEMP_DIR/"

# Switch to gh-pages branch (create if doesn't exist)
log_info "Switching to gh-pages branch..."
cd "$PROJECT_ROOT"

if git show-ref --verify --quiet refs/heads/gh-pages; then
    git checkout gh-pages
else
    git checkout --orphan gh-pages
    git rm -rf . 2>/dev/null || true
fi

# Remove everything and copy in the new docs
log_info "Updating documentation..."
git rm -rf . 2>/dev/null || true
cp -r "$TEMP_DIR"/* .

# Add .nojekyll to prevent Jekyll processing
touch .nojekyll

# Commit and push
git add -A
git commit -m "Deploy docs $(date +%Y-%m-%d)" || echo "No changes to commit"

log_info "Pushing to origin/gh-pages..."
git push origin gh-pages --force

# Return to original branch
log_info "Returning to $CURRENT_BRANCH..."
git checkout "$CURRENT_BRANCH"

# Cleanup
rm -rf "$TEMP_DIR"

log_success "Documentation deployed to gh-pages branch!"
log_info "Configure GitHub Pages at: https://github.com/horizonanalytic/stratum/settings/pages"
log_info "Set source to 'Deploy from a branch' -> 'gh-pages' -> '/ (root)'"
