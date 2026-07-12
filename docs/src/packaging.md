# Packaging & distribution

`gg` ships through multiple channels. Use this page as the operator guide; end-user install commands live in the [Install](./install.md) chapter.

## Channels

### crates.io

```bash
cargo publish
# users: cargo install git-gist --locked
```

### GitHub Releases

Push a tag `vX.Y.Z`. The Release workflow uploads archives and a man page. Users can grab binaries or the shell installer once cargo-dist is enabled.

### Homebrew

1. Create a tap repo: `chtnnh/homebrew-tap`
2. Copy [`packaging/homebrew/git-gist.rb`](../../packaging/homebrew/git-gist.rb) into `Formula/git-gist.rb`
3. Set `url` to the GitHub release source tarball and `sha256` from `shasum -a 256`
4. Users:

```bash
brew tap chtnnh/tap
brew install git-gist
```

Homebrew core submission is optional later (needs stable releases + formula review).

### Debian (`.deb`)

```bash
cargo install cargo-deb
cargo deb
# → target/debian/git-gist_*.deb
```

Metadata lives in `Cargo.toml` under `[package.metadata.deb]`. Attach the `.deb` to the GitHub Release or host an apt repo (advanced).

### RPM

```bash
cargo install cargo-generate-rpm
cargo build --release
cargo generate-rpm
# → target/generate-rpm/*.rpm
```

Metadata: `[package.metadata.generate-rpm]`.

### Nix

This repo includes [`flake.nix`](../../flake.nix):

```bash
nix run . -- version
nix profile install .
nix profile install github:chtnnh/git-gist   # after push
```

Optional follow-ups: nixpkgs package PR, NUR.

### Scoop / AUR / Winget (optional)

- **Scoop:** JSON manifest in a bucket pointing at Windows release zips
- **AUR:** `PKGBUILD` building from crates.io or git tag
- **Winget:** YAML manifest against GitHub release assets

## Release checklist

1. `cargo test --workspace` and `./scripts/coverage.sh` (≥95%)
2. Bump version in `Cargo.toml` + `CHANGELOG.md` + `flake.nix`
3. Tag `vX.Y.Z` and push
4. Verify Release assets
5. `cargo publish`
6. Update Homebrew tap sha256/url
7. Attach deb/rpm if not automated
8. Smoke-test: `brew install`, `nix run`, `gg version`
