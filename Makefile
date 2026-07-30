all:
	cargo build

preview:
	cd doc && quarto preview

doc:
	cd doc && quarto render

www:
	open http://localhost:8080/cgi-bin/contelia.hsl
	busybox httpd -f -p8080 -h www/

cross:
	PKG_CONFIG_SYSROOT_DIR=/usr/aarch64-linux-gnu \
	RUSTFLAGS="-C linker=aarch64-linux-gnu-gcc" \
	cargo build -r --target aarch64-unknown-linux-gnu

.PHONY: www doc