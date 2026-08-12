# git-gist (`gg`)

**Run git across all child repositories — fast.**

Current release: **1.3.0** · Docs: **https://gg.chtnnhfoundation.org/**

`gg` discovers git repos under a directory and runs git commands (or built-in insights) in parallel. One CLI for multi-checkout workspaces, client folders, and polyrepos.

```bash
# overview of every child repo
gg

# passthrough: git status in each repo
gg status -sb

# only dirty repos
gg --only-dirty pull

# target an alias or group
gg -g work fetch --all

# show paths next to repo names (also: show_path = true in config)
gg --show-path ov

# top commits across selection
gg commits -n 10
```

![`gg -g oss ov` — overview for a group](docs/src/images/overview-oss.png)

## Install

### Homebrew (tap)

```bash
brew tap chtnnh/tap          # first time; may need: brew trust chtnnh/tap
brew install git-gist
# or: brew install chtnnh/tap/git-gist

brew update && brew upgrade git-gist   # later releases
```

If `gg version` still looks old after upgrading, Homebrew’s `gg` may be shadowed by `~/.cargo/bin/gg`. Check with `which -a gg`, or use `/opt/homebrew/opt/git-gist/bin/gg` (Apple Silicon) / `/usr/local/opt/git-gist/bin/gg` (Intel).

### cargo-dist shell / PowerShell installer

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/chtnnh/git-gist/releases/latest/download/git-gist-installer.sh | sh
```

Windows (PowerShell):

```powershell
irm https://github.com/chtnnh/git-gist/releases/latest/download/git-gist-installer.ps1 | iex
```

### From crates.io / source

```bash
cargo install git-gist --locked
# or from this repo:
cargo install --path . --locked
# binary: gg
```

### Debian / RPM

Download `.deb` / `.rpm` from [GitHub Releases](https://github.com/chtnnh/git-gist/releases), or build with `cargo deb` / `cargo generate-rpm` (see [packaging/README.md](packaging/README.md)).

### Nix

```bash
nix run github:chtnnh/git-gist -- version
nix profile install github:chtnnh/git-gist
```

Operator guide for brew/deb/rpm/nix: [packaging/README.md](packaging/README.md).

## Shell setup

```bash
# bash
eval "$(gg completions bash)"
source <(curl -fsSL https://raw.githubusercontent.com/chtnnh/git-gist/main/shell/gg.bash)  # or local path

# zsh
eval "$(gg completions zsh)"
source /path/to/git-gist/shell/gg.zsh

# fish
gg completions fish | source
source /path/to/git-gist/shell/gg.fish
```

Helpers provide `gg-cd <alias>` and an optional prompt snippet.

Interactive config (wizard / TUI): see the [Interactive config](https://gg.chtnnhfoundation.org/interactive.html) chapter.

![`gg config wizard` — interactive config hub](docs/src/images/config-wizard.png)

## Built-in commands

| Command | Description |
|---------|-------------|
| `overview` / `ov` | Dashboard: branch, dirty, ahead/behind (colored) |
| `list` / `ls` | List discovered repos |
| `info` | Detailed status |
| `commits -n` | Top-N commits |
| `worktrees` | Worktree listing |
| `doctor` / `doctor --config` | Health checks / config hygiene |
| `each` | Run arbitrary shell in each repo (`sh` / Windows `cmd`) |
| `sync [--pull]` | Fetch (+ optional ff-only pull) |
| `update` | Force enroll from `[[auto_enroll]]` (also runs automatically) |
| `stale --days N` | Repos without recent commits |
| `alias` / `group` / `tag` | Manage aliases, groups, tags (`wizard` / `ui` / `prune`) |
| `config` | Show/get/set/edit; `wizard` / `ui` / `enroll` |
| `wizard` / `ui` | Interactive config hub (prompts / full-screen TUI) |
| `init` / `scaffold` | Scaffold from a profile |
| `hooks` | Install hook packs |
| `remotes` | Remote catalog (`wizard` / `ui`) |
| `completions` | Shell completions |
| `man` | Generate man pages (root + nested subcommands) |
| `self-update` | Update guidance / release probe |

Anything else is passed through to `git` in each selected repo. Escape hatch: `gg git -- <args>`.

## Configuration

- Global: `~/.git-gist/config.toml` (legacy XDG / Application Support paths migrate automatically)
- Local: `.gg.toml` or `.git-gist.toml` (walks up from cwd)
- Ignore globs: config `ignore` + `.ggignore`
- Interactive: `gg config wizard` / `gg config ui` (also `gg alias wizard`, `gg group ui`, …)

```toml
schema_version = 1
depth = 6
jobs = 8
theme = "vivid"
show_path = false   # or true / use --show-path
ignore = ["**/node_modules/**", "**/target/**"]

[aliases]
api = "/Users/you/src/api"
web = "/Users/you/src/web"

[groups]
work = ["api", "web"]

[remotes]
origin-template = "git@github.com:org/NAME.git"

[profiles.default]
default_branch = "main"
user_name = "You"
user_email = "you@example.com"

[[auto_enroll]]
path = "/Users/you/src"
path_prefix = "learning/"
depth = 6
tags = ["learning"]
```

## Global flags

- `--root`, `--in` / `-i`, `--exclude` / `-x`, `-g <group>`, `--tag`, `--depth`
- `-j` jobs, `--fail-fast`, `--dry-run`, `--timing`, `-q` / `--quiet`
- `--only-dirty`, `--only-clean`, `--only-ahead`, `--only-behind`, …
- `--format human|json|ndjson`, `--color auto|always|never`, `--theme`, `--show-path`

Selection notes:

- `--root` does not pull in aliases **outside** that tree (use `-i`).
- `--depth` also applies to under-root aliases.
- An existing **directory** for `-i` / `-x` includes or excludes all selected repos under that prefix.
- Selection flags apply to reporting and multi-repo commands (`ov`, `list`, `sync`, `each`, passthrough, …). Catalog/config commands (`alias`, `group`, `config`, `hooks list`, …) ignore them.
- Put global flags **before** external git verbs (`gg --dry-run status`). Misplaced globals after the verb error with a hint.

## What’s in 1.3.0

- Config lives in `~/.git-gist/`; interactive `wizard` / `ui` for aliases, groups, tags, remotes, auto-enroll
- Automatic (throttled) auto-enroll; `gg update --prune-stale` / `gg alias prune` reclaim short names after moves
- `path_prefix` on `[[auto_enroll]]`, selection summaries for `-g` / `--tag`, `gg doctor --config`

Full notes: [CHANGELOG.md](CHANGELOG.md). Planned follow-ups stay under `[Unreleased]`.

## Documentation

- Online: https://gg.chtnnhfoundation.org/
- Source: [`docs/`](docs/) (mdBook). Local: `mdbook serve docs --open`
- Before commit/push: `./scripts/ci.sh`

## License

MIT — see [LICENSE](LICENSE).
