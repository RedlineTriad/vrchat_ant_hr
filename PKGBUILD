pkgname=vrchat-ant-hr
pkgver=0.1.0
pkgrel=1
pkgdesc="Bridges ANT+ heart-rate sensors to VRChat using OSC"
arch=('x86_64')
url="https://github.com/RedlineTriad/vrchat_ant_hr"
license=('MIT')
depends=('libusb')
makedepends=('cargo' 'pkgconf')

prepare() {
  cd "$startdir"
  export RUSTUP_TOOLCHAIN=stable
  cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}

build() {
  cd "$startdir"
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo build --release --all-features
}

check() {
  cd "$startdir"
  export RUSTUP_TOOLCHAIN=stable
  cargo test --all-features
}

package() {
  cd "$startdir"
  install -Dm755 "target/release/vrchat_ant_hr" "$pkgdir/usr/bin/vrchat-ant-hr"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
