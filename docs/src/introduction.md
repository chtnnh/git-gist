# Introduction

**git-gist** (`gg`) is a fast, cross-platform CLI that discovers git repositories under a root and runs git (or built-in insights) across them in parallel.

Design pillars:

1. **Hybrid discovery** — auto-scan children, plus aliases, groups, and filters
2. **Direct passthrough** — `gg status` means `git status` everywhere selected
3. **Reserved builtins** — overview, list, scaffold, etc. win over passthrough
4. **Scriptable** — JSON/NDJSON, stable exit aggregation, dry-run
