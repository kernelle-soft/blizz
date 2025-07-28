#!/usr/bin/env bash
set -euo pipefail

# Create and push a git tag with the current workspace version
# Usage: tag-version.sh

echo "🏷️  Creating git tag from workspace version"

# Extract current version from workspace Cargo.toml
CURRENT_VERSION=$(grep -A 10 '^\[workspace\.package\]' Cargo.toml | grep '^version = ' | sed 's/version = "//' | sed 's/"//' | tr -d ' \t\r\n')

if [ -z "$CURRENT_VERSION" ]; then
  echo "❌ Could not find version in workspace Cargo.toml"
  exit 1
fi

echo "📦 Current version: $CURRENT_VERSION"

# Create tag name with 'v' prefix
TAG_NAME="v$CURRENT_VERSION"

echo "🔖 Creating tag: $TAG_NAME"

# Check if tag already exists
if git tag -l | grep -q "^$TAG_NAME$"; then
  echo "⚠️  Tag $TAG_NAME already exists"
  echo "🔍 Checking if it points to current commit..."
  
  CURRENT_COMMIT=$(git rev-parse HEAD)
  TAG_COMMIT=$(git rev-list -n 1 "$TAG_NAME" 2>/dev/null || echo "")
  
  if [ "$CURRENT_COMMIT" = "$TAG_COMMIT" ]; then
    echo "✅ Tag $TAG_NAME already points to current commit"
    exit 0
  else
    echo "❌ Tag $TAG_NAME exists but points to different commit"
    echo "   Current commit: $CURRENT_COMMIT"
    echo "   Tag commit: $TAG_COMMIT"
    echo ""
    echo "💡 To fix this:"
    echo "   1. Delete the existing tag: git tag -d $TAG_NAME && git push origin :$TAG_NAME"
    echo "   2. Or bump the version to create a new tag"
    exit 1
  fi
fi

# Create annotated tag with release message
git tag -a "$TAG_NAME" -m "Release $CURRENT_VERSION

This release includes the latest changes from the development branch.
Generated automatically by the CI/CD pipeline."

echo "✅ Created tag $TAG_NAME"

# Push the tag to remote
git push origin "$TAG_NAME"

echo "🚀 Pushed tag $TAG_NAME to remote"
echo "📋 Tag summary:"
echo "   Name: $TAG_NAME"
echo "   Version: $CURRENT_VERSION" 
echo "   Commit: $(git rev-parse HEAD)"
