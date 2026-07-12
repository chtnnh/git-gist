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

Requires [`chtnnh/homebrew-tap`](https://github.com/chtnnh/homebrew-tap). On Homebrew 6+, you may need `brew trust chtnnh/tap` once.

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

## Shell / PowerShell installer (cargo-dist)

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/chtnnh/git-gist/releases/latest/download/git-gist-installer.sh | sh
```

```powershell
irm https://github.com/chtnnh/git-gist/releases/latest/download/git-gist-installer.ps1 | iex
```

Full operator guide: [Packaging](./packaging.md) and [`packaging/README.md`](../../packaging/README.md).
