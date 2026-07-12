# Contributing to git-gist

Thanks for helping improve `gg`.

## Development

Requirements: Rust 1.75+, Git.

```bash
cargo build
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Binary name is `gg` (`cargo run -- <args>`).

### Coverage

Line coverage must stay **≥ 95%** (enforced in CI).

```bash
./scripts/coverage.sh
# or:
cargo llvm-cov --workspace --fail-under-lines 95 \
  --ignore-filename-regex '(tests/|/cargo/registry/)'
```

Add or extend tests under `tests/` for every new command or public code path.

## Guidelines

- Prefer small, focused PRs aligned with the version roadmap in the README / docs.
- Passthrough must shell out to `git` on `PATH` — do not reimplement porcelain.
- Add/extend tests under `tests/` for CLI behavior.
- Update `CHANGELOG.md` for user-visible changes.
- Keep reserved builtin names documented when adding commands.

## Commit style

Conventional, imperative subjects are appreciated:

- `feat: add stale filter`
- `fix: skip search root in discovery`
- `docs: clarify shell setup`

## Code of conduct

By participating you agree to the [Code of Conduct](CODE_OF_CONDUCT.md).
