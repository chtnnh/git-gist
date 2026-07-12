#!/usr/bin/env bash
# Install gg from this source tree into ~/.cargo/bin (or CARGO_HOME/bin)
set -euo pipefail
cd "$(dirname "$0")/.."
cargo install --path . --locked --force
echo "Installed: $(command -v gg || echo 'gg not on PATH — add ~/.cargo/bin')"
gg version
