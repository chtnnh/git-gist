# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- `--cwd` / default `--root .` when no config root for “just this folder” workflows
- `--under <path>` sugar for directory include; optional globals-after-verb via `--` sentinel
- `gg group sync` from discovery
- Quiet overview / summary-only mode for large dirty trees
- Publish matching crates.io version so `cargo install git-gist` tracks GitHub releases

## [1.3.0] - 2026-08-12

### Added
- Canonical config/state dir: `~/.git-gist/` (`config.toml`, `discovery.json`, `state.json`) with one-time migration from XDG / Application Support
- Interactive config: `gg config wizard` / `gg wizard` (`inquire`) and `gg config ui` / `gg ui` (`ratatui`); scoped `wizard`/`ui` on `alias`, `group`, `tag`, `remotes`, `config enroll`
- `gg tag` CRUD; `gg config enroll`; `gg config edit`; `gg group member add|remove`; `gg alias prune`; `gg group prune`
- `[[auto_enroll]].path_prefix` to limit group/tag assignment under a relative prefix
- Automatic throttled auto-enroll during selection commands; `gg update` remains a force/dry-run fallback (`--prune-stale`, `--ask`, `--no-prune-stale`)
- `gg doctor --config` — stale aliases, unknown-key suggestions, missing watch roots, orphan group/tag members, dangerous root+groups rules
- Selection summary on stderr when `-g` / `--tag` / `-i` / `-x` filters are active
- Programmatic unknown-key detection via edit distance (no hardcoded misspellings); clearer `gg update` empty-rules errors (includes config path)
- Interactive config chapter + synthetic-repo screenshots (`scripts/docs-screenshots.py`)
- `gg man --output` writes nested subcommand pages (`gg-alias.1`, `gg-config-enroll.1`, …) beside the root page
- `tests/command_matrix.rs` — systematic happy/unhappy CLI coverage for every command/subcommand

### Changed
- README and user guide for `~/.git-gist/` layout and interactive config UX
- `gg each` uses the platform shell (`sh -c` on Unix, `COMSPEC`/`cmd.exe /C` on Windows)
- Auto-enroll throttle: time interval + rules-hash + watch-root mtime (not mtime alone)
- Selection summary prints after `--only-*` status filters

### Fixed
- `gg sync --format json` emitted two JSON documents; now a single sync-row array with per-repo `fetch_ok`
- `gg sync --pull` no longer attempts `git pull --ff-only` when fetch failed for that repo
- `--tag` now reaches tagged aliases outside discovery depth/root (parity with `-g` / `-i`)
- Circular group definitions error instead of stack overflowing
- `--fail-fast` includes skipped repos in results/JSON instead of dropping them
- Status filters warn on probe failures instead of silently dropping repos
- Auto-enroll errors surface on stderr (and fail under `--refresh`) instead of being discarded
- Auto-enroll: `record_state` failure after a successful config save is a warning (config stays saved; in-memory cfg still reloads)
- `path_prefix` matches Windows-style backslash prefixes
- Config migration overwrites empty/`schema_version`-only stub dest when legacy config has content
- `gg config edit` defaults to `notepad.exe` on Windows when `EDITOR`/`VISUAL` unset
- `save_global` returns an error when home cannot be resolved instead of panicking
- Selection summary `skipped` counts discovered-but-unselected repos (correct with out-of-universe tags)
- Empty/blank `COMSPEC` falls back to `cmd.exe` for `gg each` on Windows
- Discovery walk/`.ggignore` errors propagate instead of becoming an empty selection
- Status-filter probes hard-fail on `git status` / `git stash list` errors (including combined `--only-*` flags)

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
