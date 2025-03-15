#!/bin/bash
set -euo pipefail

echo "=== Starting Release Process ==="

# Ensure we have the latest tags
git fetch --tags

# Detect latest version from tag.
new_version=$(git describe --tags --abbrev=0)
if [[ -z "$new_version" ]]; then
  echo "❌ No version tag found. Exiting release process."
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

# Authenticate with GitHub CLI.
echo "$GH_TOKEN" | gh auth login --with-token
unset GH_TOKEN
env -u GH_TOKEN gh auth status || { echo "❌ GitHub CLI authentication failed"; exit 1; }

# Configure GPG.
mkdir -p ~/.gnupg
chmod 700 ~/.gnupg
printf '%s' "$FINE_SIGNATURE_KEY_B64" | base64 -d | gpg --batch --import
echo "pinentry-mode loopback" >> ~/.gnupg/gpg.conf

# Build and package.
cargo build --release
cargo package --allow-dirty
cargo make package

# Sign release assets.
for file in target/package/fin-*; do
  gpg --detach-sign --armor "$file"
  sha256sum "$file" > "$file.sha256"
done

# Create GitHub release.
gh release create "$new_version" --title "Release $new_version" --notes "Release $new_version" target/package/*
