.PHONY: all build test clean install release check fmt lint help

# Default target
all: build

# Build the project
build:
	cargo build

# Build release version
release:
	cargo build --release

# Run tests
test:
	cargo test

# Run all checks (tests, lints, formatting)
check: test lint fmt-check

# Run clippy lints
lint:
	cargo clippy -- -D warnings

# Format code
fmt:
	cargo fmt

# Check formatting
fmt-check:
	cargo fmt -- --check

# Clean build artifacts
clean:
	cargo clean

# Run the server (for development)
run:
	cargo run

# Update dependencies
update:
	cargo update

