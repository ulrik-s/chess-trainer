.PHONY: setup fmt fmt-check lint typos doc test coverage arch deny machete mutants wasm serve check

# Lokal uppsättning: git-hooks och de verktyg som inte följer med rustup.
setup:
	git config core.hooksPath .githooks
	rustup toolchain install
	cargo install --locked cargo-llvm-cov cargo-mutants cargo-deny cargo-machete typos-cli taplo-cli committed trunk

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

# Webbcratet är undantaget: det är DOM-lim som bara går att köra i en webbläsare.
coverage:
	cargo llvm-cov --workspace --exclude chess-trainer-web --all-features --locked --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100

arch:
	sh scripts/arch.sh

deny:
	cargo deny check

machete:
	cargo machete

mutants:
	cargo mutants --workspace --no-shuffle -- --locked

# Bygger webbappen till crates/web/dist.
wasm:
	cd crates/web && trunk build --release --locked

# Startar webbappen lokalt på http://127.0.0.1:8080.
serve:
	cd crates/web && trunk serve

# Samma kontroller som CI, i samma ordning.
check: fmt-check lint typos doc test coverage arch deny machete mutants wasm
