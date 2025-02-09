# Finë

Finë is a simple, GTK4-based session controller designed for Linux desktops. The aim is simple: to offer an unassuming yet effective means of managing session transitions like logging out, locking, rebooting, or shutting down—while embracing clarity and thoughtful design. It started as a personal project for my own use in HyprLand, but it should work most Linux desktop environments, given the required dependencies. I know my choice of immutable data structures are perhaps not the most efficient, but I wanted to experiment with them in a real-world application. And for such a simple application, the performance impact is negligible.

## Name Origin and Pronunciation

**Finë** is a word in Quenya. It means "end" or "ending." The pronunciation is similar to the English word "fin" but with a long "e" sound at the end. The diaeresis (ë) indicates that the "e" is pronounced separately from the preceding vowel. So, it's pronounced "fin-eh."

However, in the brutal realm of command-line interfaces—where a rogue umlaut can cause chaos rivaling an angry troll—our enchanting project, officially called Finë, must humble itself. For everyday use, the executable is simply named `fin` in ASCII. Think of it as trading a lavish farewell in an enchanted forest for a polite nod at the bus stop—simple, reliable, and free from typographical misadventures.

## Features

**Finë** is designed to be simple and effective—but we know there’s always room to grow. In the spirit of open-source collaboration, we welcome ideas and contributions from the community. Here are some potential areas where your contributions could make a difference:

- **Extensible Plugin System**:
  Imagine adding the ability for users to extend Finë with custom command sets or actions. If you have ideas for a plugin API or want to contribute an initial set of plugins, your help would be most welcome.

- **Enhanced Theming and Customization**:
  While Finë’s interface is intentionally minimal, there’s potential to offer customizable themes, icons, and even subtle animations. Contributions in this area could let users personalize the look and feel to match their desktop environment.

- **Advanced Session Management**:
  Future work could include features that save and restore user sessions, offering greater control over how applications are closed or restarted. If you’re interested in tackling state management challenges, this is an exciting area to explore.

- **Broader Configuration Support**:
  Currently, configuration is done via TOML. You might help by adding support for other popular configuration formats (like JSON or YAML), thereby giving users more flexibility.

- **Deeper Desktop Environment Integration**:
  Further integration with different desktop environments could provide environment-specific enhancements (such as tailored command sets or appearance tweaks) without sacrificing the core simplicity. Contributions that help detect and adapt to various environments would be very valuable.

## Dependencies

To build and run Finë, you need to have the following dependencies installed:

- **Rust**: Version 1.56 or later. You can install Rust using [rustup](https://rustup.rs/).
- **GTK4**: Version 4.9 or later (recommended for best results).

### Solus

```sh
sudo eopkg it libgtk-4-devel 
## Installation
 ```

### Nix / NixOS

```sh

nix-env -iA nixpkgs.gtk4

```

### Arch

sudo pacman -S gtk4

```sh


### Fedora / Red Hat / CentOS

```sh
sudo dnf install gtk4-devel
```

### Debian and derivatives (Ubuntu, Mint, etc.)

```sh

sudo apt install libgtk-4-dev

``` 

To install Finë, you can clone the repository and build the application using Cargo. The provided Makefile will
automatically install cargo-make if it is not already installed.

```bash
git clone
cd fin
cargo make install
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
```

#### Install the binary:

```sh

sudo cp target/release/fin /usr/local/bin/
sudo chmod 755 /usr/local/bin/fin
```

## Usage

To run the application, simply execute:

```bash
fin
```

## Configuration

The application reads its configuration from a TOML file located at `/usr/share/fin/config.toml`. You can
customize
this file to suit your needs.

## License

This project is licensed under the MIT License.
