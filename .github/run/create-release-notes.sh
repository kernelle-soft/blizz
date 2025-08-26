#!/usr/bin/env bash
set -euo pipefail

# Generate release notes for Blizz releases
# Usage: generate-release-notes.sh <tag> <version>

if [ $# -ne 2 ]; then
  echo "Usage: $0 <tag> <version>"
  echo "Example: $0 v0.1.1 0.1.1"
  exit 1
fi

TAG="$(echo "$1" | xargs)"
VERSION="$2"
OUTPUT_FILE="release_notes.md"

echo "🔍 Generating release notes for $TAG (version $VERSION)"

# Get previous tag for changelog
# We want the tag before the current one, not HEAD~1

PREV_TAG=$(git tag --sort=-version:refname | grep -v "^$TAG$" | head -1 2>/dev/null || echo "")
PREV_TAG="$(echo "$PREV_TAG" | xargs)"

if [ ! -z "$PREV_TAG" ]; then
  echo "📝 Found previous tag: $PREV_TAG"
else
  echo "🆕 No previous tag found - this appears to be the first release"
fi

# Start writing release notes
cat > "$OUTPUT_FILE" << EOF
# Blizz $VERSION

EOF

# Add changelog section
if [ ! -z "$PREV_TAG" ]; then
  echo "## What's Changed" >> "$OUTPUT_FILE"
  echo "" >> "$OUTPUT_FILE"
  
  # Get commits since last tag, format as bullet points
  if git log "${PREV_TAG}..HEAD" --oneline --no-merges --quiet 2>/dev/null; then
    git log "${PREV_TAG}..HEAD" --oneline --no-merges --pretty=format:"* %s" >> "$OUTPUT_FILE"
  else
    echo "* Initial version bump to $VERSION" >> "$OUTPUT_FILE"
  fi
  
  echo "" >> "$OUTPUT_FILE"
  echo "" >> "$OUTPUT_FILE"
else
  # First release
  echo "## What's New" >> "$OUTPUT_FILE"
  echo "" >> "$OUTPUT_FILE"  
  echo "Initial release of the Blizz toolshed." >> "$OUTPUT_FILE"
  echo "" >> "$OUTPUT_FILE"
fi

# Add static installation and tools information
cat >> "$OUTPUT_FILE" << EOF
## Installation

\`\`\`bash
# Download and extract source
curl -L https://github.com/kernelle-soft/blizz/archive/$TAG.tar.gz | tar xz
cd kernelle-$TAG-source

# Install using included script
./scripts/install.sh
\`\`\`

All tools are unified at version $VERSION.
EOF

DOT="%2E"

# Add changelog link if we have a previous tag
if [ ! -z "$PREV_TAG" ]; then
  echo "" >> "$OUTPUT_FILE"
  echo "**Full Changelog**: https://github.com/kernelle-soft/blizz/compare/${PREV_TAG}${DOT}${DOT}${DOT}${TAG}" >> "$OUTPUT_FILE"
fi

echo "✅ Release notes generated: $OUTPUT_FILE"
echo ""
echo "Preview:"
echo "=========================================="
cat "$OUTPUT_FILE"
echo "==========================================" 
