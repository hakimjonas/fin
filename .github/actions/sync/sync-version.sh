#!/usr/bin/env bash
set -euo pipefail

echo "🔄 Starting Release Automation Process..."

# --------------------------
# Configuration
# --------------------------
PACKAGE_NAME="fin"
TARGET_DIR="target"
ARTIFACT_DIR="${TARGET_DIR}/artifacts"
DEB_ARCH="amd64"
PLATFORMS=("solus" "arch" "nix")

# --------------------------
# Version Determination
# --------------------------
determine_version() {
    TAG_VERSION=""
    # If TAG is empty, try to use GITHUB_REF (for tag pushes)
    if [[ -z "${TAG:-}" && -n "${GITHUB_REF:-}" ]]; then
      TAG="${GITHUB_REF##*/}"
      echo "🔍 Using tag from GITHUB_REF: $TAG"
    fi

    if [[ -n "${TAG:-}" ]]; then
        TAG_VERSION="${TAG#v}"
        echo "ℹ️ Using provided version: ${TAG_VERSION}"
    elif [[ -d ".git" ]]; then
        TAG_VERSION=$(git describe --tags --abbrev=0 2>/dev/null | sed 's/^v//')
    fi

    # Validate semantic version format
    if [[ ! "${TAG_VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "❌ Invalid/missing semantic version: '${TAG_VERSION}'"
        exit 1
    fi

    # Only increment if no TAG was provided
    if [[ -z "${TAG:-}" ]]; then
        IFS='.' read -r major minor patch <<< "${TAG_VERSION}"
        new_patch=$((patch + 1))
        TAG_VERSION="${major}.${minor}.${new_patch}"
        echo "🔼 Bumped version to: ${TAG_VERSION}"
    fi
}

# --------------------------
# Dependency Management
# --------------------------
check_dependencies() {
    # Define commands that must be available.
    local dependencies=("cargo" "tar")

    # Check for dpkg-deb. If missing, try to install if apt-get is available.
    if ! command -v dpkg-deb &>/dev/null; then
        echo "❌ Missing required dependency: dpkg-deb."
        if command -v apt-get &>/dev/null; then
            echo "📦 Attempting to install dpkg-deb..."
            sudo apt-get update && sudo apt-get install -y dpkg-deb
        else
            echo "❌ dpkg-deb is not installed and automatic installation is not supported on this system."
            exit 1
        fi
    fi

    for cmd in "${dependencies[@]}"; do
        if ! command -v "${cmd}" &>/dev/null; then
            echo "❌ Missing required dependency: ${cmd}"
            exit 1
        fi
    done

    # Install cargo-edit if missing
    if ! command -v cargo-set-version &>/dev/null; then
        echo "📦 Installing cargo-edit..."
        cargo install cargo-edit
    fi

    # Install cargo-deb if missing
    if ! command -v cargo-deb &>/dev/null; then
        echo "📦 Installing cargo-deb..."
        cargo install cargo-deb
    fi
}

# --------------------------
# File Updates
# --------------------------
update_project_files() {
    echo "📦 Updating Cargo.toml..."
    cargo set-version "${TAG_VERSION}"

    echo "📄 Updating INSTALL.md..."
    sed -i -E \
        -e "s/(${PACKAGE_NAME}-)[0-9]+\.[0-9]+\.[0-9]+(-arch\.tar\.gz)/\1${TAG_VERSION}\2/g" \
        -e "s/(${PACKAGE_NAME}-)[0-9]+\.[0-9]+\.[0-9]+(-nix\.tar\.gz)/\1${TAG_VERSION}\2/g" \
        -e "s/(${PACKAGE_NAME}-)[0-9]+\.[0-9]+\.[0-9]+(-solus\.tar\.gz)/\1${TAG_VERSION}\2/g" \
        -e "s/(${PACKAGE_NAME}_)[0-9]+\.[0-9]+\.[0-9]+(_${DEB_ARCH}\.deb)/\1${TAG_VERSION}\2/g" \
        INSTALL.md
}

# --------------------------
# Changelog Management
# --------------------------
update_changelog() {
    local changelog_file="CHANGELOG.md"
    local release_date
    release_date=$(date +%Y-%m-%d)
    echo "📝 Generating changelog entry..."
    local last_tag
    if [ -d ".git" ]; then
      last_tag=$(git tag --sort=-v:refname | sed 's/^v//' | grep -E "^[0-9]+\.[0-9]+\.[0-9]+$" | grep -v "^${TAG_VERSION}$" | head -n1)
    fi

    local new_entry="## [${TAG_VERSION}] - ${release_date}\n\n"
    if [[ -n "$last_tag" ]]; then
        new_entry+="### Changes since ${last_tag}:\n"
        local commit_log
        commit_log=$(git log "v${last_tag}"..HEAD --pretty=format:"- %s (%h)" || true)
        new_entry+="${commit_log:-No significant changes detected}\n"
    else
        new_entry+="### Initial Release\n"
    fi

    # Overwrite changelog (or prepend to it)
    local temp_file
    temp_file=$(mktemp)
    echo -e "$new_entry" > "$temp_file"
    if [[ -f "${changelog_file}" ]]; then
        cat "${changelog_file}" >> "$temp_file"
    fi
    mv "$temp_file" "$changelog_file"
}

# --------------------------
# Build Process
# --------------------------
build_project() {
    echo "🔨 Building project..."
    cargo build --release
}

# --------------------------
# Artifact Preparation
# --------------------------
prepare_artifacts() {
    echo "📦 Preparing packaging artifacts..."
    rm -rf "${ARTIFACT_DIR}"
    mkdir -p "${ARTIFACT_DIR}"

    for platform in "${PLATFORMS[@]}"; do
        local pkg_dir="${ARTIFACT_DIR}/${platform}"
        mkdir -p "${pkg_dir}"
        cp "${TARGET_DIR}/release/${PACKAGE_NAME}" "${pkg_dir}/"
        cp -r assets "${pkg_dir}/"
    done
}

# --------------------------
# Package Generation
# --------------------------
generate_packages() {
    echo "📦 Generating distribution packages..."

    for platform in "${PLATFORMS[@]}"; do
        echo "📦 Packaging ${platform}..."
        tar -czf "${TARGET_DIR}/${PACKAGE_NAME}-${TAG_VERSION}-${platform}.tar.gz" \
            -C "${ARTIFACT_DIR}/${platform}" .
    done

    echo "📦 Building Debian package..."
    cargo deb --target x86_64-unknown-linux-gnu \
        --no-build \
        --version "${TAG_VERSION}" \
        --output-dir "${TARGET_DIR}"

    local deb_pattern="${TARGET_DIR}/${PACKAGE_NAME}_${TAG_VERSION}_${DEB_ARCH}.deb"
    if ! ls "${deb_pattern}" 1> /dev/null 2>&1; then
        echo "❌ Debian package not found after building."
        exit 1
    fi
}

# --------------------------
# Final Verification
# --------------------------
final_verification() {
    echo "✅ Release artifacts:"
    find "${TARGET_DIR}/" -name "${PACKAGE_NAME}-*" -exec ls -lh {} \; | awk '{print "- " $0}'
    echo "🎉 Release ${TAG_VERSION} prepared successfully!"
    echo "📦 Artifacts are in the ${TARGET_DIR}/ directory."
}

# --------------------------
# Main Execution
# --------------------------
main() {
    determine_version
    check_dependencies
    update_project_files
    update_changelog
    build_project
    prepare_artifacts
    generate_packages
    final_verification
}

main
