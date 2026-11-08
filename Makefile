.PHONY: setup fmt fmt-check lint typos doc test coverage arch deny machete mutants check

# Lokal uppsättning: git-hooks och de verktyg som inte följer med rustup.
setup:
	git config core.hooksPath .githooks
	rustup toolchain install
	cargo install --locked cargo-llvm-cov cargo-mutants cargo-deny cargo-machete typos-cli taplo-cli committed

fmt:
	cargo fmt --all
	taplo fmt

fmt-check:
	cargo fmt --all --check
	taplo fmt --check

lint:
	cargo clippy --workspace --all-targets --all-features --locked

typos:
	typos

doc:
	cargo doc --workspace --no-deps --document-private-items --locked

test:
	cargo test --workspace --all-features --locked

coverage:
	cargo llvm-cov --workspace --all-features --locked --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100

arch:
	sh scripts/arch.sh

deny:
	cargo deny check

machete:
	cargo machete

mutants:
	cargo mutants --workspace --no-shuffle -- --locked

# Samma kontroller som CI, i samma ordning.
check: fmt-check lint typos doc test coverage arch deny machete mutants
