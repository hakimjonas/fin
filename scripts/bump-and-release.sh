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

REPO_OWNER="hakimjonas"
REPO_NAME="fin"
API_URL="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases"

auth_header() {
  echo "Authorization: token $GH_TOKEN"
}

# Integrate any commits already pushed to trunk (e.g. from a previous run whose
# release step failed after pushing the version bump). CircleCI reruns use the
# original build SHA, so without this the push is rejected (non-fast-forward).
echo "🔄 Syncing with origin/trunk..."
git fetch origin trunk
git rebase "origin/trunk" || { git rebase --abort; echo "❌ Rebase conflict, aborting"; exit 1; }

# Get current version
current_version=$(awk -F'"' '/^version *=/ {print $2; exit}' Cargo.toml)
echo "Current version: $current_version"
tag="v$current_version"

# Decide whether we still need to bump. Skip if the last commit is already a
# version bump, or if the tag was already pushed by a previous run.
need_bump=true
if git log -1 --pretty=%B | grep -q "Bump version to"; then
  echo "🚫 Last commit is already a version bump. Skipping bump."
  need_bump=false
elif git ls-remote --tags --exit-code origin "refs/tags/$tag" >/dev/null 2>&1; then
  echo "🚫 Tag $tag already exists remotely. Skipping bump."
  need_bump=false
fi

if [ "$need_bump" = true ]; then
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

  # Update AerynOS stone.yaml
  sed -i "s/^version[[:space:]]*:[[:space:]]*.*/version     : $new_version/" packaging/aeryn/stone.yaml
  sed -i "s#git|https://github.com/hakimjonas/fin.git : v[0-9.]*#git|https://github.com/hakimjonas/fin.git : v$new_version#" packaging/aeryn/stone.yaml

  # Update Cargo.lock with new version
  echo "Updating Cargo.lock..."
  cargo update -p fin

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
  git add Cargo.toml PKGBUILD fin.sol flake.nix INSTALL.md CHANGELOG.md Cargo.lock packaging/aeryn/stone.yaml
  git commit -m "Bump version to $new_version [skip ci]"

  # Push, integrating any remote changes first to avoid non-fast-forward rejects
  git fetch origin trunk
  if ! git rebase "origin/trunk"; then
    git rebase --abort
    echo "❌ Rebase conflict while pushing bump, aborting"
    exit 1
  fi
  git push origin HEAD:refs/heads/trunk
  echo "✅ Version bumped to $new_version and pushed to trunk"

  # Create and push tag
  tag="v$new_version"
  echo "Creating tag: $tag"
  git tag -a "$tag" -m "Release $tag"
  git push origin "$tag"
  echo "✅ Tag $tag created and pushed"
fi

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

# Find existing release for this tag, or create it (idempotent)
echo "🚀 Ensuring GitHub release exists for $tag..."
release_info=$(curl -s -H "$(auth_header)" -H "Accept: application/vnd.github.v3+json" "$API_URL/tags/$tag")

if echo "$release_info" | jq -e '.id' >/dev/null 2>&1; then
  echo "✅ Release already exists for $tag, reusing it."
  release_id=$(echo "$release_info" | jq -r '.id')
  upload_url=$(echo "$release_info" | jq -r '.upload_url' | sed 's/{?name,label}//')
else
  release_payload=$(jq -n \
    --arg tag "$tag" \
    --arg name "Release ${tag#v}" \
    --arg body "Release ${tag#v}" \
    '{tag_name: $tag, name: $name, body: $body, draft: false, prerelease: false}')

  release_response=$(curl -s -X POST "$API_URL" \
    -H "$(auth_header)" \
    -H "Accept: application/vnd.github.v3+json" \
    -d "$release_payload")

  if echo "$release_response" | jq -e '.id' >/dev/null 2>&1; then
    echo "✅ GitHub release created"
    release_id=$(echo "$release_response" | jq -r '.id')
    upload_url=$(echo "$release_response" | jq -r '.upload_url' | sed 's/{?name,label}//')
  else
    echo "❌ Failed to create release"
    echo "$release_response"
    exit 1
  fi
fi

# Upload assets (skip those already uploaded)
existing_assets=$(curl -s -H "$(auth_header)" "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/$release_id/assets" | jq -r '.[].name')

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
      name=$(basename "$asset")

      # Upload main asset if missing
      if echo "$existing_assets" | grep -qxF "$name"; then
        echo "⏭️  $name already uploaded, skipping"
      else
        echo "Uploading: $name"
        curl -s --data-binary @"$asset" \
          -H "Content-Type: application/octet-stream" \
          -H "$(auth_header)" \
          "$upload_url?name=$name"
      fi

      # Upload signature
      if [[ -f "$asset.asc" ]]; then
        sig_name="$name.asc"
        if echo "$existing_assets" | grep -qxF "$sig_name"; then
          echo "⏭️  $sig_name already uploaded, skipping"
        else
          echo "Uploading: $sig_name"
          curl -s --data-binary @"$asset.asc" \
            -H "Content-Type: application/octet-stream" \
            -H "$(auth_header)" \
            "$upload_url?name=$sig_name"
        fi
      fi

      # Upload checksum
      if [[ -f "$asset.sha256" ]]; then
        sum_name="$name.sha256"
        if echo "$existing_assets" | grep -qxF "$sum_name"; then
          echo "⏭️  $sum_name already uploaded, skipping"
        else
          echo "Uploading: $sum_name"
          curl -s --data-binary @"$asset.sha256" \
            -H "Content-Type: text/plain" \
            -H "$(auth_header)" \
            "$upload_url?name=$sum_name"
        fi
      fi
    fi
  done
done

echo "✅ Release complete: https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/tag/$tag"
echo "=== Bump and Release Process Complete ==="
