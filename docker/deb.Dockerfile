FROM ubuntu:24.10

# Install system dependencies and GitHub CLI dependencies
RUN apt-get update && apt-get install -y \
    curl \
    libgtk-4-dev \
    build-essential \
    pkg-config \
    git \
    ca-certificates

# Install GitHub CLI
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null && \
    apt-get update && apt-get install -y gh

# Set PKG_CONFIG_PATH environment variable
ENV PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install cargo-make and cargo-audit
RUN cargo install cargo-make cargo-audit

# Set the working directory
WORKDIR /workspace

# Copy the project files
COPY . .

# Update Rust toolchain and dependencies
RUN rustup update && cargo update

# Install Rust dependencies
RUN cargo fetch

# Build the project
RUN cargo build

# Test the project
RUN cargo test

# Build the project in release mode
RUN cargo build --release
