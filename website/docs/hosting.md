# Hosting this site

Published at **https://gg.chtnnhfoundation.org/** via GitHub Pages (Docusaurus + Actions).

Fallback URL: https://chtnnh.github.io/git-gist/ (redirects to the custom domain once it is configured).

## Preview locally

```bash
cd website
npm install
npm start
# → http://localhost:3000
```

Production build:

```bash
cd website
npm run build
npm run serve
```

## Deploy

Pushes to `main` that change `website/` (or `.github/workflows/docs.yml`) deploy automatically via the **Docs** workflow. Manual:

```bash
gh workflow run docs.yml
```

## Versioned docs

- `/` — latest **released** freeze (`versions.json` / `versioned_docs/`)
- `/head` — unreleased (`docs/`); switch via the navbar dropdown
- Freeze before tagging: `./scripts/docs-version.sh` from the repo root

## Custom domain DNS

| Type | Name | Target | Proxy |
|------|------|--------|-------|
| CNAME | `gg` | `chtnnh.github.io` | DNS only (grey cloud) first |

- Target is **`chtnnh.github.io`**, not `…/git-gist`.
- After GitHub shows the domain verified and the certificate approved, enable **Enforce HTTPS**.
- For the Umami `/stats` Worker route, DNS must be **proxied** (orange cloud) and SSL mode **Full** (not Flexible). Grey-cloud (DNS only) is fine only until you enable the proxy.

### “DNS looks fine but the site won’t load”

1. Confirm `dig +short gg.chtnnhfoundation.org` → `chtnnh.github.io.` and GitHub Pages IPs.
2. Flush local DNS cache (macOS: `sudo dscacheutil -flushcache; sudo killall -HUP mDNSResponder`).
3. Remember `chtnnh.github.io/git-gist` **redirects** to the custom domain — if the custom name fails, both URLs appear broken.
4. Try another network or a private browser window.

Operator notes: [`website/README.md`](https://github.com/chtnnh/git-gist/blob/main/website/README.md).

## Analytics

[Umami](https://umami.is/) is self-hosted at `https://umami.chtnnhfoundation.org`. Production pages load the tracker **first-party** from `/stats/*` via a Cloudflare Worker ([`workers/umami-proxy`](https://github.com/chtnnh/git-gist/tree/main/workers/umami-proxy)), so requests stay on `gg.chtnnhfoundation.org`.

### Operator setup

1. In Umami, add a website for **`gg.chtnnhfoundation.org`** and copy its website ID (UUID).
2. Repo **Settings → Secrets and variables → Actions**:
   - `UMAMI_WEBSITE_ID` — website UUID (Docs workflow)
   - `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ACCOUNT_ID` — deploy the proxy Worker
3. DNS for `gg` must be **proxied** (orange cloud). SSL/TLS mode **Full**.
4. Deploy the Worker once: `cd workers/umami-proxy && npm ci && npx wrangler deploy`  
   (or push to `workers/umami-proxy/**` / run workflow **Umami proxy**).
5. Redeploy docs (`gh workflow run docs.yml` or push to `website/`).

Optional local preview **without** the Worker (hits Umami directly):

```bash
cd website
UMAMI_WEBSITE_ID=<uuid> UMAMI_DIRECT=1 npm start
```

Builds without `UMAMI_WEBSITE_ID` skip the script. Cloudflare Web Analytics was removed with the mdBook migration.

