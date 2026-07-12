# Docs site (mdBook)

The user guide under [`docs/`](.) is an [mdBook](https://rust-lang.github.io/mdBook/).

## Local preview

```bash
cargo install mdbook --locked
mdbook serve docs --open
# → http://localhost:3000
```

## Production

GitHub Actions (`.github/workflows/docs.yml`) builds and deploys to **GitHub Pages** on pushes to `main` that touch `docs/`.

- Site: https://gg.chtnnhfoundation.org/
- Fallback: https://chtnnh.github.io/git-gist/

### One-time GitHub setup

1. Repo **Settings → Pages → Build and deployment → Source**: **GitHub Actions**
2. After the first successful Docs workflow run, set the custom domain to `gg.chtnnhfoundation.org` (or rely on the committed `CNAME` in the Pages artifact).
3. Enable **Enforce HTTPS** once DNS has propagated.

### DNS (Cloudflare / your DNS host)

For subdomain `gg.chtnnhfoundation.org`:

| Type  | Name | Target              | Proxy |
|-------|------|---------------------|-------|
| CNAME | `gg` | `chtnnh.github.io`  | DNS only (grey cloud) recommended first |

Notes:

- Target is **`chtnnh.github.io`** (username/org Pages host), not `chtnnh.github.io/git-gist`.
- With Cloudflare orange-cloud proxy, use SSL mode **Full** (not Flexible) after GitHub issues the certificate.
- Apex domains need A/AAAA records to GitHub’s Pages IPs; this hostname is a subdomain so CNAME is enough.

### Manual deploy

```bash
gh workflow run Docs.yml
```
