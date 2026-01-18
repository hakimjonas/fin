{
  description = "A flake for the Finë Application";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "fin";
          version = "0.2.22";
          src = ./.;

          # Use Cargo.lock directly instead of manual hash management
          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          buildInputs = [ pkgs.gtk4 ];

          nativeBuildInputs = [
            pkgs.pkg-config
            # Required for GTK4 apps: wraps binary to set GSettings schemas,
            # GIO modules, and other GTK environment variables
            pkgs.wrapGAppsHook4
          ];

          postInstall = ''
            install -Dm644 assets/config.toml $out/share/fin/config.toml
            install -Dm644 assets/style.css $out/share/fin/style.css
            install -Dm644 assets/default.toml $out/share/fin/themes/default.toml
            install -Dm644 assets/fin.desktop $out/share/applications/fin.desktop
          '';

          meta = with pkgs.lib; {
            description = "Finë: a simple GTK4-based session controller for Linux desktops";
            homepage = "https://github.com/hakimjonas/fin";
            license = licenses.mit;
            maintainers = [ maintainers.hakimjonas ];
            platforms = platforms.linux;
          };
        };

        # Development shell with all build dependencies
        devShells.default = pkgs.mkShell {
          name = "fin-dev";

          buildInputs = [
            pkgs.gtk4
          ];

          nativeBuildInputs = [
            # Rust toolchain
            pkgs.rustc
            pkgs.cargo
            pkgs.clippy
            pkgs.rustfmt

            # Build dependencies
            pkgs.pkg-config

            # GTK4 introspection and development
            pkgs.wrapGAppsHook4
          ];

          # Ensure GTK can find schemas during development
          shellHook = ''
            echo "Finë development environment"
            echo "Rust: $(rustc --version)"
            echo "Cargo: $(cargo --version)"
          '';
        };
      });
}