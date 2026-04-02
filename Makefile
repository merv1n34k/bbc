.PHONY: setup dev build test test-all lint fmt clean

setup:
	cargo build

dev:
	cargo run -- --interactive

build:
	cargo build --release

test:
	cargo test --lib

test-all:
	cargo test

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

clean:
	cargo clean
