# git-gist (`gg`)

**Run git across all child repositories — fast.**

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

# top commits across selection
gg commits -n 10
```

## Install

### From source

```bash
cargo install --path . --locked
# binary: gg
```

### Homebrew (tap)

```bash
brew install chtnnh/tap/git-gist
```

### Debian / RPM

Download `.deb` / `.rpm` from [GitHub Releases](https://github.com/chtnnh/git-gist/releases), or build with `cargo deb` / `cargo generate-rpm` (see [packaging/README.md](packaging/README.md)).

### Nix

```bash
nix run github:chtnnh/git-gist -- version
nix profile install github:chtnnh/git-gist
```

### cargo-dist shell installer

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/chtnnh/git-gist/releases/latest/download/git-gist-installer.sh | sh
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
| `overview` / `ov` | Dashboard: branch, dirty, ahead/behind |
| `list` / `ls` | List discovered repos |
| `info` | Detailed status |
| `commits -n` | Top-N commits |
| `worktrees` | Worktree listing |
| `doctor` | Health checks |
| `each` | Run arbitrary shell in each repo |
| `sync [--pull]` | Fetch (+ optional ff-only pull) |
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

- Global: `~/.config/git-gist/config.toml`
- Local: `.gg.toml` or `.git-gist.toml` (walks up from cwd)
- Ignore globs: config `ignore` + `.ggignore`

```toml
schema_version = 1
depth = 6
jobs = 8
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
```

## Global flags

- `--root`, `--in` / `-i`, `--exclude` / `-x`, `-g <group>`
- `-j` jobs, `--fail-fast`, `--dry-run`, `--timing`
- `--only-dirty`, `--only-clean`, `--only-ahead`, `--only-behind`, …
- `--format human|json|ndjson`, `--color auto|always|never`, `--theme`

## License

MIT — see [LICENSE](LICENSE).
