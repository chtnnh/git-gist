---
slug: /
sidebar_position: 1
---

# Introduction

**git-gist** (`gg`) is a fast, cross-platform CLI that discovers git repositories under a root and runs git (or built-in insights) across them in parallel.

Current release: **1.3.0**. User guide: [https://gg.chtnnhfoundation.org/](https://gg.chtnnhfoundation.org/).

Design pillars:

1. **Hybrid discovery** — auto-scan children, plus aliases, groups, and filters
2. **Direct passthrough** — `gg status` means `git status` everywhere selected
3. **Reserved builtins** — overview, list, sync, update, scaffold, etc. win over passthrough
4. **Scriptable** — JSON/NDJSON, stable exit aggregation, dry-run
5. **Interactive config** — wizard + TUI so you rarely hand-edit TOML

![Overview for group `oss`](./images/overview-oss.png)

## What’s in 1.3.0

- Config at `~/.git-gist/`; `gg config wizard` / `gg config ui` (+ scoped `wizard`/`ui` on alias/group/tag/remotes/enroll)
- Automatic throttled auto-enroll; `path_prefix`; `gg alias prune` / `gg update --prune-stale`
- `gg doctor --config`, selection summaries for `-g` / `--tag`

See [Configuration](./config), [Interactive config](./interactive) (wizard / TUI + screenshots), and [Install](./install).

## What’s in 1.2.0

- Faster status probes and `--only-*` filters (fewer git spawns; maintainers: `benches/PROBE_PERF.md`)
- `show_path` / `--show-path` — human tables print `name (path)`
- Selection fixes: depth-aware under-root aliases, directory `--exclude`/`--in`, clearer passthrough flag-order errors
- User guide on GitHub Pages, `./scripts/ci.sh`, and reproducible `scripts/bench.py`

See [Install](./install) for Homebrew upgrade / `PATH` shadowing notes, and [Targeting & flags](./targeting) for selection behavior.

## What’s in 1.1.0

- Semantic colors in `overview` / `sync` / `stale` tables (tree, age, ahead/behind)
- `[[auto_enroll]]` + `gg update` to enroll new repos into aliases / groups / tags
- Correct `--root` / selection / `--dry-run` behavior across commands (see [Targeting & flags](./targeting))
