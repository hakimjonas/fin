# Docker Image Requirements

## CircleCI Build Image: `hakimjonas/fin-deb-build`

This document specifies the requirements for the Docker image used in CircleCI builds.

### Required System Packages

The image must include the following system packages:

#### Core Build Tools
- `curl` - For downloading dependencies
- `git` - For source control operations
- `build-essential` (Debian/Ubuntu) or equivalent
- `pkg-config` / `pkgconf` - For library detection

#### GTK4 Development
- `libgtk-4-dev` (Debian/Ubuntu)
- `libglib2.0-dev`
- `libcairo2-dev`
- `libpango1.0-dev`

#### Rust Toolchain
- `rustc` - Rust compiler (stable channel)
- `cargo` - Rust package manager
- `rustup` (optional, for toolchain management)

####Release and Packaging Tools
- `jq` - JSON processor (used in `scripts/release.sh:71-75`)
- `gh` - GitHub CLI (used in `scripts/version-bump.sh:76-81`)
- `gpg` / `gnupg` - GPG for signing releases (used in `scripts/release.sh:46-47`)

### Required Cargo Tools

These tools must be pre-installed or installed during CI runs:

1. **cargo-make** (v0.37+)
   - Used in: `scripts/release.sh:52`
   - Installation: `cargo install cargo-make`

2. **cargo-deb** (v1.42.0)
   - Used in: `Makefile.toml:109, .circleci/config.yml:75`
   - Installation: `cargo install cargo-deb --version 1.42.0`

3. **cargo-audit** (v0.21+)
   - Used in: `.circleci/config.yml:55-60`
   - Installation: `cargo install cargo-audit`

### Environment Variables

The following environment variables must be set in CircleCI project settings:

- `GH_TOKEN` - GitHub personal access token with repo permissions
- `FINE_SIGNATURE_KEY_B64` - Base64-encoded GPG private key for signing
- `FINE_SIGNATURE_PASSPHRASE` - Passphrase for the GPG key
- `CIRCLE_SHA1` - (Automatically provided by CircleCI)

### Dockerfile Example

```dockerfile
FROM ubuntu:latest

# Install system dependencies
RUN apt-get update && apt-get install -y \\
    curl \\
    git \\
    build-essential \\
    pkg-config \\
    libgtk-4-dev \\
    libglib2.0-dev \\
    libcairo2-dev \\
    libpango1.0-dev \\
    jq \\
    gnupg \\
    && rm -rf /var/lib/apt/lists/*

# Install GitHub CLI
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \\
    && chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg \\
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \\
    && apt-get update \\
    && apt-get install -y gh \\
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

# Pre-install Cargo tools to speed up CI
RUN cargo install cargo-make cargo-deb --version 1.42.0 && cargo install cargo-audit

# Set working directory
WORKDIR /workspace

CMD ["/bin/bash"]
```

### Building the Image

```bash
docker build -t hakimjonas/fin-deb-build:latest .
docker push hakimjonas/fin-deb-build:latest
```

### Verification

To verify the image has all required tools:

```bash
docker run --rm hakimjonas/fin-deb-build:latest bash -c "
    command -v curl && echo '✓ curl' || echo '✗ curl' &&
    command -v git && echo '✓ git' || echo '✗ git' &&
    command -v jq && echo '✓ jq' || echo '✗ jq' &&
    command -v gh && echo '✓ gh CLI' || echo '✗ gh CLI' &&
    command -v gpg && echo '✓ gpg' || echo '✗ gpg' &&
    command -v cargo && echo '✓ cargo' || echo '✗ cargo' &&
    command -v rustc && echo '✓ rustc' || echo '✗ rustc' &&
    cargo make --version && echo '✓ cargo-make' || echo '✗ cargo-make' &&
    cargo deb --version && echo '✓ cargo-deb' || echo '✗ cargo-deb' &&
    cargo audit --version && echo '✓ cargo-audit' || echo '✗ cargo-audit' &&
    pkg-config --exists gtk4 && echo '✓ GTK4 dev' || echo '✗ GTK4 dev'
"
```

### Maintenance

- Update the image when new Rust stable releases are available
- Keep cargo tools updated to latest stable versions
- Regularly update system packages for security patches

### Notes

- The image size can be optimized by using multi-stage builds
- Consider using Alpine Linux for smaller image size (may require musl adjustments)
- Cache `/root/.cargo/registry` and `/root/.cargo/git` for faster builds
