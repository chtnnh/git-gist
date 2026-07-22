# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- README and user guide refreshed for 1.2.0 (Homebrew upgrade / PATH shadowing, `--show-path`, directory `--in`/`--exclude`)

### Planned
- `gg doctor --config` — warn on stale binary vs latest, empty `auto_enroll`, missing group members, duplicate basenames
- `--cwd` / default `--root .` when no config root for “just this folder” workflows
- `--under <path>` sugar for directory include; optional globals-after-verb via `--` sentinel
- `gg update` wizard suggestions when `auto_enroll` is empty; `gg group sync` from discovery
- Disambiguate basename collisions in human output (relative path under root; overlaps with `--show-path`)
- Quiet overview / summary-only mode for large dirty trees
- Publish matching crates.io version so `cargo install git-gist` tracks GitHub releases

## [1.2.0] - 2026-07-22

### Added
- mdBook user guide on GitHub Pages: https://gg.chtnnhfoundation.org/
- `./scripts/ci.sh` — local gate matching GitHub Actions (`fmt` + `clippy` + tests + ≥95% coverage)
- `scripts/bench.py` + `benches/` — reproducible wall-clock benchmarks for probe-heavy commands
- `show_path` config + `--show-path` — human tables/findings print `name (path)` (relative under root when possible)

### Changed
- Status probing: full overview probe uses 3 git spawns instead of ~8 (`status --porcelain=v2 --branch`, `stash list`, combined `log`); in-progress detection is filesystem-only
- `--only-*` filters gather only the fields they need (dirty/ahead/behind/detached → one porcelain call; stash-only skips status)
- `stale` and `doctor` use partial probes and parallel rayon pools
- Discovery cache hits no longer re-read the cache file to decide whether to save

### Fixed
- rustfmt drift that failed CI `cargo fmt --check`
- Under-root alias injection now respects `--depth`, config `ignore`, and `.ggignore` (deep aliases no longer bypass discovery limits)
- `-x` / `--exclude` and `-i` / `--in` treat existing directories as path prefixes (exclude/include all selected repos under that tree)
- Passthrough commands error with a clear hint when common global flags appear after the git verb (`gg status --dry-run` → put `--dry-run` before `status`)

## [1.1.0] - 2026-07-12

### Added
- Semantic colors in overview/sync/stale tables (clean/dirty, age bands, ahead/behind)
- `[[auto_enroll]]` config rules + `gg update` to enroll new repos into aliases/groups/tags
- Cross-command `tests/flags_matrix.rs` covering selection, dry-run side effects, and format flags

### Fixed
- Scaffold profiles parse without requiring empty `remotes` / `hooks` tables
- `--root` no longer unions out-of-root aliases into the selection; `--root` on a git repo includes that repo
- Selection flags no longer gate catalog/config commands (`alias`, `group`, `config`, `hooks list`, …)
- `--dry-run` honored by `hooks install`, `remotes` mutations, `init`/`scaffold`, `alias`/`group`/`config set`
- `-j` honored by overview probing and status filters; `-q`/`--timing` honored by `each`
- `info PATH` respects `--only-*` / `--in` / related selection filters
- `sync --dry-run --format json` emits JSON instead of suppressing repo headers
- deb/rpm packaging trigger after cargo-dist releases

## [1.0.0] - 2026-07-12

### Added
- Initial stable release of `gg` (git-gist)
- Hybrid discovery of child git repositories with depth limits, ignores, and `.ggignore`
- Git passthrough (`gg status`, …) with reserved builtins and `gg git` escape hatch
- Insights: `overview`, `list`, `info`, `commits`, `worktrees`, `doctor`, `stale`
- Targeting: `--in`, `--exclude`, `-g` groups, aliases, tags, status filters
- Config: global + local TOML, schema_version, remotes catalog, scaffold profiles, hook packs
- `each`, `sync`, `init`/`scaffold`, `hooks`, `remotes`
- Completions, man page generation, color themes, JSON/NDJSON output
- Shell helpers (`gg-cd`, prompt), CI, cargo-dist packaging metadata
- OSS docs: README, LICENSE, CONTRIBUTING, CODE_OF_CONDUCT, SECURITY, FUNDING
- Full command integration tests + unit tests; CI coverage gate at **≥95%** line coverage (`cargo llvm-cov`)
- Packaging: cargo-dist release (shell/PowerShell installers + Homebrew publish), deb/rpm attach workflow, live `chtnnh/homebrew-tap`
- Operator guide in `packaging/README.md`

## [0.1.0] - 2026-07-12

### Added
- Project scaffold and development milestones leading to 1.0.0
