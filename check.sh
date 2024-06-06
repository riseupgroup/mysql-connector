#!/bin/sh

cargo check --workspace --no-default-features
cargo check --workspace --all-features
cargo test --workspace --lib --no-default-features
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- --deny warnings
cargo fmt --all --check
typos # install: cargo install typos-cli
