default:
	cargo build

install:
	cargo build --release --bin counter-tui
	sudo install target/release/counter-tui /usr/local/bin

uninstall:
	sudo rm /usr/local/bin/tui-counter
