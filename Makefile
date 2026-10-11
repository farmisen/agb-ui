.PHONY: check test clippy clippy-host fmt fmt-check help

# The GBA target. No .cargo/config.toml: its build-std would clash with the host's std
# in `make test`, so the GBA build passes it here instead.
GBA := --target thumbv4t-none-eabi -Zbuild-std=core,alloc

check: fmt-check test clippy clippy-host  ## Everything CI runs

test:             ## Unit tests, on the host (without agb, which only builds for the GBA)
	cargo test --locked --no-default-features

clippy:           ## Lint for the GBA, with agb
	cargo clippy --locked $(GBA) -- -D warnings

clippy-host:      ## Lint the host build and the tests
	cargo clippy --locked --all-targets --no-default-features -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

help:
	@grep -E '^[a-z-]+:.*##' $(MAKEFILE_LIST) | sed 's/:.*##/\t/'
