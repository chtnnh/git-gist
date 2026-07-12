# Packaging channels for git-gist (`gg`)

## Quick matrix

| Channel | Status | How users install | How you publish |
|---------|--------|-------------------|-----------------|
| **crates.io** | Manual | `cargo install git-gist --locked` | `cargo publish` |
| **GitHub Releases** | cargo-dist on tags | Archives + installers | Push `v*` tag |
| **Shell / PowerShell installer** | cargo-dist | `curl …/git-gist-installer.sh \| sh` | Automatic on tag |
| **Homebrew** | Tap live | `brew install chtnnh/tap/git-gist` | cargo-dist publishes formula via `HOMEBREW_TAP_TOKEN` |
| **deb / rpm** | On release publish | Download from Releases | `.github/workflows/packages.yml` |
| **Nix** | Flake | `nix profile install github:chtnnh/git-gist` | Flake on `main`; optional nixpkgs/NUR later |

## Release flow

1. Bump version in `Cargo.toml`, `CHANGELOG.md`, `flake.nix`
2. `cargo test --workspace` (CI also enforces ≥95% coverage on `main` / tags)
3. Commit, tag `vX.Y.Z`, push tag
4. **Release** workflow (cargo-dist) builds archives, `git-gist-installer.sh` / `.ps1`, Homebrew formula, and GitHub Release
5. **Linux packages** workflow attaches `.deb` / `.rpm` once the Release is published
6. `cargo publish` when ready for crates.io

## Homebrew

Tap repo: [`chtnnh/homebrew-tap`](https://github.com/chtnnh/homebrew-tap)

```bash
brew tap chtnnh/tap   # first time; may need `brew trust chtnnh/tap` on Homebrew 6+
brew install git-gist
```

- **v1.0.0 bootstrap:** source-build formula in the tap (historical).
- **v1.1.0+:** cargo-dist overwrites `Formula/git-gist.rb` with a bottle/prebuilt formula (`tap = "chtnnh/homebrew-tap"` in `dist-workspace.toml`).
- Requires repo secret `HOMEBREW_TAP_TOKEN` (PAT with Contents write on `chtnnh/homebrew-tap`).

Template / fallback source formula: [`packaging/homebrew/git-gist.rb`](homebrew/git-gist.rb) (kept in sync for operators who build from source).

## cargo-dist

Config: [`dist-workspace.toml`](../dist-workspace.toml) + `[package.metadata.dist]` formula name in `Cargo.toml`.

```bash
brew install axodotdev/tap/cargo-dist   # or: cargo install cargo-dist
dist plan                               # preview artifacts
dist generate                           # refresh `.github/workflows/release.yml`
```

Do **not** hand-edit `release.yml`; regenerate from `dist-workspace.toml`.

## deb / rpm locally

```bash
cargo install cargo-deb cargo-generate-rpm
cargo build --release
cargo deb
cargo generate-rpm
```

## Nix

```bash
nix run . -- version
nix profile install github:chtnnh/git-gist
```
