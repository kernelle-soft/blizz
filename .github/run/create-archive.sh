#!/usr/bin/env bash
set -euo pipefail

# Create source archives for kernelle releases
# Usage: create-source-archive.sh <tag>

if [ $# -ne 1 ]; then
  echo "Usage: $0 <tag>"
  echo "Example: $0 v0.1.1"
  exit 1
fi

TAG="$1"
ARCHIVE_NAME="kernelle-${TAG}-source"

echo "📦 Creating source archives for $TAG"

# Create clean source archives using git archive
echo "🗜️  Creating tar.gz archive..."
git archive --format=tar.gz --prefix="${ARCHIVE_NAME}/" HEAD > "${ARCHIVE_NAME}.tar.gz"

echo "🗜️  Creating zip archive..."  
git archive --format=zip --prefix="${ARCHIVE_NAME}/" HEAD > "${ARCHIVE_NAME}.zip"

# Verify archives were created and show sizes
echo "✅ Archives created successfully:"
ls -lh "${ARCHIVE_NAME}".{tar.gz,zip}

# Show what's included in the archives (first few entries)
echo ""
echo "📋 Archive contents preview:"
echo "tar.gz contents:"
tar -tzf "${ARCHIVE_NAME}.tar.gz" | head -10
echo "..."

echo ""
echo "🎉 Source archives ready for release!" 
