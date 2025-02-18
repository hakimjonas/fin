# solus.Dockerfile

# Use the Solus solbuild base image
FROM silkeh/solus:solbuild

# Update and install dependencies
RUN sudo eopkg up -y && \
    sudo eopkg it -y curl libgtk-4-devel gcc pkgconf git

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Initialize solbuild with the unstable profile (adjust if needed)
RUN sudo solbuild init -p unstable-x86_64

# Set working directory
WORKDIR /workspace

# Copy project files
COPY .. .

# Build the Rust project
RUN cargo build --release

# Build the Solus package
RUN sudo solbuild build -p unstable-x86_64 fin.sol