.PHONY: check release clean install

# Run all checks
check:
	cargo fmt --check
	cargo clippy -- -D warnings
	cargo test

# Install to ~/.local/bin
install:
	cargo install --path . && cp ~/.cargo/bin/kerrigan ~/.local/bin/kerrigan

# Release: auto-detect version bump from commits, generate changelog, tag, push
release:
	./scripts/release.sh $(VERSION)

# Check CI status
ci:
	./scripts/check-ci.sh

# Clean build artifacts
clean:
	cargo clean
