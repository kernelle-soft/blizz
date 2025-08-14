#!/usr/bin/env bash
set -euo pipefail

# Conditionally bump version in workspace Cargo.toml
# Usage: bump-version.sh [patch|minor|major]
# If no argument provided, defaults to patch version bump
# Only bumps if current Cargo version is not ahead of latest release

BUMP_TYPE="${1:-patch}"

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    patch|minor|major)
      BUMP_TYPE="$1"
      shift
      ;;
    *)
      shift
      ;;
  esac
done

echo "🔧 Preparing to bump $BUMP_TYPE version in workspace Cargo.toml"
echo "🔍 Checking if version bump is needed..."

# Fetch latest tags
git fetch --tags --force >/dev/null 2>&1 || true

# Get current Cargo version
CURRENT_VERSION=$(grep -A 10 '^\[workspace\.package\]' Cargo.toml | grep '^version = ' | sed 's/version = "//' | sed 's/"//' | tr -d ' \t\r\n')

if [ -z "$CURRENT_VERSION" ]; then
  echo "❌ Could not find version in workspace Cargo.toml"
  exit 1
fi

# Find latest semantic version tag
LATEST_TAG=$(git tag -l 'v[0-9]*.[0-9]*.[0-9]*' --sort=-v:refname | head -n1 || true)

if [ -n "$LATEST_TAG" ]; then
  LATEST_VERSION=${LATEST_TAG#v}
  echo "📦 Current Cargo version: $CURRENT_VERSION"
  echo "🏷️  Latest released version: $LATEST_VERSION"

  # Compare versions semantically using sort -V (-V is a semver comparator)
  # If CURRENT_VERSION is greater than LATEST_VERSION, it will appear last when sorted
  HIGHEST=$(printf "%s\n%s\n" "$CURRENT_VERSION" "$LATEST_VERSION" | sort -V | tail -n1)
  
  if [ "$CURRENT_VERSION" = "$HIGHEST" ] && [ "$CURRENT_VERSION" != "$LATEST_VERSION" ]; then
    echo "✅ Cargo version ($CURRENT_VERSION) is ahead of latest release ($LATEST_VERSION). No bump needed."
    exit 0
  else
    echo "🟡 Cargo version is not ahead of latest release. Will bump from latest version."
    # Update CURRENT_VERSION to be the latest released version for proper bumping
    CURRENT_VERSION="$LATEST_VERSION"
  fi
else
  echo "ℹ️  No existing releases found. No bump needed for first release."
  exit 0
fi

# Extract current version from workspace Cargo.toml
CURRENT_VERSION=$(grep -A 10 '^\[workspace\.package\]' Cargo.toml | grep '^version = ' | sed 's/version = "//' | sed 's/"//' | tr -d ' \t\r\n')

if [ -z "$CURRENT_VERSION" ]; then
  echo "❌ Could not find version in workspace Cargo.toml"
  exit 1
fi

echo "📦 Current version: $CURRENT_VERSION"

# Parse semantic version components
if [[ ! "$CURRENT_VERSION" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
  echo "❌ Version format is not semantic (major.minor.patch): $CURRENT_VERSION"
  exit 1
fi

MAJOR="${BASH_REMATCH[1]}"
MINOR="${BASH_REMATCH[2]}"
PATCH="${BASH_REMATCH[3]}"

# Calculate new version based on bump type
case "$BUMP_TYPE" in
  "major")
    NEW_MAJOR=$((MAJOR + 1))
    NEW_MINOR=0
    NEW_PATCH=0
    ;;
  "minor")
    NEW_MAJOR=$MAJOR
    NEW_MINOR=$((MINOR + 1))
    NEW_PATCH=0
    ;;
  "patch")
    NEW_MAJOR=$MAJOR
    NEW_MINOR=$MINOR
    NEW_PATCH=$((PATCH + 1))
    ;;
  *)
    echo "❌ Invalid bump type: $BUMP_TYPE. Use patch, minor, or major"
    exit 1
    ;;
esac

NEW_VERSION="$NEW_MAJOR.$NEW_MINOR.$NEW_PATCH"

echo "🚀 New version: $NEW_VERSION"

# Update version in Cargo.toml
# Use sed to replace the version line in the [workspace.package] section
sed -i "/^\[workspace\.package\]/,/^\[/ s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml

# Verify the change was made
UPDATED_VERSION=$(grep -A 10 '^\[workspace\.package\]' Cargo.toml | grep '^version = ' | sed 's/version = "//' | sed 's/"//' | tr -d ' \t\r\n')

if [ "$UPDATED_VERSION" != "$NEW_VERSION" ]; then
  echo "❌ Failed to update version in Cargo.toml"
  echo "   Expected: $NEW_VERSION"
  echo "   Found: $UPDATED_VERSION"
  exit 1
fi

echo "✅ Successfully bumped version from $CURRENT_VERSION to $NEW_VERSION"

# Stage the change
# Update Cargo.lock so the version bump is reflected
echo "Updating Cargo.lock..."
cargo check --workspace

# Commit the version bump
echo "Committing version bump to git..."

# Extract author info from Cargo.toml
AUTHOR_LINE=$(grep -A 10 '^\[workspace\.package\]' Cargo.toml | grep '^authors = ' | sed 's/authors = \["//' | sed 's/"\]//')
AUTHOR_NAME=$(echo "$AUTHOR_LINE" | sed 's/ <.*$//')
AUTHOR_EMAIL=$(echo "$AUTHOR_LINE" | sed 's/.*<\(.*\)>.*/\1/')

git config --local user.email "$AUTHOR_EMAIL"
git config --local user.name "$AUTHOR_NAME"
git add .
git commit -m "v$NEW_VERSION" -m "[validated]"
