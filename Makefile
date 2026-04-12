.PHONY: check check-storage check-cli release clean

# Run all checks (both crates, parallel-safe)
check: check-storage check-cli

check-storage:
	cd kerrigan-storage && make check

check-cli:
	cd kerrigan-cli && make check

# Release: generate changelog via kimi, tag, push
# Usage: make release VERSION=0.2.0
release:
	@test -n "$(VERSION)" || (echo "Usage: make release VERSION=x.y.z" && exit 1)
	./scripts/release.sh $(VERSION)

# Clean both crates
clean:
	cd kerrigan-storage && make clean
	cd kerrigan-cli && make clean
