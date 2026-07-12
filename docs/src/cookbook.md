# Cookbook

## Overview for the current folder only

```bash
gg ov --root .
```

## Update all dirty work repos

```bash
gg -g work --only-dirty pull --ff-only
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
