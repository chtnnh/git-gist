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
| `doctor` | | Environment + repo health checks (`--config` for config hygiene) |
| `stale --days N` | | Repos with no commits newer than N days |

Default (no subcommand) runs `overview`.

![`gg -g oss ov`](./images/overview-oss.png)

![`gg -g learning list`](./images/list-learning.png)

## Multi-repo actions

| Command | Purpose |
|---------|---------|
| `each <shell…>` | Run a shell command in each selected repo (`--dry-run`, `-j`, `--fail-fast`, `-q`). Uses `sh -c` on Unix and `COMSPEC` / `cmd.exe /C` on Windows (POSIX-only scripts need Git Bash or WSL). |
| `sync [--pull]` | `git fetch --all --prune`; optional ff-only pull when clean and behind. `--format json` emits a single array of sync rows (per-repo `fetch_ok`); it does not also dump raw fetch output. |
| `update` | Force enroll from `[[auto_enroll]]` (`--dry-run`, `--prune-stale`, `--ask`); also runs automatically |

![`gg update --dry-run`](./images/update-dry-run.png)

![`gg doctor --config`](./images/doctor-config.png)

## Catalog & config

| Command | Purpose |
|---------|---------|
| `alias` / `group` / `tag` | Manage aliases, groups, tags (`prune`, `member`, `wizard`, `ui`) |
| `config` | `show` / `path` / `get` / `set` / `edit` / `enroll` / `wizard` / `ui` |
| `wizard` / `ui` | Interactive config hub ([walkthrough](./interactive.md)) |
| `remotes` | Catalog + `add-to` + `wizard` / `ui` |
| `hooks` | List packs / `install` into selection |
| `init` / `scaffold` | Create a repo from a profile |

Selection flags do **not** apply to catalog-only subcommands (`alias list`, `config show`, `hooks list`, …). See [Targeting & flags](./targeting.md).

## Meta

| Command | Purpose |
|---------|---------|
| `completions <shell>` | Shell completions |
| `man [--output PATH]` | Generate man pages: root + nested subcommands (`gg.1`, `gg-alias.1`, `gg-config-enroll.1`, …). `PATH` may be a directory or a file like `…/gg.1` (siblings written beside it). Stdout without `--output` is the root page only. |
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
