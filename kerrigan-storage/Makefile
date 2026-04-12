.PHONY: build test lint fmt check clean

# Build the project
build:
	cargo build

# Build release binary
release:
	cargo build --release

# Run all tests
test:
	cargo test

# Run clippy lints
lint:
	cargo clippy -- -D warnings

# Check formatting
fmt:
	cargo fmt -- --check

# Format code in place
fmt-fix:
	cargo fmt

# Full CI check: fmt + lint + test
check: fmt lint test

# Clean build artifacts
clean:
	cargo clean
