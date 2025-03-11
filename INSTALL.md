# Installation

## Installation via Packages

**⚠️ Warning! Prebuild packages are still untested and may not work as expected. Please report any issues you encounter.
**

You can find pre-built packages here for various distributions:
[Finë Releases](https://github.com/hakimjonas/fin/releases).

### Available Packages

- `fin-0.2.5-arch.tar.gz` (811 KB) - 2025-02-11T23:22:55Z
- `fin-0.2.5-nix.tar.gz` (811 KB) - 2025-02-11T23:22:56Z
- `fin-0.2.5-solus.tar.gz` (811 KB) - 2025-02-11T23:22:55Z
- `fin_0.2.5_amd64.deb` (632 KB) - 2025-02-11T23:22:54Z
- Source code (zip) - 2025-02-11T23:13:13Z
- Source code (tar.gz) - 2025-02-11T23:13:13Z

### Solus (tarball)

To install the Solus tarball, use the following command:

```sh
sudo eopkg it fin-0.2.3-solus.tar.gz
```

### Arch Linux (tarball or PKGBUILD)

To install the Arch tarball, use the following command:

```sh
sudo pacman -U fin-0.2.3-arch.tar.gz```

Alternatively, you can install the package from the AUR using an AUR helper like `yay`:

```sh
yay -S fin```

### Debian-based Distributions (.deb)

To install the `.deb` package on Ubuntu or other Debian-based distributions, use the following command:

```sh
sudo dpkg -i fin_0.2.3_amd64.deb```

If there are any missing dependencies, you can resolve them with:

```sh
sudo apt-get install -f
```

### Manual Installation

If you prefer to manually install the application, follow these steps:

#### Build and install the application:

```sh
cargo make install
```

#### Copy assets:

```sh
sudo mkdir -p /usr/share/fin
sudo cp assets/config.toml /usr/share/fin/config.toml
sudo cp assets/style.css /usr/share/fin/style.css
sudo mkdir -p /usr/share/fonts/nerdfonts
sudo cp assets/fonts/MononokiNerdFont-Regular.ttf /usr/share/fonts/nerdfonts/
sudo fc-cache -fv

#### Install the binary:

```sh

sudo cp target/release/fin /usr/local/bin/
sudo chmod 755 /usr/local/bin/fin
```

#### Additional Notes

Ensure you have the necessary permissions to install packages on your system.
If you encounter any issues during installation, refer to the package manager's documentation for troubleshooting steps.

