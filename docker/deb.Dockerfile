# deb.Dockerfile
FROM ubuntu:24.10

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    libgtk-4-dev \
    build-essential \
    pkg-config \
    git

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