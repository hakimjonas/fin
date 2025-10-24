## [0.2.20] - 2025-10-24

## [0.2.19] - 2025-10-24

## [0.2.18] - 2025-10-24

## [0.2.17] - 2025-10-23

## [0.2.16] - 2025-10-22

### Fixed
- **Build Dependencies**: Added explicit build dependencies for gdk-pixbuf and graphene
  - Updated Makefile.toml with complete dependency installation for all supported distributions
  - Updated INSTALL.md with comprehensive build dependency documentation
  - Fixed missing pkg-config files (gdk-pixbuf-2.0.pc, graphene-gobject-1.0.pc) that caused build failures on fresh systems
  - Debian/Ubuntu: Now explicitly installs `libgdk-pixbuf-2.0-dev` and `libgraphene-1.0-dev`
  - Arch Linux: Now explicitly installs `gdk-pixbuf2` and `graphene`
  - Solus: Now explicitly installs `gdk-pixbuf-devel` and `graphene-devel`
  - Fedora/RHEL: Now explicitly installs `gdk-pixbuf2-devel` and `graphene-devel`

## [0.2.15] - 2025-10-22

## [0.2.14] - 2025-03-17

## [0.2.13] - 2025-03-13

## [0.2.12] - 2025-03-13

## [0.2.11] - 2025-03-13

## [0.2.10] - 2025-03-13

## [0.2.9] - 2025-03-11
## [0.2.8] - 2025-03-11
## [0.2.7] - 2025-03-11
## [0.2.6] - 2025-03-11
## [0.2.5] - 2025-03-11
# Changelog

All notable changes to this project will be documented in this file.

## [0.1.10] - 2025-02-11

### Added

- Initial release of Finë.
- Basic session management features: logout, lock, reboot, shutdown.
- Build with GTK4 and Rust.
- Configuration via TOML file.
- Basic theming and customization options.