#!/bin/bash
set -euo pipefail

echo "=== Starting Version Bump Process ==="

# Use CIRCLE_BRANCH for branch identification.
if [ "${CIRCLE_BRANCH:-}" != "trunk" ]; then
  echo "Current branch (${CIRCLE_BRANCH:-}) is not trunk. Exiting version bump."
  exit 0
fi

# Verify required environment variables.
for var in GH_TOKEN CIRCLE_SHA1; do
  if [ -z "${!var:-}" ]; then
    echo "❌ $var is not set!"
    exit 1
  fi
done
echo "✅ Required environment variables are set."

# Determine current version from Cargo.toml
current_version=$(awk -F'"' '/^version *=/ {print $2; exit}' Cargo.toml)
echo "Current version: $current_version"

# Bump patch version.
if [[ "$current_version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-.*)?$ ]]; then
  major="${BASH_REMATCH[1]}"
  minor="${BASH_REMATCH[2]}"
  patch="${BASH_REMATCH[3]}"
  new_patch=$((patch + 1))
  new_version="${major}.${minor}.${new_patch}"
  echo "Bumping version: $current_version -> $new_version"
else
  echo "❌ Invalid version format: $current_version"
  exit 1
fi

# Update version in files.
sed -i "s/^version *= *\"[^\"]*\"/version = \"$new_version\"/" Cargo.toml
sed -i "s/^pkgver=.*/pkgver=$new_version/" PKGBUILD
sed -i "s|<Version>[^<]*</Version>|<Version>$new_version</Version>|" fin.sol
sed -i "s/^[[:space:]]*version *= *\"[^\"]*\";/  version = \"$new_version\";/" flake.nix
sed -i "s/$current_version/$new_version/g" INSTALL.md

# Update CHANGELOG.md.
if grep -q "^## \[Unreleased\]" CHANGELOG.md; then
  sed -i "s/^## \[Unreleased\]/## [$new_version] - $(date +%Y-%m-%d)/" CHANGELOG.md
else
  echo -e "## [$new_version] - $(date +%Y-%m-%d)\n" | cat - CHANGELOG.md > CHANGELOG.tmp && mv CHANGELOG.tmp CHANGELOG.md
fi

# Create a new branch for the bump.
bump_branch="version-bump-$new_version"
git checkout -b "$bump_branch"
git add Cargo.toml PKGBUILD fin.sol flake.nix INSTALL.md CHANGELOG.md

if git diff --cached --quiet; then
  echo "No changes detected. Exiting."
  exit 0
fi

git commit -m "Bump version to $new_version"
git push origin "$bump_branch"

# Create a PR using GitHub CLI and auto-merge it.
pr_url=$(gh pr create --fill --base trunk --head "$bump_branch" --title "Version bump to $new_version" --body "Automatic version bump")
echo "Created bump PR: $pr_url"

# Auto-merge the bump PR.
gh pr merge "$pr_url" --squash --delete-branch --auto
echo "=== Version bump process complete. New version: $new_version ==="
