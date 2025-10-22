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
          version = "0.2.14";
          src = ./.;
          cargoSha256 = "03ik4z1c7kf7ml0gb5as21wdmcvxbcg82migk0i4sssx9wrj2nvf";
          buildInputs = [ pkgs.gtk4 ];
          nativeBuildInputs = [ pkgs.pkg-config ];

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
      });
}