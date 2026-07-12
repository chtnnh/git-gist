# Quick start

```bash
cd ~/src          # folder that contains many git checkouts
gg                # overview (colored tree / age / sync)
gg list
gg status -sb
gg --only-dirty pull
gg -g work fetch
```

Scope to the current directory (honors `--root`; does not pull in out-of-root aliases):

```bash
gg ov --root .
```

Enroll new repos from watch rules in config:

```bash
gg update --dry-run
gg update
```

Generate completions:

```bash
gg completions zsh > ~/.zsh/completions/_gg
```

Man page:

```bash
gg man --output /usr/local/share/man/man1/gg.1
```
