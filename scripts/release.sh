#!/bin/bash
set -euo pipefail

echo "=== Starting Release Process ==="

# Exit if no tag is detected.
if [ -z "${CIRCLE_TAG:-}" ]; then
  echo "No tag detected. Exiting release process."
  exit 0
fi

# Verify required environment variables.
for var in GH_TOKEN FINE_SIGNATURE_KEY_B64 FINE_SIGNATURE_PASSPHRASE CIRCLE_SHA1; do
  if [ -z "${!var:-}" ]; then
    echo "❌ $var is not set!"
    exit 1
  fi
done
echo "✅ All required environment variables are set."

# Store GH_TOKEN locally and then unset it.
token="$GH_TOKEN"
unset GH_TOKEN

# Authenticate with GitHub CLI.
echo "$token" | gh auth login --with-token
gh auth status

# Configure GPG.
mkdir -p ~/.gnupg
chmod 700 ~/.gnupg
printf '%s' "$FINE_SIGNATURE_KEY_B64" | base64 -d | gpg --batch --import
echo "pinentry-mode loopback" >> ~/.gnupg/gpg.conf

# Determine new version from the tag.
new_version="${CIRCLE_TAG#v}"
echo "Tag detected: releasing version $new_version"

# Build and package.
rustup default stable
cargo build --release
cargo package --allow-dirty
echo "Packaging distribution-specific files..."
cargo make package

# Sign release assets.
shopt -s nullglob
for file in target/debian/fin_*_amd64.deb target/fin-*-arch.tar.gz target/fin-*-solus.tar.gz target/fin-*-nix.tar.gz; do
  if [ -f "$file" ]; then
    gpg --detach-sign --armor "$file"
    sha256sum "$file" > "$file.sha256"
  else
    echo "File $file does not exist, skipping signing."
  fi
done

# Gather asset files.
assets=()
for file in target/debian/fin_*_amd64.deb target/fin-*-arch.tar.gz target/fin-*-solus.tar.gz target/fin-*-nix.tar.gz; do
  if [ -f "$file" ]; then
    assets+=("$file")
  fi
done
if [ ${#assets[@]} -eq 0 ]; then
  for file in target/package/fin-*; do
    if [ -f "$file" ]; then
      assets+=("$file")
    fi
  done
fi

# Ensure the tag exists locally and push it if necessary.
if git rev-parse "v$new_version" >/dev/null 2>&1; then
  echo "Tag v$new_version exists locally, pushing tag."
  git push origin "v$new_version"
else
  echo "Creating new tag v$new_version."
  git tag "v$new_version"
  git push origin "v$new_version"
fi

# Create or update the GitHub release.
if gh release view "v$new_version" >/dev/null 2>&1; then
  echo "Release v$new_version exists; updating release."
  gh release edit "v$new_version" --title "Release v$new_version" --notes "Release $new_version"
else
  if [ ${#assets[@]} -eq 0 ]; then
    echo "No assets found. Creating release without assets."
    gh release create "v$new_version" --title "Release v$new_version" --notes "Release $new_version"
  else
    gh release create "v$new_version" --title "Release v$new_version" --notes "Release $new_version" "${assets[@]}"
  fi
fi

echo "=== Release process complete. New version: $new_version ==="
