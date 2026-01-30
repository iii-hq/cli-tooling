#!/bin/bash
# Build template zip files using the CLI
# This ensures identical zip generation between local dev and CI

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$REPO_ROOT"

# Build the CLI if needed, then run with build-zips
if [ -f "target/release/motia-cli" ]; then
    ./target/release/motia-cli build-zips --template-dir=./templates 
elif [ -f "target/debug/motia-cli" ]; then
    ./target/debug/motia-cli build-zips --template-dir=./templates 
else
    # Build and run via cargo
    cargo run --quiet -- build-zips --template-dir=./templates 
fi

# Stage the zip files for git
cd templates
for zip in *.zip; do
    [ -f "$zip" ] && git add "$zip" 2>/dev/null || true
done

echo "Template zips staged for commit"
