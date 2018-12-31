all: run build

build:
	cargo build

run:
	env RUSTFLAGS=-Awarnings cargo run --release

test:
	env RUSTFLAGS=-Awarnings cargo test --release
