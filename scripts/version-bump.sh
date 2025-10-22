#!/bin/bash
set -euo pipefail

echo "=== Starting Version Bump Process ==="

# Ensure we're on trunk
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

# Prevent infinite loops by checking the last commit message.
if git log -1 --pretty=%B | grep -q "Bump version to"; then
  echo "🚫 Last commit is already a version bump. Exiting to prevent infinite loop."
  exit 0
fi

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

# Set Git identity explicitly (needed for CI)
git config user.email "ci-bot@example.com"
git config user.name "CI Bot"

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

# Create a PR using GitHub CLI.
pr_url=$(gh pr create --fill --base trunk --head "$bump_branch" --title "Version bump to $new_version" --body "Automatic version bump")
echo "Created bump PR: $pr_url"

# Wait for PR to be merged using the "state" field.
echo "Waiting for PR to be merged..."
until [ "$(gh pr view "$bump_branch" --json state --jq '.state')" = "MERGED" ]; do
  echo "🔄 PR not merged yet, retrying in 10 seconds..."
  sleep 10
done
echo "✅ Version bump PR merged successfully."

# Ensure we have the latest trunk
git checkout trunk
git pull origin trunk

# Create an annotated tag for the new version if it doesn't exist.
tag="v$new_version"
if git rev-parse "$tag" >/dev/null 2>&1; then
  echo "Tag $tag already exists. Skipping tag creation."
else
  echo "Creating new tag: $tag"
  git tag -a "$tag" -m "Release $tag"
  git push origin "$tag"
fi

echo "✅ Created and pushed tag $tag"

echo "=== Version bump process complete. New version: $new_version ==="
