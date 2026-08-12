# Contributing to git-gist

Thanks for helping improve `gg`.

## Development

Requirements: Rust 1.75+, Git.

```bash
cargo build
./scripts/ci.sh   # fmt + clippy + tests + ≥95% coverage (same gates as GitHub Actions)
```

Or run pieces individually:

```bash
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --check
./scripts/coverage.sh
```

Binary name is `gg` (`cargo run -- <args>`).

### Coverage

Line coverage must stay **≥ 95%** (enforced in CI via `./scripts/ci.sh` / `./scripts/coverage.sh`).

## Guidelines

- Prefer small, focused PRs aligned with the version roadmap in the README / docs.
- Passthrough must shell out to `git` on `PATH` — do not reimplement porcelain.
- Add/extend tests under `tests/` for CLI behavior.
- Update `CHANGELOG.md` **and** user-facing docs (`README.md`, `website/docs/…`) for user-visible changes.
- Keep reserved builtin names documented when adding commands.
- Preview the guide with `cd website && npm start` when editing guide pages.
- Docs default to the latest **released** freeze; edit `website/docs/` for HEAD (`/head`). Before tagging: `./scripts/docs-version.sh`. Optional: `git config core.hooksPath .githooks`.

## Commit style

Conventional, imperative subjects are appreciated:

- `feat: add stale filter`
- `fix: skip search root in discovery`
- `docs: clarify shell setup`

## Code of conduct

By participating you agree to the [Code of Conduct](CODE_OF_CONDUCT.md).
