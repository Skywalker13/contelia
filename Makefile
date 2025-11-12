all:
	cargo build

cross:
	PKG_CONFIG_SYSROOT_DIR=/usr/aarch64-linux-gnu RUSTFLAGS="-C linker=aarch64-linux-gnu-gcc" cargo build -r --target aarch64-unknown-linux-gnu