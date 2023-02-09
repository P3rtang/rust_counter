default:
	cargo +stable build

install --linux:
	cargo build --release --bin counter-tui
	sudo install target/release/counter-tui /usr/local/bin
	mkdir -p ~/.local/share/counter-tui/

install --windows:
	cargo build --release --bin counter-tui
	mv target/release/counter-tui .
	mkdir data

uninstall:
	sudo rm /usr/local/bin/tui-counter
	rm -r ~/.local/share/counter-tui/

tests:
	cargo +stable test
