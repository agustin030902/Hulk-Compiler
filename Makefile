.PHONY: build clean

build:
	cargo build --release
	cp target/release/Hulk-Compiler ./hulk
	chmod +x ./hulk

clean:
	cargo clean
	rm -f hulk output