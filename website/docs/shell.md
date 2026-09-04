# Shell integration

Check for an existing `gg` definition before sourcing the helper. `command 'gg'` bypasses an alias, function, or Zsh global alias long enough to run the check:

```bash
# bash
command 'gg' doctor --shell bash
eval "$(command 'gg' doctor --shell bash --setup)"

# zsh
command 'gg' doctor --shell zsh
eval "$(command 'gg' doctor --shell zsh --setup)"

# fish
command 'gg' doctor --shell fish
command 'gg' doctor --shell fish --setup | source
```

The setup and helpers refuse to overwrite or use an existing `gg` alias/function (or Fish abbreviation). Remove or rename it first, then source the helper for your shell from the `shell/` directory:

- `gg.bash` / `gg.zsh` / `gg.fish` / `gg.sh`

Provides:

- `gg-cd <alias>` — jump to an aliased path
- `gg-prompt` — print dirty child count for prompts
- completion wiring when `gg` is on `PATH`
