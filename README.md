# Finë

Finë is a simple, GTK4-based session controller designed for Linux desktops. The aim is simple: to offer an unassuming
yet effective means of managing session transitions like logging out, locking, rebooting, or shutting down—while
embracing clarity and thoughtful design. It started as a personal project for my own use in HyprLand, but it should work
most Linux desktop environments, given the required dependencies. I know my choice of immutable data structures are
perhaps not the most efficient, but I wanted to experiment with them in a real-world application. And for such a simple
application, the performance impact is negligible.

## Name Origin and Pronunciation

**Finë** is a word in Quenya. It means "end" or "ending." The pronunciation is similar to the English word "fin" but
with a long "e" sound at the end. The diaeresis (ë) indicates that the "e" is pronounced separately from the preceding
vowel. So, it's pronounced "fin-eh."

However, in the brutal realm of command-line interfaces—where a rogue umlaut can cause chaos rivaling an angry troll—our
enchanting project, officially called Finë, must humble itself. For everyday use, the executable is simply named `fin`
in ASCII. Think of it as trading a lavish farewell in an enchanted forest for a polite nod at the bus stop—simple,
reliable, and free from typographical misadventures.

## Features

**Finë** is designed to be simple and effective—but we know there’s always room to grow. In the spirit of open-source
collaboration, we welcome ideas and contributions from the community. Here are some potential areas where your
contributions could make a difference:

- **Enhanced Theming and Customization**:
  While Finë’s interface is intentionally minimal, there’s potential to offer customizable themes, icons, and even
  subtle animations. Contributions in this area could let users personalize the look and feel to match their desktop
  environment.

- **Broader Configuration Support**:
  Currently, configuration is done via TOML. You might help by adding support for other popular configuration formats (
  like JSON or YAML), thereby giving users more flexibility.

## Dependencies

To build and run Finë, you need to have the following dependencies installed:

- **Rust**: Version 1.56 or later. You can install Rust using [rustup](https://rustup.rs/).
- **GTK4**: Version 4.9 or later (recommended for best results).

## Installation

For detailed installation instructions, please refer to the [INSTALL.md](INSTALL.md) file.

## Usage

To run the application, simply run:

```bash
fin
```

If you installed manually, make sure to build the project first:

```bash
cargo build --release
```

## Configuration

The application reads its configuration from a TOML file located at `/usr/share/fin/config.toml`. You can customize this
file to suit your needs.

For more details on configuration, refer to the [CONFIGURATION.md](CONFIGURATION.md) file.

Make sure to save your changes and restart the application for the new configuration to take effect.

## Reporting Issues

If you find a bug or have a feature request, please open an issue on GitHub. Provide as much detail as possible to help
us understand and address the issue.

## License

This project is licensed under the MIT License.
