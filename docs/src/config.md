# Configuration

## Locations

| Scope | Path |
|-------|------|
| Global | `~/.config/git-gist/config.toml` |
| Local | `.gg.toml` or `.git-gist.toml` (ancestors of cwd) |
| Ignore | `.ggignore` at search root |
| Cache | `~/.cache/git-gist/discovery.json` |

## Schema

`schema_version = 1` — bump with migrations documented in CHANGELOG.

Keys: `root`, `depth`, `jobs`, `ignore`, `aliases`, `groups`, `tags`, `remotes`, `profiles`, `hook_packs`, `theme`, `include_submodules`, `repo_overrides`.

```bash
gg config show
gg config get depth
gg config set depth 8
gg alias add api ~/src/api
gg group add work api web
```
