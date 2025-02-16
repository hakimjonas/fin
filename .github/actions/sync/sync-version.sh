#!/usr/bin/env bash
set -e

# Allow git discovery across filesystem boundaries.
export GIT_DISCOVERY_ACROSS_FILESYSTEM=1

# Ensure we're in the repository root.
if [ -n "$GITHUB_WORKSPACE" ]; then
  cd "$GITHUB_WORKSPACE"
elif git rev-parse --show-toplevel >/dev/null 2>&1; then
  cd "$(git rev-parse --show-toplevel)"
fi

echo "🔄 Syncing Version and Updating Build Artifacts..."

# 1. Use the provided TAG environment variable if available.
if [[ -n "$TAG" ]]; then
  # Allow version to be provided with or without a leading 'v'.
  TAG_VERSION="${TAG#v}"
else
  # 2. Otherwise, if a .git directory is available, try to get the latest tag.
  if [ -d ".git" ]; then
    TAG_VERSION=$(git describe --tags --abbrev=0 2>/dev/null | sed 's/^v//')
  fi
fi

# 3. If TAG_VERSION is still empty, try to fall back to Cargo.toml.
if [[ -z "$TAG_VERSION" ]]; then
  echo "ℹ️ No valid git tag found. Falling back to Cargo.toml version."
  TAG_VERSION=$(grep '^version = ' Cargo.toml | head -n1 | sed 's/version = "\(.*\)"/\1/')
fi

# 4. If TAG_VERSION is still empty, then error out.
if [[ -z "$TAG_VERSION" ]]; then
  echo "❌ No version provided via TAG, no valid git tag found, and Cargo.toml version is empty."
  exit 1
fi

# 5. Validate that the version is a valid semantic version.
if ! [[ "$TAG_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "❌ Invalid version format: '$TAG_VERSION'. Expected a semantic version (e.g., 0.2.0)."
  exit 1
fi

echo "📌 Current version: $TAG_VERSION"

# 6. If no TAG was provided via the environment, automatically increment the patch version.
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
cargo install cargo-edit --debug || true
cargo set-version "$TAG_VERSION"

echo "📝 Updating package filenames in INSTALL.md..."
INSTALL_FILE="INSTALL.md"
sed -i -E "s/fin-[0-9]+\.[0-9]+\.[0-9]+-arch\.tar\.gz/fin-${TAG_VERSION}-arch.tar.gz/g" "$INSTALL_FILE"
sed -i -E "s/fin-[0-9]+\.[0-9]+\.[0-9]+-nix\.tar\.gz/fin-${TAG_VERSION}-nix.tar.gz/g" "$INSTALL_FILE"
sed -i -E "s/fin-[0-9]+\.[0-9]+\.[0-9]+-solus\.tar\.gz/fin-${TAG_VERSION}-solus.tar.gz/g" "$INSTALL_FILE"
sed -i -E "s/fin_[0-9]+\.[0-9]+\.[0-9]+_amd64\.deb/fin_${TAG_VERSION}_amd64.deb/g" "$INSTALL_FILE"

echo "✅ INSTALL.md updated with latest release version: ${TAG_VERSION}"

# 7. Automatically update the CHANGELOG.md by overwriting it with a new entry for this release.
CHANGELOG_FILE="CHANGELOG.md"
NEW_ENTRY="## [$TAG_VERSION] - $(date +%Y-%m-%d)\n\n"
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

echo "🔨 Building project..."
cargo build --release

echo "📂 Listing target/release contents:"
ls -lh target/release

# 8. Determine the binary to package.
BIN_PATH="target/release/fin"
if [ ! -f "$BIN_PATH" ]; then
  echo "❌ 'fin' binary not found in target/release. Searching for an executable..."
  BIN_PATH=$(find target/release -maxdepth 1 -type f -executable | head -n 1)
  if [ -z "$BIN_PATH" ]; then
    echo "❌ No executable found in target/release."
    ls -lh target/release
    exit 1
  else
    echo "✅ Found executable: $BIN_PATH"
  fi
fi

echo "📦 Preparing packaging for version ${TAG_VERSION}..."
mkdir -p target/package/{solus,arch,nix}

# Copy binary and assets into package directories.
cp "$BIN_PATH" target/package/solus/
cp "$BIN_PATH" target/package/arch/
cp "$BIN_PATH" target/package/nix/
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
