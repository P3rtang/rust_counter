default:
	cargo build

install:
	cargo build --release --bin tui-counter
	sudo install target/release/tui-counter /usr/local/bin

uninstall:
	sudo rm /usr/local/bin/tui-counter
