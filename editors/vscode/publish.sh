#!/bin/bash
# Publish the Neve VS Code extension to the marketplace.
# Requires: vsce (npm install -g @vscode/vsce), personal access token

set -euo pipefail

cd "$(dirname "$0")"

echo "=== Building VS Code extension ==="
npm ci
npm run compile

echo "=== Packaging ==="
npx vsce package

echo "=== Publishing ==="
npx vsce publish

echo "=== Done ==="
echo "Extension published to https://marketplace.visualstudio.com/items?itemName=neve-lang.neve"
