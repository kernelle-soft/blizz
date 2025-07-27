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
CARGO_VERSION=$(awk '/^\[workspace\.package\]/{flag=1; next} /^\[/{flag=0} flag && /^version = /{gsub(/["[:space:]]/, "", $3); print $3}' Cargo.toml)

echo "📦 Cargo.toml version: $CARGO_VERSION"
echo "🏷️  Git tag version: $TAG_VERSION"

if [ "$CARGO_VERSION" != "$TAG_VERSION" ]; then
  echo "❌ Version mismatch detected!"
  echo "   Cargo.toml has version: $CARGO_VERSION"
  echo "   Git tag has version: $TAG_VERSION"
  echo ""
  echo "💡 To fix this:"
  echo "   1. Update Cargo.toml version to match tag, OR"
  echo "   2. Create a new tag with the correct version"
  exit 1
fi

echo "✅ Version verified: $CARGO_VERSION matches tag" 
