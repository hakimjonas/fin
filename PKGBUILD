# Maintainer: Your Name <you@example.com>
pkgname=fin
pkgver=0.1.0
pkgrel=1
pkgdesc="Finë: a simple GTK4-based session controller for Linux desktops"
arch=('x86_64')
url="https://github.com/yourusername/fin"
license=('MIT')
depends=('gtk4')
makedepends=('cargo' 'rust' 'git')
source=("$pkgname-$pkgver.tar.gz::https://github.com/yourusername/fin/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')  # Use a proper sha256sum in production

build() {
  cd "$srcdir/fin-$pkgver"
  cargo build --release --locked
}

package() {
  cd "$srcdir/fin-$pkgver"
  install -Dm755 "target/release/fin" "$pkgdir/usr/bin/fin"
  install -Dm644 "assets/config.toml" "$pkgdir/usr/share/fin/config.toml"
  install -Dm644 "assets/style.css" "$pkgdir/usr/share/fin/style.css"
}
