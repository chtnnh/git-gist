#!/usr/bin/env bash
# Freeze website/docs/ into a Docusaurus release snapshot (idempotent).
#
# Usage:
#   ./scripts/docs-version.sh           # version from Cargo.toml
#   ./scripts/docs-version.sh 1.4.0     # explicit (v prefix optional)
#
# Git has no pre-tag hook. Run this *before* creating vX.Y.Z, commit the
# snapshot, then tag. .githooks/pre-push rejects tag pushes without a freeze.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WEBSITE="$ROOT/website"
VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
  VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/Cargo.toml" | head -n1)"
fi
VERSION="${VERSION#v}"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-].*)?$ ]]; then
  echo "error: invalid version '$VERSION' (expected SemVer like 1.3.0)" >&2
  exit 1
fi

cd "$WEBSITE"

if [[ ! -d node_modules ]]; then
  npm ci
fi

already=false
if [[ -f versions.json ]] && command -v jq >/dev/null 2>&1; then
  if jq -e --arg v "$VERSION" 'index($v) != null' versions.json >/dev/null; then
    already=true
  fi
elif [[ -d "versioned_docs/version-$VERSION" ]]; then
  already=true
fi

if [[ "$already" == true ]]; then
  echo "docs version $VERSION already frozen"
  exit 0
fi

npm run docusaurus -- docs:version "$VERSION"

echo "Frozen docs version $VERSION (default site = first entry in versions.json)."
echo "Next: commit website/versioned_* + versions.json, then:"
echo "  git tag -a v$VERSION -m \"v$VERSION\""
