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

# Repository details (adjusted based on your repo URL)
REPO_OWNER="hakimjonas"
REPO_NAME="fin"

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
  if [[ -f "$file" ]]; then
    echo "🔑 Signing file: $file"
    gpg --detach-sign --armor "$file"
    sha256sum "$file" > "$file.sha256"
  else
    echo "⚠️ Skipping non-file: $file"
  fi
done

echo "✅ Finished signing assets."

# Create GitHub release using the GitHub API.
api_url="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases"
release_payload=$(cat <<EOF
{
  "tag_name": "$new_version",
  "name": "Release $new_version",
  "body": "Release $new_version",
  "draft": false,
  "prerelease": false
}
EOF
)

echo "🚀 Creating GitHub release..."
release_response=$(curl -s -v --max-time 30 -X POST "$api_url" \
  -H "Authorization: token $GH_TOKEN" \
  -H "Accept: application/vnd.github.v3+json" \
  -d "$release_payload")

echo "🔍 Curl response:"
echo "$release_response"

if echo "$release_response" | grep -q '"html_url"'; then
  echo "✅ GitHub release created successfully."
else
  echo "❌ Failed to create GitHub release."
  echo "Response: $release_response"
  exit 1
fi

echo "Release process completed successfully."
