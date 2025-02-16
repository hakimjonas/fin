#!/usr/bin/env bash
set -e

echo "🔄 Syncing Version and Updating Build Artifacts..."

# 1. Use the provided TAG environment variable if available.
if [[ -n "$TAG" ]]; then
  # Strip a leading 'v' if it exists.
  TAG_VERSION="${TAG#v}"
else
  # 2. Otherwise, if a .git directory is available, try to get the latest tag.
  if [ -d ".git" ]; then
    TAG_VERSION=$(git describe --tags --abbrev=0 2>/dev/null | sed 's/^v//')
  fi
fi

# 3. If TAG_VERSION is still empty, then error out.
if [[ -z "$TAG_VERSION" ]]; then
  echo "❌ No version provided via TAG and no valid git tag found."
  echo "Please supply a valid semantic version (e.g., 0.2.0) either via the workflow input or by ensuring the repository has a valid git tag."
  exit 1
fi

# 4. Validate that the version is a valid semantic version (e.g., 0.2.0).
if ! [[ "$TAG_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "❌ Invalid version format: '$TAG_VERSION'. Expected a semantic version (e.g., 0.2.0)."
  exit 1
fi

echo "📌 Current version: $TAG_VERSION"

# 5. If no TAG was provided via the environment, automatically increment the patch version.
if [[ -z "$TAG" ]]; then
  IFS='.' read -r major minor patch <<< "$TAG_VERSION"
  new_patch=$((patch + 1))
  NEW_VERSION="${major}.${minor}.${new_patch}"
  echo "🔼 Bumping patch version: $TAG_VERSION -> $NEW_VERSION"
  TAG_VERSION="$NEW_VERSION"
else
  echo "🔼 Using provided version without bump: $TAG_VERSION"
fi

echo "📦 Updating Cargo.toml..."
cargo install cargo-edit --debug || true  # Install cargo-edit if missing
cargo set-version "$TAG_VERSION"

echo "📝 Updating package filenames in INSTALL.md..."
INSTALL_FILE="INSTALL.md"
sed -i -E "s/fin-[0-9]+\.[0-9]+\.[0-9]+-arch\.tar\.gz/fin-${TAG_VERSION}-arch.tar.gz/g" "$INSTALL_FILE"
sed -i -E "s/fin-[0-9]+\.[0-9]+\.[0-9]+-nix\.tar\.gz/fin-${TAG_VERSION}-nix.tar.gz/g" "$INSTALL_FILE"
sed -i -E "s/fin-[0-9]+\.[0-9]+\.[0-9]+-solus\.tar\.gz/fin-${TAG_VERSION}-solus.tar.gz/g" "$INSTALL_FILE"
sed -i -E "s/fin_[0-9]+\.[0-9]+\.[0-9]+_amd64\.deb/fin_${TAG_VERSION}_amd64.deb/g" "$INSTALL_FILE"

echo "✅ INSTALL.md updated with latest release version: ${TAG_VERSION}"

# 6. Automatically update the CHANGELOG.md by overwriting it with a new entry for this release.
CHANGELOG_FILE="CHANGELOG.md"
NEW_ENTRY="## [$TAG_VERSION] - $(date +%Y-%m-%d)\n\n"

# Determine the most recent published tag (normalize by stripping any leading 'v')
LAST_TAG=$(git tag --sort=-v:refname | sed 's/^v//' | grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' | grep -v "^${TAG_VERSION}$" | head -n 1)

if [[ -n "$LAST_TAG" ]]; then
  NEW_ENTRY+="### Changes since $LAST_TAG:\n"
  SUMMARY=$(git log "v${LAST_TAG}"..HEAD --merges --pretty=format:"- %s")
  if [[ -n "$SUMMARY" ]]; then
    NEW_ENTRY+="$SUMMARY\n"
  else
    NEW_ENTRY+="No merged PRs found.\n"
  fi
else
  NEW_ENTRY+="No previous release found. (This is the first release.)\n"
fi

echo -e "$NEW_ENTRY" > "$CHANGELOG_FILE"
echo "✅ CHANGELOG.md overwritten with new entry for version $TAG_VERSION."


# Copy binaries and assets into package directories
cp target/release/fin target/package/solus/
cp target/release/fin target/package/arch/
cp target/release/fin target/package/nix/
cp -r assets target/package/solus/
cp -r assets target/package/arch/
cp -r assets target/package/nix/

echo "📦 Creating tarballs..."
tar -czvf "target/fin-${TAG_VERSION}-solus.tar.gz" -C target/package/solus .
tar -czvf "target/fin-${TAG_VERSION}-arch.tar.gz" -C target/package/arch .
tar -czvf "target/fin-${TAG_VERSION}-nix.tar.gz" -C target/package/nix .

echo "🎉 Packaging complete! Artifacts created in target/:"
ls -lh target/fin-"${TAG_VERSION}"-*

echo "✅ All steps completed successfully!"
