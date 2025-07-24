#!/bin/bash

# Extract the badge line (first line) from the coverage results
NEW_BADGE=$(head -n 1 code-coverage-results.md)

# Replace the coverage badge line in README.md
sed -i "s|^!\[Code Coverage\](https://img\.shields\.io/badge/.*|$NEW_BADGE|" README.md

# Check if there were changes
if git diff --quiet README.md; then
  echo "No changes to README.md"
else
  echo "Coverage badge updated in README.md"
  git config --local user.email "action@github.com"
  git config --local user.name "GitHub Action"
  git add README.md
  git commit -m "Update coverage badge [skip ci]"
  git push
fi