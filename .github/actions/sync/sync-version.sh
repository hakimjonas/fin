#!/usr/bin/env bash
set -e

echo "🔄 Syncing Version and Updating Build Artifacts..."

# 1. Use the provided TAG environment variable if available.
if [[ -n "$TAG" ]]; then
  TAG_VERSION="$TAG"
# 2. Otherwise, if a .git directory is available, try to get the latest tag.
elif [ -d ".git" ]; then
  # Suppress errors if no tag is found.
  TAG_VERSION=$(git describe --tags --abbrev=0 2>/dev/null | sed 's/^v//')
fi

# 3. If TAG_VERSION is still empty, use a default version.
if [[ -z "$TAG_VERSION" ]]; then
  TAG_VERSION="0.2.0"
  echo "No tag provided or found; falling back to default version: $TAG_VERSION"
fi

# 4. Validate that the version is a valid semantic version (e.g. 0.2.0).
if ! [[ "$TAG_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "❌ Invalid version format: '$TAG_VERSION'. Expected a semantic version (e.g., 0.2.0)."
  exit 1
fi

echo "📌 Using version: $TAG_VERSION"

echo "📦 Updating Cargo.toml..."
cargo install cargo-edit --debug || true  # Install cargo-edit if missing
cargo set-version "$TAG_VERSION"

# Step 3: Update package filenames in INSTALL.md
INSTALL_FILE="INSTALL.md"
echo "📝 Updating package filenames in $INSTALL_FILE..."

sed -i -E "s/fin-[0-9]+\.[0-9]+\.[0-9]+-arch\.tar\.gz/fin-${TAG_VERSION}-arch.tar.gz/g" "$INSTALL_FILE"
sed -i -E "s/fin-[0-9]+\.[0-9]+\.[0-9]+-nix\.tar\.gz/fin-${TAG_VERSION}-nix.tar.gz/g" "$INSTALL_FILE"
sed -i -E "s/fin-[0-9]+\.[0-9]+\.[0-9]+-solus\.tar\.gz/fin-${TAG_VERSION}-solus.tar.gz/g" "$INSTALL_FILE"
sed -i -E "s/fin_[0-9]+\.[0-9]+\.[0-9]+_amd64\.deb/fin_${TAG_VERSION}_amd64.deb/g" "$INSTALL_FILE"

echo "✅ INSTALL.md updated with latest release version: ${TAG_VERSION}"

# Step 4: Build the project (ensuring the correct version is embedded)
echo "🔨 Building project..."
cargo build --release

# Step 5: Prepare the packaging directories
mkdir -p target/package/{solus,arch,nix}

echo "📦 Preparing packaging for version ${TAG_VERSION}..."

# Copy binaries and assets into package directories
cp target/release/fin target/package/solus/
cp target/release/fin target/package/arch/
cp target/release/fin target/package/nix/

cp -r assets target/package/solus/
cp -r assets target/package/arch/
cp -r assets target/package/nix/

# Step 6: Package the artifacts with the correct version name
echo "📦 Creating tarballs..."
tar -czvf "target/fin-${TAG_VERSION}-solus.tar.gz" -C target/package/solus .
tar -czvf "target/fin-${TAG_VERSION}-arch.tar.gz" -C target/package/arch .
tar -czvf "target/fin-${TAG_VERSION}-nix.tar.gz" -C target/package/nix .

echo "🎉 Packaging complete! Artifacts created in target/:"
ls -lh target/fin-"${TAG_VERSION}"-*

echo "✅ All steps completed successfully!"
