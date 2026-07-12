# Commands

## Builtins

See `gg --help` and per-command `--help`.

Notable:

- `overview` / `ov` — table of branch / dirty / ahead-behind (semantic colors for tree, age, sync drift)
- `update` — enroll new repos from `[[auto_enroll]]` rules into aliases / groups / tags
- `sync [--pull]` — fetch all; optional ff-only pull when clean and behind
- `each 'cargo test'` — arbitrary shell per repo
- `init --profile default ./new-repo` — scaffold
- `hooks install noop` — install hook pack into selection
- `remotes add-to origin-template` — apply catalog remote

## Passthrough

```bash
gg status
gg pull --rebase
gg git -- status   # escape hatch
```

Exit code is non-zero if any selected repo fails (unless filtered empty).
