include env

all: deb win

deb: source
	cargo deb --target aarch64-unknown-linux-gnu

win: source
	cargo build --target x86_64-pc-windows-gnu --bin client --release

clean:
	-rm -r target

source: src/bin/daemon.rs src/bin/client.rs


.PHONY: all clean deb win source
