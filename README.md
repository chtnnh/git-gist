# git-gist (`gg`)

**Run git across all child repositories — fast.**

Current release: **1.2.0** · Docs: **https://gg.chtnnhfoundation.org/**

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

## Built-in commands

| Command | Description |
|---------|-------------|
| `overview` / `ov` | Dashboard: branch, dirty, ahead/behind (colored) |
| `list` / `ls` | List discovered repos |
| `info` | Detailed status |
| `commits -n` | Top-N commits |
| `worktrees` | Worktree listing |
| `doctor` | Health checks |
| `each` | Run arbitrary shell in each repo |
| `sync [--pull]` | Fetch (+ optional ff-only pull) |
| `update` | Enroll repos from `[[auto_enroll]]` rules |
| `stale --days N` | Repos without recent commits |
| `alias` / `group` | Manage aliases & groups |
| `config` | Show/get/set config |
| `init` / `scaffold` | Scaffold from a profile |
| `hooks` | Install hook packs |
| `remotes` | Remote catalog |
| `completions` | Shell completions |
| `man` | Man page |
| `self-update` | Update guidance / release probe |

Anything else is passed through to `git` in each selected repo. Escape hatch: `gg git -- <args>`.

## Configuration

- Global: `~/.config/git-gist/config.toml` (macOS: `~/Library/Application Support/git-gist/config.toml`)
- Local: `.gg.toml` or `.git-gist.toml` (walks up from cwd)
- Ignore globs: config `ignore` + `.ggignore`

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
path = "/Users/you/src/learning"
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

## What’s new in 1.2.0

- Faster status probes and `--only-*` filters (fewer git process spawns)
- `show_path` / `--show-path` for human tables
- Directory prefix `--in` / `--exclude`, depth-aware under-root aliases
- Clearer errors when globals land after a passthrough verb

Full notes: [CHANGELOG.md](CHANGELOG.md).

## Documentation

- Online: https://gg.chtnnhfoundation.org/
- Source: [`docs/`](docs/) (mdBook). Local: `mdbook serve docs --open`
- Before commit/push: `./scripts/ci.sh`

## License

MIT — see [LICENSE](LICENSE).
