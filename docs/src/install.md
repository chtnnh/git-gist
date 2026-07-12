# Install

## Cargo (crates.io / git)

```bash
cargo install git-gist --locked
# or from this repo:
cargo install --git https://github.com/chtnnh/git-gist --locked
cargo install --path . --locked
```

## Homebrew

```bash
brew tap chtnnh/tap
brew install git-gist
# or: brew install chtnnh/tap/git-gist
```

Requires the `chtnnh/homebrew-tap` repository to publish the formula (see [Packaging](./packaging.md)).

## Nix

```bash
nix run github:chtnnh/git-gist -- version
nix profile install github:chtnnh/git-gist
# from a local checkout:
nix run . -- overview
```

## deb / rpm

Download from [GitHub Releases](https://github.com/chtnnh/git-gist/releases), or build locally:

```bash
cargo install cargo-deb cargo-generate-rpm
cargo build --release
cargo deb
cargo generate-rpm
```

## Shell installer (cargo-dist)

When a release publishes the installer:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/chtnnh/git-gist/releases/latest/download/git-gist-installer.sh | sh
```

Full operator guide: [Packaging](./packaging.md) and [`packaging/README.md`](../../packaging/README.md).
