#!/usr/bin/env bash
set -euo pipefail

# Verify that git tag version matches Cargo.toml version
# Usage: verify-version.sh <tag_version>

if [ $# -ne 1 ]; then
  echo "Usage: $0 <tag_version>"
  echo "Example: $0 0.1.1"
  exit 1
fi

TAG_VERSION="$1"

echo "🔍 Verifying version consistency"

# Extract version from workspace Cargo.toml
# Look for version in [workspace.package] section
CARGO_VERSION=$(grep -A 10 '^\[workspace\.package\]' Cargo.toml | grep '^version = ' | sed 's/version = "//' | sed 's/"//' | tr -d ' \t\r\n')

echo "📦 Cargo.toml version: $CARGO_VERSION"
echo "🏷️  Git tag version: $TAG_VERSION"

# Trim any potential whitespace and normalize
CARGO_VERSION_CLEAN=$(printf '%s' "$CARGO_VERSION" | tr -d ' \t\r\n')
TAG_VERSION_CLEAN=$(printf '%s' "$TAG_VERSION" | tr -d ' \t\r\n')

if [ "$CARGO_VERSION_CLEAN" != "$TAG_VERSION_CLEAN" ]; then
  echo "❌ Version mismatch detected!"
  echo "   Cargo.toml has version: $CARGO_VERSION_CLEAN"
  echo "   Git tag has version: $TAG_VERSION_CLEAN"
  echo ""
  echo "💡 To fix this:"
  echo "   1. Update Cargo.toml version to match tag, OR"
  echo "   2. Create a new tag with the correct version"
  exit 1
fi

echo "✅ Version verified: $CARGO_VERSION_CLEAN matches tag" 
