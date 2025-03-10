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
  version = "0.2.5";
          src = ./.;
          cargoSha256 = "03ik4z1c7kf7ml0gb5as21wdmcvxbcg82migk0i4sssx9wrj2nvf";
          buildInputs = [ pkgs.gtk4 ];
          meta = with pkgs.lib; {
            description = "Finë Application";
            license = licenses.mit;
            maintainers = [ maintainers.hakimjonas ];
            platforms = platforms.linux;
          };
        };
      });
}