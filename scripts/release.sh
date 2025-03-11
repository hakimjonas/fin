#!/bin/bash
set -euo pipefail

echo "=== Starting Release Process ==="

# 1. Verify required environment variables.
for var in GH_TOKEN FINE_SIGNATURE_KEY_B64 FINE_SIGNATURE_PASSPHRASE CIRCLE_SHA1; do
  if [ -z "${!var:-}" ]; then
    echo "❌ $var is not set!"
    exit 1
  fi
done
echo "✅ All required environment variables are set."

# 2. Authenticate with GitHub CLI.
echo "$GH_TOKEN" | gh auth login --with-token
gh auth status

# 3. Configure GPG.
mkdir -p ~/.gnupg
chmod 700 ~/.gnupg
printf '%s' "$FINE_SIGNATURE_KEY_B64" | base64 -d | gpg --batch --import
echo "pinentry-mode loopback" >> ~/.gnupg/gpg.conf

# 4. Read current version from Cargo.toml.
current_version=$(awk -F'"' '/^version *= */ {print $2; exit}' Cargo.toml)
echo "Current version: $current_version"

# 5. Guard: if a release for current_version already exists, abort.
if gh release view "v$current_version" >/dev/null 2>&1; then
  echo "Version $current_version is already released. Aborting bump."
  exit 0
fi

# 6. Calculate new version by bumping patch.
if [[ "$current_version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-.*)?$ ]]; then
  major="${BASH_REMATCH[1]}"
  minor="${BASH_REMATCH[2]}"
  patch="${BASH_REMATCH[3]}"
  suffix="${BASH_REMATCH[4]:-}"
  new_patch=$((patch + 1))
  new_version="${major}.${minor}.${new_patch}${suffix}"
  echo "Bumping version: $current_version → $new_version"
else
  echo "❌ Invalid version format: $current_version"
  exit 1
fi

# 7. Update version in files.
sed -i "s/^version *= *\".*\"/version = \"$new_version\"/" Cargo.toml
sed -i "s/^pkgver=.*/pkgver=$new_version/" PKGBUILD
# Update fin.sol by replacing the XML <Version> tag.
sed -i "s|<Version>[^<]*</Version>|<Version>$new_version</Version>|" fin.sol
sed -i "s/^[[:space:]]*version *= *\"[^\"]*\";/  version = \"$new_version\";/" flake.nix
# Replace all occurrences of the old version in INSTALL.md.
sed -i "s/$current_version/$new_version/g" INSTALL.md
# Update CHANGELOG.md: if an "Unreleased" header exists, update it; otherwise, prepend one.
if grep -q "^## \[Unreleased\]" CHANGELOG.md; then
  sed -i "s/^## \[Unreleased\]/## [$new_version] - $(date +%Y-%m-%d)/" CHANGELOG.md
else
  echo -e "## [$new_version] - $(date +%Y-%m-%d)\n\n$(cat CHANGELOG.md)" > CHANGELOG.md
  echo "Appended new changelog header."
fi

# 8. Verify that critical files were updated.
if ! grep -q "<Version>$new_version</Version>" fin.sol; then
  echo "❌ fin.sol did not update to version $new_version"
  exit 1
fi
if ! grep -q "$new_version" INSTALL.md; then
  echo "❌ INSTALL.md did not update to version $new_version"
  exit 1
fi

# 9. Commit changes to a new bump branch.
git config user.email "ci-bot@example.com"
git config user.name "CI Bot"
bump_branch="version-bump-$new_version"
git checkout -b "$bump_branch"
git add Cargo.toml PKGBUILD fin.sol flake.nix INSTALL.md CHANGELOG.md
if git diff --cached --quiet; then
  echo "No changes to commit"
else
  git commit -m "Bump version to $new_version"
  git push origin "$bump_branch"
  pr_url=$(gh pr create --fill --base trunk --head "$bump_branch")
  echo "Created bump PR: $pr_url"
fi

# 10. Build and package.
cargo build --release
cargo package --allow-dirty

# 11. Build distro-specific packages.
if [ -x ./scripts/build_distro_packages.sh ]; then
  echo "Building distro-specific packages..."
  ./scripts/build_distro_packages.sh
else
  echo "No distro packaging script found, skipping distro package build."
fi

# 12. Sign release assets.
shopt -s nullglob
for file in target/debian/fin_*_amd64.deb target/fin-*-arch.tar.gz target/fin-*-solus.tar.gz target/fin-*-nix.tar.gz; do
  if [ -f "$file" ]; then
    gpg --detach-sign --armor "$file"
    sha256sum "$file" > "$file.sha256"
  else
    echo "File $file does not exist, skipping signing."
  fi
done

# 13. Gather asset files.
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

# 14. Handle tag: push existing or create new.
if git rev-parse "v$new_version" >/dev/null 2>&1; then
  echo "Tag v$new_version exists locally, pushing tag."
  git push origin "v$new_version"
else
  echo "Creating new tag v$new_version."
  git tag "v$new_version"
  git push origin "v$new_version"
fi

# 15. Create or update GitHub release.
if gh release view "v$new_version" >/dev/null 2>&1; then
  echo "Release v$new_version already exists; updating release."
  gh release edit "v$new_version" --title "Release v$new_version" --notes "Release $new_version"
else
  if [ ${#assets[@]} -eq 0 ]; then
    echo "No release assets found, creating release without assets."
    gh release create "v$new_version" --title "Release v$new_version" --notes "Release $new_version"
  else
    gh release create "v$new_version" --title "Release v$new_version" --notes "Release $new_version" "${assets[@]}"
  fi
fi

# 16. Auto-merge the bump PR.
pr_number=$(gh pr view --json number --jq ".number")
if [ -n "$pr_number" ]; then
  echo "Auto-merging bump PR #$pr_number"
  gh pr merge "$pr_number" --squash --delete-branch --auto
else
  echo "No bump PR found to merge."
fi

echo "=== Release process complete. New version: $new_version ==="
