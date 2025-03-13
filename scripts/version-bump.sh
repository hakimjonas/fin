#!/bin/bash
set -euo pipefail

echo "=== Starting Version Bump Process ==="

# Only bump on trunk
if [ "${CIRCLE_BRANCH:-}" != "trunk" ]; then
  echo "Not on trunk. Exiting."
  exit 0
fi

# Verify env vars
for var in GH_TOKEN CIRCLE_SHA1; do
  if [ -z "${!var:-}" ]; then
    echo "❌ $var not set!"
    exit 1
  fi
done
echo "✅ Environment vars OK."

current_version=$(awk -F'"' '/^version *=/ {print $2; exit}' Cargo.toml)
echo "Current version: $current_version"

if [[ "$current_version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-.*)?$ ]]; then
  major="${BASH_REMATCH[1]}"
  minor="${BASH_REMATCH[2]}"
  patch="${BASH_REMATCH[3]}"
  new_patch=$((patch + 1))
  new_version="${major}.${minor}.${new_patch}"
else
  echo "Invalid version format: $current_version"
  exit 1
fi
echo "New version: $new_version"

sed -i "s/^version *= *\"[^\"]*\"/version = \"$new_version\"/" Cargo.toml
sed -i "s/^pkgver=.*/pkgver=$new_version/" PKGBUILD
sed -i "s|<Version>[^<]*</Version>|<Version>$new_version</Version>|" fin.sol
sed -i "s/^[[:space:]]*version *= *\"[^\"]*\";/  version = \"$new_version\";/" flake.nix
sed -i "s/$current_version/$new_version/g" INSTALL.md

if grep -q "^## \[Unreleased\]" CHANGELOG.md; then
  sed -i "s/^## \[Unreleased\]/## [$new_version] - $(date +%Y-%m-%d)/" CHANGELOG.md
else
  echo -e "## [$new_version] - $(date +%Y-%m-%d)\n" | cat - CHANGELOG.md > CHANGELOG.tmp && mv CHANGELOG.tmp CHANGELOG.md
fi

git config user.email "ci-bot@example.com"
git config user.name "CI Bot"
bump_branch="version-bump-$new_version"
git checkout -b "$bump_branch"
git add Cargo.toml PKGBUILD fin.sol flake.nix INSTALL.md CHANGELOG.md

git commit -m "Bump version to $new_version [skip ci]"
git push origin "$bump_branch"

pr_url=$(gh pr create --fill --base trunk --head "$bump_branch" --title "Version bump to $new_version" --body "Automatic bump")
echo "Bump PR created: $pr_url"

gh pr merge "$pr_url" --squash --delete-branch --auto
