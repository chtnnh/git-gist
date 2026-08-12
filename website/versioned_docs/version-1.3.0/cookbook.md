# Cookbook

## Overview for the current folder only

```bash
gg ov --root .
```

## Show paths in human tables

```bash
gg --show-path ov
gg --show-path -g work stale --days 90

# persist
gg config set show_path true
```

## Update all dirty work repos

```bash
gg -g work --only-dirty pull --ff-only
```

## Drop or include an entire directory tree

```bash
# exclude every selected repo under foundation/
gg -g oss --exclude ~/code/oss/foundation list

# only repos under that directory
gg --root ~/code --in ~/code/oss/foundation list
```

## Enroll new learning / OSS checkouts

```toml
# in global config
[[auto_enroll]]
path = "/Users/you/Desktop/tech/learning"
depth = 6
tags = ["learning"]

[[auto_enroll]]
path = "/Users/you/Desktop/tech/oss"
depth = 3
groups = ["oss"]
```

```bash
gg update --dry-run
gg update
gg --tag learning ov
```

## Commit message hook everywhere

```bash
gg hooks install commit-msg-required
gg --dry-run hooks install noop   # preview
```

## JSON for scripting

```bash
gg --format json overview | jq '.[] | select(.dirty)'
gg --format json update | jq '.added'
```

## Sync then show drift

```bash
gg -g work sync --pull
gg -g work --only-behind ov
```

## Scaffold a service

```bash
gg init --profile default ./payments-api
gg --dry-run init --profile default ./scratch
```

## Passthrough dry-run (flags before the verb)

```bash
gg --dry-run --timing status -sb
```

## Upgrade Homebrew when PATH has a Cargo `gg`

```bash
brew update && brew upgrade git-gist
which -a gg
# If ~/.cargo/bin/gg wins, either reorder PATH or:
# cargo uninstall git-gist
```
