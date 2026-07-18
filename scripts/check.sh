#!/usr/bin/env bash
# The CI gauntlet: formatting, lints (deny warnings), compile-checked SQL
# metadata freshness, and the full test suite.
#
# Prerequisite: the dev database is up and seeded —
#     docker compose up -d && cargo run --bin seed
# (integration tests and `sqlx prepare --check` talk to it).
set -euo pipefail
cd "$(dirname "$0")/.."

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo sqlx prepare --check --workspace
cargo test --workspace

echo "ALL CHECKS PASSED"
