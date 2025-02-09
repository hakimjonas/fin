{ stdenv, rustPlatform, fetchFromGitHub, gtk4, glib }:

rustPlatform.buildRustPackage rec {
  pname = "fin";
  version = "0.1.0";

  src = fetchFromGitHub {
    owner = "yourusername";
    repo = "fin";
    rev = "v${version}";
    sha256 = "sha256-placeholder"; # Replace with the actual hash (use nix-prefetch-git)
  };

  cargoSha256 = "cargo-sha256-placeholder"; # Replace this with the hash computed by Nix

  nativeBuildInputs = [ gtk4 glib ];

  meta = {
    description = "Finë: a simple GTK4-based session controller for Linux desktops";
    license = stdenv.lib.licenses.mit;
    platforms = stdenv.lib.platforms.linux;
    homepage = "https://github.com/yourusername/fin";
  };
}
