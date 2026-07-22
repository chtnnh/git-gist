# Commands

See `gg --help` and per-command `--help` for the full flag list.

## Insights

| Command | Alias | Purpose |
|---------|-------|---------|
| `overview` | `ov` | Dashboard: branch, dirty/clean, ahead/behind, age, in-progress (semantic colors; `--show-path` adds path to the repo column) |
| `list` | `ls` | Discovered / selected repos (`--refresh` bypasses cache; always prints name + path) |
| `info [PATH]` | | Detailed status; optional path still respects `--only-*` / `--in` when those are set |
| `commits -n N` | | Top-N commits across selection |
| `worktrees` | | Worktree listing |
| `doctor` | | Environment + repo health checks |
| `stale --days N` | | Repos with no commits newer than N days |

Default (no subcommand) runs `overview`.

## Multi-repo actions

| Command | Purpose |
|---------|---------|
| `each <shell…>` | Run a shell command in each selected repo (`--dry-run`, `-j`, `--fail-fast`, `-q`) |
| `sync [--pull]` | `git fetch --all --prune`; optional ff-only pull when clean and behind |
| `update` | Enroll repos from `[[auto_enroll]]` into aliases / groups / tags |

## Catalog & config

| Command | Purpose |
|---------|---------|
| `alias` / `group` | Manage path aliases and groups |
| `config` | `show` / `path` / `get` / `set` |
| `remotes` | Catalog + `add-to` selected repos |
| `hooks` | List packs / `install` into selection |
| `init` / `scaffold` | Create a repo from a profile |

Selection flags do **not** apply to catalog-only subcommands (`alias list`, `config show`, `hooks list`, …). See [Targeting & flags](./targeting.md).

## Meta

| Command | Purpose |
|---------|---------|
| `completions <shell>` | Shell completions |
| `man [--output FILE]` | Generate man page |
| `version` | Print version |
| `self-update` | Release probe / upgrade guidance |

## Passthrough

Anything that is not a builtin is passed to `git` in each selected repo:

```bash
gg status -sb
gg pull --rebase
gg git -- status   # escape hatch when a name collides with a builtin
```

Put global flags **before** the git verb: `gg --dry-run status` (not `gg status --dry-run`). If a common global flag appears after the verb, `gg` errors with a hint instead of forwarding it to git.

Exit code is non-zero if any selected repo fails (unless the selection is empty).
