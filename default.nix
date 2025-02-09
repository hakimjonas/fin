{ stdenv, rustPlatform, fetchFromGitHub, gtk4, glib }:

rustPlatform.buildRustPackage rec {
  pname = "fin";
  version = "0.1.0";

  src = fetchFromGitHub {
    owner = "hakimjonas";
    repo = "fin";
    rev = "v${version}";
    sha256 = "sha256-placeholder";  # TODO: Run 'nix-prefetch-git' to get the correct hash
  };

  # cargoSha256 is computed during the build; update it after the first build.
  cargoSha256 = "cargo-sha256-placeholder";

  nativeBuildInputs = [ gtk4 glib ];

  meta = {
    description = "Finë: a simple GTK4-based session controller for Linux desktops";
    license = stdenv.lib.licenses.mit;
    platforms = stdenv.lib.platforms.linux;
    homepage = "https://github.com/hakimjonas/fin";
  };
}
