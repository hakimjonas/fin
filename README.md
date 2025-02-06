# HyprPower Logout Manager

HyprPower is a simple, GTK4-based logout manager designed for Linux desktops. The aim is simple: to offer an unassuming
yet effective means of managing session transitions like logging out, locking, rebooting, or shutting down—while
embracing clarity and thoughtful design. It started as a personal project for my own use in HyprLand, but it should work
most Linux desktop environments, given the required dependencies.

## Features

- **GTK4 Interface**: The application uses GTK4 to present a clean and accessible interface. Each button is intended to
  perform a clear and deliberate function.
- **Configuration Driven**: Settings are read from a TOML file, allowing for customization to suit different desktop
  environments without imposing unnecessary complexity.
- **Functional Programming Principles**: The design separates pure functions from side effects, reflecting a commitment
  to clarity and a measured approach to software design.
- **Immutable Data Structures**: Using immutable collections (via the `im` crate), the application underscores a respect
  for stability and consistency in its operation.
- **Keyboard Navigation**: Navigation through the interface is facilitated by arrow keys and the Tab key, ensuring that
  the experience is both intuitive and inclusive.
- **Accessibility**: Leveraging GTK’s built-in accessible properties, along with thoughtful tooltips, HyprPower aims to
  serve every user with modest care.

## Dependencies

- **Rust**: Version 1.56 or later.
- **GTK4**: Version 4.9 or later (recommended for best results).
- **Cargo Make**: Optional, useful for streamlining the build and installation process.

## Build and Installation

### Building

To compile the application in release mode, use:

```sh
cargo build --release