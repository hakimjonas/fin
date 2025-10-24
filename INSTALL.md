# Installation

## System Requirements

Finë requires the following runtime dependencies to be installed on your system:

### Required Dependencies

- **GTK4** (>= 4.8): The graphical toolkit
  - Debian/Ubuntu: `libgtk-4-1`
  - Arch Linux: `gtk4`
  - Solus: `gtk4`
  - Fedora/RHEL: `gtk4`

- **GLib** (>= 2.76): Core library
  - Debian/Ubuntu: `libglib2.0-0` (usually installed with GTK4)
  - Arch Linux: `glib2` (dependency of gtk4)
  - Solus: `glib2` (dependency of gtk4)

- **shared-mime-info**: MIME type definitions
  - Debian/Ubuntu: `shared-mime-info`
  - Arch Linux: `shared-mime-info`
  - Solus: `shared-mime-info`

### Recommended (Optional)

- **adwaita-icon-theme**: Default icon theme for better visual appearance
- **GStreamer plugins**: For media playback support (if needed by GTK4)
  - Debian/Ubuntu: `libgtk-4-bin`, `libgtk-4-media-gstreamer`
  - Arch Linux: `gst-plugins-base`, `gst-plugins-good`

### Build Dependencies (Only for manual compilation)

- **Rust** (>= 1.70): Install via [rustup](https://rustup.rs/)
- **GTK4 and related development files**:
  - Debian/Ubuntu: `libgtk-4-dev`, `libgdk-pixbuf-2.0-dev`, `libgraphene-1.0-dev`
  - Arch Linux: `gtk4`, `gdk-pixbuf2`, `graphene` (includes development files)
  - Solus: `libgtk-4-devel`, `gdk-pixbuf-devel`, `graphene-devel`, `pango-devel`
  - Fedora/RHEL: `gtk4-devel`, `gdk-pixbuf2-devel`, `graphene-devel`
  - NixOS: `gtk4`, `gdk-pixbuf`, `graphene`, `glib.dev`
- **pkg-config**: Build tool for finding libraries
  - Debian/Ubuntu: `pkg-config`
  - Arch Linux: `pkgconf`
  - Solus: `pkgconf`
  - Fedora/RHEL: `pkg-config`
- **Build essentials**: Compiler and build tools
  - Debian/Ubuntu: `build-essential`
  - Arch Linux: `base-devel`
  - Solus: `gcc`, `make`
  - Fedora/RHEL: `gcc`
- **cargo-make**: Rust task runner (optional, for using `cargo make install`)
  - Install via: `cargo install cargo-make`

## Installation via Packages

**⚠️ Warning! Prebuild packages are still untested and may not work as expected. Please report any issues you encounter.
**

You can find pre-built packages here for various distributions:
[Finë Releases](https://github.com/hakimjonas/fin/releases).

**Note:** Pre-built packages will automatically install required dependencies.

### Available Packages

- `fin-0.2.21-arch.tar.gz` (811 KB) - 2025-02-11T23:22:55Z
- `fin-0.2.21-nix.tar.gz` (811 KB) - 2025-02-11T23:22:56Z
- `fin-0.2.21-solus.tar.gz` (811 KB) - 2025-02-11T23:22:55Z
- `fin_0.2.21_amd64.deb` (632 KB) - 2025-02-11T23:22:54Z
- Source code (zip) - 2025-02-11T23:13:13Z
- Source code (tar.gz) - 2025-02-11T23:13:13Z

### Solus (tarball)

To install the Solus tarball, use the following command:

```sh
sudo eopkg it fin-0.2.21-solus.tar.gz
```

### Arch Linux (tarball or PKGBUILD)

To install the Arch tarball, use the following command:

```sh
sudo pacman -U fin-0.2.21-arch.tar.gz
```

Alternatively, you can install the package from the AUR using an AUR helper like `yay`:

```sh
yay -S fin
```

### Debian-based Distributions (.deb)

To install the `.deb` package on Ubuntu or other Debian-based distributions, use the following command:

```sh
sudo dpkg -i fin_0.2.21_amd64.deb
```

If there are any missing dependencies, you can resolve them with:

```sh
sudo apt-get install -f
```

### Manual Installation

If you prefer to manually build and install the application, follow these steps:

#### 1. Install Dependencies

First, install the required dependencies for your distribution:

**Debian/Ubuntu:**
```sh
sudo apt-get update
sudo apt-get install -y libgtk-4-dev libgdk-pixbuf-2.0-dev libgraphene-1.0-dev pkg-config build-essential
```

**Arch Linux:**
```sh
sudo pacman -Syu --noconfirm gtk4 gdk-pixbuf2 graphene base-devel pkgconf
```

**Solus:**
```sh
sudo eopkg up
sudo eopkg install -y pkgconf libgtk-4-devel gdk-pixbuf-devel graphene-devel pango-devel gcc make
```

**Fedora/RHEL:**
```sh
sudo dnf install -y gtk4-devel gdk-pixbuf2-devel graphene-devel pkg-config gcc
```

**NixOS:**
```sh
nix-env -iA nixpkgs.gtk4 nixpkgs.gdk-pixbuf nixpkgs.graphene nixpkgs.glib.dev nixpkgs.pkg-config nixpkgs.gcc
```

#### 2. Install Rust (if not already installed)

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### 3. Build and install the application

**Using cargo-make (recommended):**
```sh
cargo install cargo-make
cargo make install
```

**Or manually:**

```sh
# Build the release binary
cargo build --release

# Install the binary
sudo install -Dm755 target/release/fin /usr/local/bin/fin

# Install configuration and assets
sudo install -Dm644 assets/config.toml /usr/share/fin/config.toml
sudo install -Dm644 assets/style.css /usr/share/fin/style.css
sudo install -Dm644 assets/default.toml /usr/share/fin/themes/default.toml
sudo install -Dm644 assets/fin.desktop /usr/share/applications/fin.desktop

# (Optional) Install Nerd Font for icon support
sudo install -Dm644 assets/MononokiNerdFontMono-Regular.ttf /usr/share/fonts/nerdfonts/MononokiNerdFontMono-Regular.ttf
sudo fc-cache -fv
```

#### Additional Notes

Ensure you have the necessary permissions to install packages on your system.
If you encounter any issues during installation, refer to the package manager's documentation for troubleshooting steps.

