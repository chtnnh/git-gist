# Quick start

```bash
cd ~/src          # folder that contains many git checkouts
gg                # overview (colored tree / age / sync)
gg list
gg status -sb
gg --only-dirty pull
gg -g work fetch
```

![`gg -g oss ov`](./images/overview-oss.png)

Scope to the current directory (honors `--root`; does not pull in out-of-root aliases):

```bash
gg ov --root .
```

Show paths next to repo names (useful when basenames collide):

```bash
gg --show-path ov
# or permanently:
gg config set show_path true
```

Shallow discovery / drop a whole subtree:

```bash
gg --root ~/code --depth 2 --refresh list
gg -g work --exclude ~/code/legacy list
```

Enroll new repos from watch rules in config:

```bash
gg update --dry-run
gg update
```

![`gg update --dry-run`](./images/update-dry-run.png)

Global flags go **before** a passthrough git verb:

```bash
gg --dry-run status -sb    # good
# gg status --dry-run      # errors with a hint
```

Generate completions:

```bash
gg completions zsh > ~/.zsh/completions/_gg
```

Man pages (root + subcommands):

```bash
# writes gg.1, gg-alias.1, gg-config.1, gg-config-enroll.1, …
gg man --output /usr/local/share/man/man1
# same when pointing at the root file:
gg man --output /usr/local/share/man/man1/gg.1
```
