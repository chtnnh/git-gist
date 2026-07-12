# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Semantic colors in overview/sync/stale tables (clean/dirty, age bands, ahead/behind)
- `[[auto_enroll]]` config rules + `gg update` to enroll new repos into aliases/groups/tags

### Fixed
- Scaffold profiles parse without requiring empty `remotes` / `hooks` tables

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
