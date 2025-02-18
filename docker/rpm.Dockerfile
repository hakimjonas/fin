# rpm.Dockerfile
FROM fedora:41

# Install system and RPM build dependencies
RUN dnf update -y && dnf install -y \
    curl \
    git \
    rpm-build \
    rpmdevtools \
    gtk4-devel \
    pkg-config \
    gcc \
    gcc-c++ \
    make \
    nodejs \
    npm \
  && dnf clean all

# Set up the RPM build tree (creates ~/rpmbuild/{BUILD,RPMS,SOURCES,SPECS,SRPMS})
RUN rpmdev-setuptree

# Install Rust via rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install cargo-make and cargo-rpm (a tool to help build RPMs for Rust projects)
RUN cargo install cargo-make
RUN cargo install cargo-rpm

# (Optional) If you need a more recent Node.js version,
# you can update using Nodesource’s script:
# RUN curl -fsSL https://rpm.nodesource.com/setup_20.x | bash - && dnf install -y nodejs

# Set the working directory
WORKDIR /workspace

# Copy your project files (assumes Docker build context is set to the project root)
COPY .. .

# Fetch dependencies and install Node.js dependencies for your GitHub action (if needed)
RUN cargo fetch
RUN npm install --prefix .github/actions/sync

# Build your project (you can keep your testing/build steps as before)
RUN cargo build
RUN cargo test
RUN cargo build --release

# Finally, build the RPM package using cargo-rpm
RUN cargo rpm build
