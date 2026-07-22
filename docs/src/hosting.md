# Hosting this book

Published at **https://gg.chtnnhfoundation.org/** via GitHub Pages (mdBook + Actions).

Fallback URL: https://chtnnh.github.io/git-gist/ (redirects to the custom domain once it is configured).

## Preview locally

```bash
cargo install mdbook --locked
mdbook serve docs --open
```

## Deploy

Pushes to `main` that change `docs/` (or `.github/workflows/docs.yml`) deploy automatically via the **Docs** workflow. Manual:

```bash
gh workflow run docs.yml
```

## Custom domain DNS

| Type | Name | Target | Proxy |
|------|------|--------|-------|
| CNAME | `gg` | `chtnnh.github.io` | DNS only (grey cloud) first |

- Target is **`chtnnh.github.io`**, not `…/git-gist`.
- After GitHub shows the domain verified and the certificate approved, enable **Enforce HTTPS**.
- If Cloudflare orange-cloud is enabled later, use SSL mode **Full** (not Flexible).

### “DNS looks fine but the site won’t load”

1. Confirm `dig +short gg.chtnnhfoundation.org` → `chtnnh.github.io.` and GitHub Pages IPs.
2. Flush local DNS cache (macOS: `sudo dscacheutil -flushcache; sudo killall -HUP mDNSResponder`).
3. Remember `chtnnh.github.io/git-gist` **redirects** to the custom domain — if the custom name fails, both URLs appear broken.
4. Try another network or a private browser window.

Operator notes: [`docs/README.md`](https://github.com/chtnnh/git-gist/blob/main/docs/README.md).

## Analytics

Cloudflare Web Analytics is injected via [`theme/head.hbs`](https://github.com/chtnnh/git-gist/blob/main/docs/theme/head.hbs) on every page of the built book.
