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

# Install cargo-make
RUN cargo install cargo-make

# Install Node.js
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs

# Set the working directory
WORKDIR /workspace

# Copy the project files
COPY .. .

# Install Rust dependencies
RUN cargo fetch

# Install Node.js dependencies
RUN npm install --prefix .github/actions/sync

# Build the project
RUN cargo build

# Test the project
RUN cargo test

# Build the project in release mode
RUN cargo build --release