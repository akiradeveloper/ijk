install:
	cargo install --bin ijk --path .
uninstall:
	cargo uninstall --bin ijk
test:
	cargo test -- --test-threads=1
