#!/bin/sh

cargo check --workspace --no-default-features && \
cargo check --workspace --examples --all-features && \
cargo test --lib --workspace --no-default-features && \
cargo test --lib --workspace --all-features && \
cargo +nightly test --doc --workspace --all-features && \
cargo clippy --workspace --examples --all-features -- --deny warnings && \
cargo fmt --all --check && \
typos # install: cargo install typos-cli
