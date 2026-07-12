# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 1.x     | Yes       |
| 0.x     | Best effort until 1.0 |

## Reporting a vulnerability

Please **do not** open a public issue for security vulnerabilities.

Use [GitHub Private Vulnerability Reporting](https://github.com/chtnnh/git-gist/security/advisories/new)
or email the repository maintainers.

Include:

- Affected version / commit
- Reproduction steps
- Impact assessment (e.g. command injection via config, path traversal)

We aim to acknowledge reports within 7 days.

## Hardening notes

- `gg` shells out to `git` and `sh` for passthrough / `each` — treat repo paths and `each` commands as trusted input in your environment.
- Config is TOML loaded from the user config dir and project `.gg.toml`; do not place untrusted files there.
- `self-update` does not silently overwrite your binary; prefer signed package installs (brew/deb/rpm/cargo-dist).
