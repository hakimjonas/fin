#!/usr/bin/env bash
set -e

echo "🔄 Syncing Version and Updating Build Artifacts..."

# 1. Use the provided TAG environment variable if available.
if [[ -n "$TAG" ]]; then
  TAG_VERSION="$TAG"
# 2. Otherwise, if a .git directory is available, try to get the latest tag.
elif [ -d ".git" ]; then
  TAG_VERSION=$(git describe --tags --abbrev=0 2>/dev/null | sed 's/^v//')
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

echo "📌 Using version: $TAG_VERSION"

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

echo "🔨 Building project..."
cargo build --release

echo "📦 Preparing packaging for version ${TAG_VERSION}..."
mkdir -p target/package/{solus,arch,nix}

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
