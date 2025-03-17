# Maintainer: Hakim Jonas Ghoula <hakim@walkthisway.dk>
pkgname=fin
pkgver=0.2.14
pkgrel=1
pkgdesc="Finë: a simple GTK4-based session controller for Linux desktops"
arch=('x86_64')
url="https://github.com/hakimjonas/fin"
license=('MIT')
depends=('gtk4')
makedepends=('cargo' 'rust' 'git')
# Download the source tarball from GitHub.
source=("$pkgname-$pkgver.tar.gz::https://github.com/hakimjonas/fin/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('a3c0fea40ec3b3e6cfddd91539cf6183bed3b4ea5da24050b01b3f160682ede4')  # TODO: Replace with a proper sha256 sum computed via 'sha256sum'

build() {
  cd "$srcdir/fin-$pkgver"  # Ensure that the extracted directory is named as expected.
  # Use --locked if you want to enforce that Cargo.lock is up-to-date.
  cargo build --release --locked
}

package() {
  cd "$srcdir/fin-$pkgver"
  # Install the binary.
  install -Dm755 "target/release/fin" "$pkgdir/usr/bin/fin"
  # Install configuration and assets.
  install -Dm644 "assets/config.toml" "$pkgdir/usr/share/fin/config.toml"
  install -Dm644 "assets/style.css" "$pkgdir/usr/share/fin/style.css"
}
