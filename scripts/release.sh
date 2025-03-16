#!/bin/bash
set -euo pipefail

echo "=== Starting Release Process ==="

# Force-fetch tags (to update local tags and avoid conflicts)
git fetch --tags --force

# Determine new version by reading Cargo.toml and forcing a "v" prefix.
new_version=$(grep '^version' Cargo.toml | sed -E 's/version *= *"(.*)"/v\1/')
if [[ -z "$new_version" ]]; then
  echo "❌ Could not determine new version. Exiting release process."
  exit 1
fi
echo "Tag detected: releasing version $new_version"

# Verify required environment variables.
for var in GH_TOKEN FINE_SIGNATURE_KEY_B64 FINE_SIGNATURE_PASSPHRASE CIRCLE_SHA1; do
  if [ -z "${!var:-}" ]; then
    echo "❌ $var is not set!"
    exit 1
  fi
done
echo "✅ All required environment variables are set."

# Repository details.
REPO_OWNER="hakimjonas"
REPO_NAME="fin"

# Optional: Print the Cargo.toml version for debugging.
echo "Current Cargo.toml version:"
grep '^version' Cargo.toml

# Configure GPG: Import your key and set non-interactive mode.
mkdir -p ~/.gnupg
chmod 700 ~/.gnupg
printf '%s' "$FINE_SIGNATURE_KEY_B64" | base64 -d | gpg --batch --import
echo "pinentry-mode loopback" >> ~/.gnupg/gpg.conf

# Build and package.
cargo build --release
cargo package --allow-dirty
cargo make package

# Sign release assets – only process regular files.
for file in target/package/fin-*; do
  if [[ -f "$file" ]]; then
    echo "🔑 Signing file: $file"
    gpg --batch --yes --pinentry-mode loopback --passphrase "$FINE_SIGNATURE_PASSPHRASE" --detach-sign --armor "$file"
    sha256sum "$file" > "$file.sha256"
  else
    echo "⚠️ Skipping non-file: $file"
  fi
done

echo "✅ Finished signing assets."

# Create GitHub release.
api_url="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases"

# Use jq to generate valid JSON payload.
release_payload=$(jq -n \
  --arg tag "$new_version" \
  --arg name "Release $new_version" \
  --arg body "Release $new_version" \
  '{tag_name: $tag, name: $name, body: $body, draft: false, prerelease: false}')

echo "🚀 Creating GitHub release..."
release_response=$(curl -s -X POST "$api_url" \
  -H "Authorization: token $GH_TOKEN" \
  -H "Accept: application/vnd.github.v3+json" \
  -d "$release_payload")

echo "Release response:"
echo "$release_response"

if echo "$release_response" | grep -q '"html_url"'; then
  echo "✅ GitHub release created successfully."
else
  echo "❌ Failed to create GitHub release."
  echo "Response: $release_response"
  exit 1
fi

# Parse the upload URL from the release response using jq.
upload_url=$(echo "$release_response" | jq -r '.upload_url' | sed 's/{?name,label}//')
echo "Parsed upload URL: $upload_url"

# List artifact patterns for distro-specific assets.
assets=(
  "target/debian/fin_*.deb"
  "target/fin-*-solus.tar.gz"
  "target/fin-*-arch.tar.gz"
  "target/fin-*-nix.tar.gz"
)

# Upload each asset.
for pattern in "${assets[@]}"; do
  for asset in $pattern; do
    if [[ -f "$asset" ]]; then
      echo "Uploading asset: $asset"
      curl -s --data-binary @"$asset" \
        -H "Content-Type: application/octet-stream" \
        -H "Authorization: token $GH_TOKEN" \
        "$upload_url?name=$(basename "$asset")"
    else
      echo "No asset found for pattern: $pattern"
    fi
  done
done

echo "Release process completed successfully."
