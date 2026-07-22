# Configuration

## Locations

| Scope | Path |
|-------|------|
| Global | `~/.config/git-gist/config.toml` (macOS: `~/Library/Application Support/git-gist/config.toml`) |
| Local | `.gg.toml` or `.git-gist.toml` (ancestors of cwd) |
| Ignore | `.ggignore` at search root |
| Cache | `~/.cache/git-gist/discovery.json` |

Example file: [`examples/config.toml`](https://github.com/chtnnh/git-gist/blob/main/examples/config.toml).

## Schema

`schema_version = 1` — bump with migrations documented in CHANGELOG.

Keys: `root`, `depth`, `jobs`, `ignore`, `aliases`, `groups`, `tags`, `remotes`, `profiles`, `hook_packs`, `theme`, `include_submodules`, `show_path`, `repo_overrides`, `auto_enroll`.

```bash
gg config show
gg config get depth
gg config set depth 8
gg alias add api ~/src/api
gg group add work api web
```

CLI overrides for one invocation: `--root`, `--depth`, `-j`, `--theme`, `--include-submodules`, `--show-path`.

## Auto-enroll

Declare watch folders; `gg update` creates aliases for newly discovered git repos and adds them to the listed groups/tags:

```toml
[[auto_enroll]]
path = "/home/you/src/learning"
depth = 6
tags = ["learning"]

[[auto_enroll]]
path = "/home/you/src/oss"
depth = 3
groups = ["oss"]
```

```bash
gg update --dry-run   # preview
gg update             # write aliases / groups / tags
gg --format json update
```

Notes:

- Existing aliases are left in place; missing group/tag membership is repaired.
- Alias names prefer the directory basename, then a path-derived name, then numeric suffixes on collision.
- Missing watch roots are skipped with a warning.
- `gg update` does **not** use `--root` / `-i` selection; it only follows `[[auto_enroll]]` rules.

## Themes

`theme = "default" | "mono" | "vivid"` (or `--theme` on the CLI). Overview / sync / stale tables use semantic cell colors for dirty trees, stale ages (≥30d / ≥90d), and ahead/behind drift.

## Show path with repo name

`show_path = true` (or `--show-path`) prints `name (relative-or-absolute path)` in human overview / sync / stale / commits / doctor / worktrees tables. JSON still uses separate `name` and `path` fields. Relative paths are preferred when the repo is under `--root` / config `root`.
