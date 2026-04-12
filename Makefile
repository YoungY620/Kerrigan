.PHONY: check check-storage check-cli release clean

# Run all checks (both crates, parallel-safe)
check: check-storage check-cli

check-storage:
	cd kerrigan-storage && make check

check-cli:
	cd kerrigan-cli && make check

# Release: auto-detect version bump from commits, generate changelog, tag, push
# Usage: make release              (auto version)
#        make release VERSION=1.0.0 (manual override)
release:
	./scripts/release.sh $(VERSION)

# Clean both crates
clean:
	cd kerrigan-storage && make clean
	cd kerrigan-cli && make clean
