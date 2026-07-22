# Packaging & distribution

`gg` ships through multiple channels. End-user install commands live in the [Install](./install.md) chapter; this page is the operator guide.

## Channels

### crates.io

```bash
cargo publish
# users: cargo install git-gist --locked
```

### GitHub Releases (cargo-dist)

Push a tag `vX.Y.Z`. The generated [Release](../../.github/workflows/release.yml) workflow (from `dist generate`) uploads archives, checksums, `git-gist-installer.sh` / `.ps1`, and a Homebrew formula. Config lives in [`dist-workspace.toml`](../../dist-workspace.toml).

### Homebrew

Tap: [`chtnnh/homebrew-tap`](https://github.com/chtnnh/homebrew-tap). cargo-dist publishes on each tag when `HOMEBREW_TAP_TOKEN` is set.

```bash
brew tap chtnnh/tap
brew install git-gist
brew update && brew upgrade git-gist
```

After upgrading, if `gg version` is stale, check `which -a gg` — a Cargo install under `~/.cargo/bin` often shadows Homebrew. See [Install](./install.md).

### Debian / RPM

Attached by [`.github/workflows/packages.yml`](../../.github/workflows/packages.yml) after a Release is published. Locally: `cargo deb` / `cargo generate-rpm` (metadata in `Cargo.toml`).

### Nix

[`flake.nix`](../../flake.nix):

```bash
nix run github:chtnnh/git-gist -- version
nix profile install github:chtnnh/git-gist
```

Optional later: nixpkgs / NUR PRs.

## Release checklist

1. `cargo test --workspace` and coverage gate (≥95%)
2. Bump version in `Cargo.toml` + `CHANGELOG.md` + `flake.nix`
3. Tag `vX.Y.Z` and push
4. Verify Release assets (archives + installers)
5. Confirm Homebrew tap formula updated
6. `cargo publish`
7. Smoke-test: `brew upgrade git-gist`, `which -a gg`, installer script, `nix run`, `gg version`
