# Installation

## Installation via Packages

### Solus (tarball)

To install the Solus tarball, use the following command:

```sh
sudo eopkg it fin-0.1.0-1-1-x86_64.eopkg
```

### Arch Linux (tarball or PKGBUILD)

To install the Arch tarball, use the following command:

```sh
sudo pacman -U fin-0.1.0-1-x86_64.pkg.tar.zst
```

Alternatively, you can install the package from the AUR using an AUR helper like `yay`:

```sh
yay -S fin```

### Debian-based Distributions (.deb)

To install the `.deb` package on Ubuntu or other Debian-based distributions, use the following command:

```sh
sudo dpkg -i fin_0.1.0_amd64.deb
```

If there are any missing dependencies, you can resolve them with:

```sh
sudo apt-get install -f
```

### Red Hat-based Distributions (.rpm)

To install the .rpm package on Fedora or other Red Hat-based distributions, use the following command:

```sh
sudo rpm -i fin-0.1.0-1.x86_64.rpm
```

### Manual Installation

If you prefer to manually install the application, follow these steps:

#### Build the application:

```sh
cargo build --release
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

