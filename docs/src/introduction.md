# Introduction

**git-gist** (`gg`) is a fast, cross-platform CLI that discovers git repositories under a root and runs git (or built-in insights) across them in parallel.

Current release: **1.1.0**. User guide: [https://gg.chtnnhfoundation.org/](https://gg.chtnnhfoundation.org/).

Design pillars:

1. **Hybrid discovery** — auto-scan children, plus aliases, groups, and filters
2. **Direct passthrough** — `gg status` means `git status` everywhere selected
3. **Reserved builtins** — overview, list, sync, update, scaffold, etc. win over passthrough
4. **Scriptable** — JSON/NDJSON, stable exit aggregation, dry-run

## What’s in 1.1.0

- Semantic colors in `overview` / `sync` / `stale` tables (tree, age, ahead/behind)
- `[[auto_enroll]]` + `gg update` to enroll new repos into aliases / groups / tags
- Correct `--root` / selection / `--dry-run` behavior across commands (see [Targeting & flags](./targeting.md))
