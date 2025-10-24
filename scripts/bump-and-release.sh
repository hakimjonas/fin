#!/bin/bash
set -euo pipefail

echo "=== Starting Bump and Release Process ==="

# Verify required environment variables
for var in GH_TOKEN FINE_SIGNATURE_KEY_B64 FINE_SIGNATURE_PASSPHRASE CIRCLE_SHA1; do
  if [ -z "${!var:-}" ]; then
    echo "❌ $var is not set!"
    exit 1
  fi
done
echo "✅ All required environment variables are set."

# Prevent running on version bump commits
if git log -1 --pretty=%B | grep -q "Bump version to"; then
  echo "🚫 Last commit is already a version bump. Skipping to prevent double-bump."
  exit 0
fi

# Get current version and bump it
current_version=$(awk -F'"' '/^version *=/ {print $2; exit}' Cargo.toml)
echo "Current version: $current_version"

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

# Update version in all files
sed -i "s/^version *= *\"[^\"]*\"/version = \"$new_version\"/" Cargo.toml
sed -i "s/^pkgver=.*/pkgver=$new_version/" PKGBUILD
sed -i "s|<Version>[^<]*</Version>|<Version>$new_version</Version>|" fin.sol
sed -i "s/^[[:space:]]*version *= *\"[^\"]*\";/  version = \"$new_version\";/" flake.nix
sed -i "s/$current_version/$new_version/g" INSTALL.md

# Update CHANGELOG
if grep -q "^## \[Unreleased\]" CHANGELOG.md; then
  sed -i "s/^## \[Unreleased\]/## [$new_version] - $(date +%Y-%m-%d)/" CHANGELOG.md
else
  echo -e "## [$new_version] - $(date +%Y-%m-%d)\n" | cat - CHANGELOG.md > CHANGELOG.tmp && mv CHANGELOG.tmp CHANGELOG.md
fi

# Configure git
git config user.email "ci-bot@example.com"
git config user.name "CI Bot"

# Commit version bump directly to trunk
git add Cargo.toml PKGBUILD fin.sol flake.nix INSTALL.md CHANGELOG.md Cargo.lock
git commit -m "Bump version to $new_version"
git push origin HEAD:refs/heads/trunk

echo "✅ Version bumped to $new_version and pushed to trunk"

# Create and push tag
tag="v$new_version"
echo "Creating tag: $tag"
git tag -a "$tag" -m "Release $tag"
git push origin "$tag"
echo "✅ Tag $tag created and pushed"

# Configure GPG
mkdir -p ~/.gnupg
chmod 700 ~/.gnupg
printf '%s' "$FINE_SIGNATURE_KEY_B64" | base64 -d | gpg --batch --import
echo "pinentry-mode loopback" >> ~/.gnupg/gpg.conf

# Build and package
echo "🔨 Building release..."
cargo build --release
cargo make package

# Sign release assets
sign_files=(
  target/debian/fin_*.deb
  target/fin-*-solus.tar.gz
  target/fin-*-arch.tar.gz
  target/fin-*-nix.tar.gz
)

echo "🔑 Signing release assets..."
for pattern in "${sign_files[@]}"; do
  for file in $pattern; do
    if [[ -f "$file" ]]; then
      echo "Signing: $file"
      gpg --batch --yes --pinentry-mode loopback --passphrase "$FINE_SIGNATURE_PASSPHRASE" --detach-sign --armor "$file"
      sha256sum "$file" > "$file.sha256"
    fi
  done
done

echo "✅ Assets signed"

# Create GitHub release
REPO_OWNER="hakimjonas"
REPO_NAME="fin"
api_url="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases"

release_payload=$(jq -n \
  --arg tag "$tag" \
  --arg name "Release $new_version" \
  --arg body "Release $new_version" \
  '{tag_name: $tag, name: $name, body: $body, draft: false, prerelease: false}')

echo "🚀 Creating GitHub release..."
release_response=$(curl -s -X POST "$api_url" \
  -H "Authorization: token $GH_TOKEN" \
  -H "Accept: application/vnd.github.v3+json" \
  -d "$release_payload")

if echo "$release_response" | grep -q '"html_url"'; then
  echo "✅ GitHub release created"
else
  echo "❌ Failed to create release"
  echo "$release_response"
  exit 1
fi

# Upload assets
upload_url=$(echo "$release_response" | jq -r '.upload_url' | sed 's/{?name,label}//')

assets=(
  "target/debian/fin_*.deb"
  "target/fin-*-solus.tar.gz"
  "target/fin-*-arch.tar.gz"
  "target/fin-*-nix.tar.gz"
)

echo "📦 Uploading release assets..."
for pattern in "${assets[@]}"; do
  for asset in $pattern; do
    if [[ -f "$asset" ]]; then
      echo "Uploading: $(basename "$asset")"
      curl -s --data-binary @"$asset" \
        -H "Content-Type: application/octet-stream" \
        -H "Authorization: token $GH_TOKEN" \
        "$upload_url?name=$(basename "$asset")"

      # Upload signature
      if [[ -f "$asset.asc" ]]; then
        curl -s --data-binary @"$asset.asc" \
          -H "Content-Type: application/octet-stream" \
          -H "Authorization: token $GH_TOKEN" \
          "$upload_url?name=$(basename "$asset").asc"
      fi

      # Upload checksum
      if [[ -f "$asset.sha256" ]]; then
        curl -s --data-binary @"$asset.sha256" \
          -H "Content-Type: text/plain" \
          -H "Authorization: token $GH_TOKEN" \
          "$upload_url?name=$(basename "$asset").sha256"
      fi
    fi
  done
done

echo "✅ Release complete: https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/tag/$tag"
echo "=== Bump and Release Process Complete ==="
