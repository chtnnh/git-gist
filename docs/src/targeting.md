# Targeting & flags

Global flags are defined once on `gg` and apply to selection-based commands.

## Discovery & selection

| Flag | Effect |
|------|--------|
| `--root <DIR>` | Search root (overrides config `root`). Includes the root itself when it is a git repo. Aliases **outside** this root are not auto-included (use `-i`). |
| `--depth <N>` | Max scan depth (`0` = unlimited). Also applies to under-root aliases (deep aliases are skipped unless you pull them in with `-i` / `-g` / `--tag`). |
| `-i` / `--in <TARGET>` | Include alias, path, group, basename, or glob (repeatable). An existing **directory** includes all discovered repos under that prefix. |
| `-x` / `--exclude <TARGET>` | Exclude (repeatable). An existing **directory** excludes all selected repos under that prefix (not just an exact path match). |
| `-g` / `--group <NAME>` | Select a named group |
| `--tag <TAG>` | Filter by config tag |
| `--refresh` | Bypass discovery cache |
| `--include-submodules` | Treat gitfile submodules as repos |

Status filters (probe each repo):

| Flag | Keeps |
|------|-------|
| `--only-dirty` / `--only-clean` | Working tree state |
| `--only-ahead` / `--only-behind` | Upstream divergence |
| `--only-stashed` | Repos with stashes |
| `--only-detached` | Detached HEAD |

## Execution & output

| Flag | Effect |
|------|--------|
| `-j` / `--jobs` | Parallelism for passthrough, `each`, overview probes, and status filters |
| `--fail-fast` | Stop scheduling more work after first failure (passthrough / `each`) |
| `--dry-run` | Print planned actions; also honored by `update`, `hooks install`, `remotes` mutations, `init`/`scaffold`, `alias`/`group`/`config set`. For passthrough, put globals **before** the verb (`gg --dry-run status`, not `gg status --dry-run`). |
| `-q` / `--quiet` | Suppress informational output; hides successful passthrough/`each` blocks |
| `--timing` / `-v` | Per-repo timing on passthrough / `each` |
| `--format human\|json\|ndjson` | Output shape |
| `--color auto\|always\|never` | Color |
| `--theme <NAME>` | `default`, `mono`, `vivid` |
| `--show-path` | Human tables/findings show `name (path)` (also `show_path = true` in config) |

## Which commands use selection?

**Yes** (discovery + filters): `overview`, `list`, `info`, `commits`, `worktrees`, `doctor` (without `--config`), `stale`, `each`, `sync`, `hooks install`, `remotes add-to`, git passthrough. When `-g` / `--tag` / `-i` / `-x` are set, human mode prints a selection summary on stderr.

**No** (ignore `--in` / `--root` / `only_*`): `update` (uses `auto_enroll` paths; enrollment also runs throttled during selection), `alias`, `group`, `tag`, `config`, `wizard`, `ui`, `hooks list`, `remotes list|add|remove|wizard|ui`, `doctor --config`, `init`/`scaffold`, `self-update`, `completions`, `man`, `version`.

![Selection summary + overview for `-g oss`](./images/overview-oss.png)

![`gg -g learning list`](./images/list-learning.png)

## Examples

```bash
# Current directory only
gg ov --root .

# One group, dirty repos only
gg -g work --only-dirty status -sb

# Preview enroll without writing config
gg update --dry-run

# Reach an alias outside --root
gg --root . --in elsewhere list

# Shallow scan (deep aliases under root are skipped)
gg --root ~/code --depth 1 --refresh list

# Drop an entire tree (all repos under foundation/)
gg -g oss --exclude ~/code/foundation list

# Only repos under a directory
gg --root ~/code --in ~/code/foundation list
```
