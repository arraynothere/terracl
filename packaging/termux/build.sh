TERMUX_PKG_HOMEPAGE=https://github.com/arraynothere/terracl
TERMUX_PKG_DESCRIPTION="Lightweight terminal code editor with file tree and multi-encoding support"
TERMUX_PKG_LICENSE="MIT"
TERMUX_PKG_MAINTAINER="@arraynothere"
TERMUX_PKG_VERSION="0.1.0"
TERMUX_PKG_SRCURL=https://github.com/arraynothere/terracl/archive/refs/tags/v${TERMUX_PKG_VERSION}.tar.gz
TERMUX_PKG_SHA256=REPLACE_WITH_SHA256_OF_SOURCE_TARBALL
TERMUX_PKG_AUTO_UPDATE=true
TERMUX_PKG_BUILD_IN_SRC=true

termux_step_make() {
	termux_setup_rust

	cargo build \
		--jobs $TERMUX_PKG_MAKE_PROCESSES \
		--target $CARGO_TARGET_NAME \
		--release
}

termux_step_make_install() {
	install -Dm755 -t $TERMUX_PREFIX/bin target/${CARGO_TARGET_NAME}/release/terracl
}
