.PHONY: check release clean install viz-build build

VIZ_DIR = viz

# Build the frontend (requires Node.js 18+)
viz-build:
	cd $(VIZ_DIR) && npm install --no-audit --no-fund && npm run build

# Build everything: frontend + Rust binary
build: viz-build
	cargo build --release

# Run all checks
check:
	cargo fmt --check
	cargo clippy -- -D warnings
	cargo test

# Install to ~/.local/bin (builds frontend first, copies viz/out to ~/.kerrigan/viz/)
install: viz-build
	cargo install --path . && cp ~/.cargo/bin/kerrigan ~/.local/bin/kerrigan
	mkdir -p ~/.kerrigan/viz
	cp -r $(VIZ_DIR)/out/. ~/.kerrigan/viz/

# Release: auto-detect version bump from commits, generate changelog, tag, push
release:
	./scripts/release.sh $(VERSION)

# Check CI status
ci:
	./scripts/check-ci.sh

# Clean all build artifacts
clean:
	cargo clean
	rm -rf $(VIZ_DIR)/.next $(VIZ_DIR)/out $(VIZ_DIR)/node_modules
