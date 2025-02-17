#!/usr/bin/env bash
set -euo pipefail

echo "🔄 Starting Release Automation Process..."

# --------------------------
# Repository Root
# --------------------------
if [ -n "${GITHUB_WORKSPACE:-}" ]; then
  cd "$GITHUB_WORKSPACE"
elif git rev-parse --show-toplevel >/dev/null 2>&1; then
  cd "$(git rev-parse --show-toplevel)"
fi
echo "📂 Current working directory: $(pwd)"

# --------------------------
# Version Determination
# --------------------------
TAG_VERSION=""
# If TAG is not set, try to use GITHUB_REF (e.g., refs/tags/v0.2.5)
if [[ -z "${TAG:-}" && -n "${GITHUB_REF:-}" ]]; then
  TAG="${GITHUB_REF##*/}"
  echo "🔍 Using tag from GITHUB_REF: $TAG"
fi

if [[ -n "${TAG:-}" ]]; then
  # Use provided tag (strip any leading 'v')
  TAG_VERSION="${TAG#v}"
elif [[ -d ".git" ]]; then
  # Otherwise, attempt to get latest Git tag
  TAG_VERSION=$(git describe --tags --abbrev=0 2>/dev/null | sed 's/^v//')
fi

# Fallback: If no tag is found, use Cargo.toml version
if [[ -z "$TAG_VERSION" ]]; then
  echo "ℹ️ No valid git tag found. Falling back to Cargo.toml version."
  TAG_VERSION=$(grep -E '^\s*version\s*=\s*".+"' Cargo.toml | head -n1 | sed -E 's/^\s*version\s*=\s*"([^"]+)".*/\1/')
fi

# Error out if still empty.
if [[ -z "$TAG_VERSION" ]]; then
  echo "❌ No version provided via TAG, no valid git tag found, and Cargo.toml version is empty."
  exit 1
fi

# Validate version format (allow only semantic versions like X.Y.Z)
if ! [[ "$TAG_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "❌ Invalid version format: '$TAG_VERSION'. Expected a semantic version (e.g., 0.2.0)."
  exit 1
fi

echo "📌 Current version: $TAG_VERSION"

# --------------------------
# Version Increment Logic
# --------------------------
if [[ -z "${TAG:-}" ]]; then
  # If no TAG was provided, automatically bump the patch.
  IFS='.' read -r major minor patch <<< "$TAG_VERSION"
  new_patch=$((patch + 1))
  TAG_VERSION="${major}.${minor}.${new_patch}"
  echo "🔼 Bumped version to: $TAG_VERSION"
else
  echo "🔼 Using provided version without bump: $TAG_VERSION"
fi

# --------------------------
# Dependency Management
# --------------------------
if ! command -v cargo-set-version &>/dev/null; then
  echo "📦 Installing cargo-edit..."
  cargo install cargo-edit
fi

if ! command -v cargo-deb &>/dev/null; then
  echo "📦 Installing cargo-deb..."
  cargo install cargo-deb
fi

# --------------------------
# Project Configuration
# --------------------------
echo "📦 Updating Cargo.toml..."
cargo set-version "$TAG_VERSION"

echo "📄 Updating INSTALL.md..."
sed -i -E \
  -e "s/(fin-)[0-9]+\.[0-9]+\.[0-9]+(-arch\.tar\.gz)/\1${TAG_VERSION}\2/g" \
  -e "s/(fin-)[0-9]+\.[0-9]+\.[0-9]+(-nix\.tar\.gz)/\1${TAG_VERSION}\2/g" \
  -e "s/(fin-)[0-9]+\.[0-9]+\.[0-9]+(-solus\.tar\.gz)/\1${TAG_VERSION}\2/g" \
  -e "s/(fin_)[0-9]+\.[0-9]+\.[0-9]+(_amd64\.deb)/\1${TAG_VERSION}\2/g" \
  INSTALL.md

echo "✅ INSTALL.md updated with latest release version: ${TAG_VERSION}"

# --------------------------
# Changelog Generation
# --------------------------
CHANGELOG_FILE="CHANGELOG.md"
RELEASE_DATE=$(date +%Y-%m-%d)
NEW_ENTRY="## [$TAG_VERSION] - $RELEASE_DATE\n\n"

if [ -d ".git" ]; then
  echo "🔍 .git directory found. Generating changelog summary..."
  # Normalize tags (strip 'v') and sort semantically.
  LAST_TAG=$(git tag --sort=-v:refname | sed 's/^v//' | grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' | grep -v "^${TAG_VERSION}$" | head -n1)
  if [[ -n "$LAST_TAG" ]]; then
    NEW_ENTRY+="### Changes since ${LAST_TAG}:\n"
    COMMIT_LOG=$(git log "v${LAST_TAG}"..HEAD --pretty=format:"- %s (%h)" || true)
    NEW_ENTRY+="${COMMIT_LOG:-No significant changes detected}\n"
  else
    NEW_ENTRY+="### Initial Release\n"
  fi
else
  echo "⚠️ .git directory not found. Manual changelog entry required."
  NEW_ENTRY+="Manual changelog entry required.\n"
fi

# Overwrite CHANGELOG.md with the new entry.
echo -e "$NEW_ENTRY" > "$CHANGELOG_FILE"
echo "✅ CHANGELOG.md overwritten with new entry for version $TAG_VERSION."

# --------------------------
# Build Process
# --------------------------
echo "🔨 Building project..."
cargo build --release

echo "📂 Contents of target/release:"
ls -lh target/release

# --------------------------
# Package Preparation
# --------------------------
PKG_DIR="target/package"
echo "📦 Preparing packages in $PKG_DIR..."
rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR"/{solus,arch,nix}

copy_package_files() {
  local target_dir=$1
  cp target/release/fin "$target_dir/"
  cp -r assets "$target_dir/"
}

copy_package_files "$PKG_DIR/solus"
copy_package_files "$PKG_DIR/arch"
copy_package_files "$PKG_DIR/nix"

# --------------------------
# Artifact Generation
# --------------------------
echo "📦 Creating distribution artifacts..."
(
  cd target
  rm -f ./*.tar.gz ./*.deb

  # Create platform tarballs
  for platform in solus arch nix; do
    tar -czf "fin-${TAG_VERSION}-${platform}.tar.gz" -C "package/$platform" .
  done
)

echo "📦 Building Debian package..."
cargo deb --version "$TAG_VERSION"
DEB_PATH=$(find target/debian -type f -name "*.deb" | head -n 1)
if [[ -z "$DEB_PATH" ]]; then
  echo "❌ Debian package not found after building."
  exit 1
fi
mv "$DEB_PATH" "target/fin_${TAG_VERSION}_amd64.deb"

# --------------------------
# Final Verification
# --------------------------
echo "✅ Release artifacts:"
find target/ -name "fin-*" -exec ls -lh {} \; | awk '{print "- " $0}'

echo "🎉 Release ${TAG_VERSION} prepared successfully!"
echo "📦 Artifacts are in the target/ directory."
