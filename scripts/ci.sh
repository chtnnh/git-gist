#!/usr/bin/env bash
# Mirror GitHub Actions CI locally before commit/push.
set -euo pipefail
cd "$(dirname "$0")/.."

export RUSTFLAGS="${RUSTFLAGS:--D warnings}"

echo "==> cargo fmt --check"
cargo fmt --check

echo "==> cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets -- -D warnings

echo "==> cargo test --workspace"
cargo test --workspace

echo "==> coverage (≥95%)"
./scripts/coverage.sh

echo "PASS: local CI gate"
