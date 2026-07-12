# Configuration

## Locations

| Scope | Path |
|-------|------|
| Global | `~/.config/git-gist/config.toml` (macOS: `~/Library/Application Support/git-gist/config.toml`) |
| Local | `.gg.toml` or `.git-gist.toml` (ancestors of cwd) |
| Ignore | `.ggignore` at search root |
| Cache | `~/.cache/git-gist/discovery.json` |

## Schema

`schema_version = 1` — bump with migrations documented in CHANGELOG.

Keys: `root`, `depth`, `jobs`, `ignore`, `aliases`, `groups`, `tags`, `remotes`, `profiles`, `hook_packs`, `theme`, `include_submodules`, `repo_overrides`, `auto_enroll`.

```bash
gg config show
gg config get depth
gg config set depth 8
gg alias add api ~/src/api
gg group add work api web
```

## Auto-enroll

Declare watch folders; `gg update` creates aliases for new git repos and adds them to the listed groups/tags:

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
```

Existing aliases are left in place; missing group/tag membership is repaired. Alias names prefer the directory basename, then a path-derived name if that collides.
