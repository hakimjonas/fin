# Distribution Packaging Guide

This document provides comprehensive information for distribution maintainers who want to package Finë.

## Package Information

- **Name**: fin (executable), Finë (full name)
- **License**: MIT
- **Homepage**: https://github.com/hakimjonas/fin
- **Description**: Simple GTK4-based session controller for Linux desktops

## Build Dependencies

### Required
- Rust toolchain (1.56+, stable channel recommended)
- Cargo
- GTK4 development files (libgtk-4-dev or gtk4-devel)
- pkg-config / pkgconf

### Runtime Dependencies
- GTK4 (version 4.10+)

## Build Instructions

### Standard Cargo Build
```bash
cargo build --release
```

### Using cargo-make (recommended for packaging)
```bash
cargo install cargo-make
cargo make package-<distro>
```

Where `<distro>` is one of: `deb`, `arch`, `solus`, `nix`

## Installation Paths

Finë follows the Filesystem Hierarchy Standard (FHS):

| File | Path | Permissions |
|------|------|-------------|
| Binary | `/usr/bin/fin` or `/usr/local/bin/fin` | 755 |
| System config | `/usr/share/fin/config.toml` | 644 |
| System stylesheet | `/usr/share/fin/style.css` | 644 |
| Default theme | `/usr/share/fin/themes/default.toml` | 644 |
| Desktop file | `/usr/share/applications/fin.desktop` | 644 |
| Documentation | `/usr/share/doc/fin/README.md` | 644 |

### User Configuration
Users can override system configuration in:
- `$XDG_CONFIG_HOME/fin/config.toml` or
- `$HOME/.config/fin/config.toml`

## Distro-Specific Packaging

### Debian/Ubuntu (.deb)

**Using cargo-deb:**
```bash
cargo install cargo-deb --version 1.42.0
cargo deb
```

The package metadata is defined in `Cargo.toml` under `[package.metadata.deb]`.

**Package details:**
- Section: `utils`
- Priority: `optional`
- Depends: Automatically detected via `$auto`

**Files installed:**
All files listed in the table above are automatically included.

### Arch Linux

**PKGBUILD** is provided in the repository root.

```bash
makepkg -si
```

**Key points:**
- Uses `cargo build --release --locked` for reproducible builds
- SHA256 checksum must be updated for each release
- Includes all assets and .desktop file

**AUR Submission:**
The PKGBUILD is AUR-ready. Maintainers can submit to:
- `fin` (stable releases from tags)
- `fin-git` (latest from git)

### Solus

**Manifest:** `fin.sol` (XML format)

**Building:**
```bash
cargo make package-solus
```

Creates a tarball with the Solus manifest included.

**Integration:**
- Group: Utility
- Depends: gtk4
- Binary location: `/usr/local/bin/fin`

### NixOS

**Flake:** `flake.nix` is provided.

**Usage:**
```nix
{
  inputs.fin.url = "github:hakimjonas/fin";

  # In your configuration:
  environment.systemPackages = [ inputs.fin.packages.${system}.default ];
}
```

**Building locally:**
```bash
nix build
```

**Note:** `cargoSha256` must be updated when dependencies change.

## Desktop Integration

### .desktop File

Finë provides a freedesktop.org compatible `.desktop` file:

```ini
[Desktop Entry]
Name=Finë
GenericName=Session Controller
Comment=Simple GTK4-based session controller
Exec=fin
Icon=system-shutdown
Terminal=false
Type=Application
Categories=System;
Keywords=logout;shutdown;reboot;lock;session;power;
```

**Categories:** Appears in System menus

**Keywords:** Searchable by the listed terms

### Icon
Uses the standard `system-shutdown` icon from the system icon theme.

## Configuration

### Desktop Environment Auto-Detection

Finë automatically detects the desktop environment using:
1. `$XDG_CURRENT_DESKTOP`
2. `$DESKTOP_SESSION` (fallback)

Supported DEs:
- Hyprland
- Sway
- i3
- GNOME
- Budgie
- XFCE
- COSMIC
- Generic fallback

### Default Commands

Each desktop environment has pre-configured commands in `/usr/share/fin/config.toml`.

Example for GNOME:
```toml
[default_commands.gnome]
columns = 2
buttons = [
    { label = "󰌾", command = "dbus-send ... Lock", ... },
    { label = "󰍃", command = "gnome-session-quit --logout", ... },
    { label = "󰜉", command = "systemctl reboot", ... },
    { label = "󰐥", command = "systemctl poweroff", ... }
]
```

## Theming

### Default Theme
Based on Rose Pine Moon color scheme.

Located at: `/usr/share/fin/themes/default.toml`

### Custom Themes
Users can create themes in `~/.config/fin/themes/<name>.toml`

Theme structure:
- 16 palette colors (palette0-palette15)
- UI-specific colors (background, foreground)
- Button states (normal, hover, focus)

## Testing the Package

### Basic Functionality Test
```bash
# 1. Check version
fin --version

# 2. Check help
fin --help

# 3. Launch (requires GUI)
fin

# 4. Test keyboard navigation
# - Arrow keys to navigate
# - Enter to execute
# - Escape to quit
```

### Configuration Test
```bash
# Verify system config exists
test -f /usr/share/fin/config.toml && echo "✓ Config installed"

# Verify theme exists
test -f /usr/share/fin/themes/default.toml && echo "✓ Theme installed"

# Verify .desktop file
desktop-file-validate /usr/share/applications/fin.desktop && echo "✓ Desktop file valid"
```

## Packaging Checklist

- [ ] Update version in package metadata
- [ ] Verify all dependencies are listed
- [ ] Test build from source tarball
- [ ] Check binary works after installation
- [ ] Verify all files are installed to correct locations
- [ ] Validate .desktop file with `desktop-file-validate`
- [ ] Test with different desktop environments
- [ ] Check file permissions are correct
- [ ] Verify uninstall removes all files cleanly

## Troubleshooting

### Build Failures

**GTK4 not found:**
```
error: failed to run custom build command for `gtk4-sys`
```
**Solution:** Install GTK4 development packages.

**Rust version too old:**
```
error: package requires rustc 1.56 or newer
```
**Solution:** Update Rust toolchain.

### Runtime Issues

**Config not found:**
- Check `/usr/share/fin/config.toml` exists
- Verify read permissions (644)

**Theme not loading:**
- Check `/usr/share/fin/themes/default.toml` exists
- Check file permissions

## Maintainer Contact

For packaging questions or issues:
- GitHub Issues: https://github.com/hakimjonas/fin/issues
- Maintainer: Hakim Jonas Ghoula <hakim@walkthisway.dk>

## Upstream Releases

Finë uses semantic versioning (MAJOR.MINOR.PATCH).

- **Releases:** Tagged as `v<version>` on GitHub
- **Changelog:** See `CHANGELOG.md`
- **Release artifacts:** Signed tarballs available on GitHub Releases

### Automated Versioning

The project uses automated version bumping on the `trunk` branch. Each merge to trunk triggers a patch version bump.

### Tracking Upstream

Subscribe to releases:
```bash
gh repo subscribe hakimjonas/fin --releases-only
```

Or watch the RSS feed:
```
https://github.com/hakimjonas/fin/releases.atom
```

## Contributing Packaging Improvements

Packaging improvements are welcome! Please submit PRs with:
- Description of the change
- Testing performed
- Distro-specific considerations

Thank you for packaging Finë! 🎉
