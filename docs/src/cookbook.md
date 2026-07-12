# Cookbook

## Update all dirty work repos

```bash
gg -g work --only-dirty pull --ff-only
```

## Commit message hook everywhere

```bash
gg hooks install commit-msg-required
```

## JSON for scripting

```bash
gg --format json overview | jq '.[] | select(.dirty)'
```

## Scaffold a service

```bash
gg init --profile default ./payments-api
```
