# This file documents packaging channels for git-gist (`gg`).
# See also: docs/src/packaging.md and packaging/

## Quick matrix

| Channel | Status | How users install | How you publish |
|---------|--------|-------------------|-----------------|
| **crates.io** | Ready anytime | `cargo install git-gist` | `cargo publish` |
| **GitHub Releases** | CI on tags | Download binary / installer | Push `v*` tag |
| **Homebrew** | Tap formula | `brew install chtnnh/tap/git-gist` | Push formula to tap repo |
| **deb** | Metadata + CI | `apt install ./git-gist_*.deb` | `cargo deb` in release |
| **rpm** | Metadata + CI | `rpm -i git-gist-*.rpm` | `cargo generate-rpm` in release |
| **Nix** | `flake.nix` | `nix profile install github:chtnnh/git-gist` | Merge flake; optional NUR |
| **Scoop** (Windows) | Optional | `scoop install git-gist` | Add bucket JSON |
| **AUR** | Optional | `yay -S git-gist` | Publish PKGBUILD |

## Recommended publish order

1. Push code to GitHub, create annotated tag `v1.0.0`
2. Release workflow builds archives + man page
3. Publish to crates.io (`cargo publish`)
4. Create `chtnnh/homebrew-tap`, copy/update `packaging/homebrew/git-gist.rb` with release URL + sha256
5. Attach `.deb` / `.rpm` from `cargo deb` / `cargo generate-rpm` (or cargo-dist)
6. Point README install section at live URLs
7. Optional: submit nixpkgs PR later; flake works immediately

## Homebrew tap (concrete steps)

```bash
# one-time
gh repo create chtnnh/homebrew-tap --public
git clone git@github.com:chtnnh/homebrew-tap.git
mkdir -p homebrew-tap/Formula
cp packaging/homebrew/git-gist.rb homebrew-tap/Formula/
# edit url + sha256 from GitHub release tarball
# shasum -a 256 git-gist-1.0.0.tar.gz
cd homebrew-tap && git add Formula/git-gist.rb && git commit -m "git-gist 1.0.0" && git push

# users
brew tap chtnnh/tap
brew install git-gist
```

## deb / rpm locally

```bash
cargo install cargo-deb cargo-generate-rpm
cargo build --release
cargo deb
cargo generate-rpm
# artifacts under target/debian/ and target/generate-rpm/
```

## Nix

```bash
nix run . -- version
nix profile install .
# or from GitHub once pushed:
nix profile install github:chtnnh/git-gist
```

## cargo-dist (optional upgrade)

`Cargo.toml` already has `[package.metadata.dist]`. To fully automate brew/deb-like installers:

```bash
cargo install cargo-dist
dist init   # merges release workflow
dist build
```

Prefer one release system (cargo-dist **or** the hand-rolled `.github/workflows/release.yml`) to avoid duplicate uploads.
