#!/usr/bin/env bash
# Install gg from this source tree into ~/.cargo/bin (or CARGO_HOME/bin)
set -euo pipefail
cd "$(dirname "$0")/.."
cargo install --path . --locked --force
echo "Installed: $(command -v gg || echo 'gg not on PATH — add ~/.cargo/bin')"
command 'gg' version
case "${SHELL##*/}" in
  bash|zsh|fish) command 'gg' doctor --shell "${SHELL##*/}" ;;
  *) echo "Before shell setup, run: command 'gg' doctor --shell <bash|zsh|fish>" ;;
esac
