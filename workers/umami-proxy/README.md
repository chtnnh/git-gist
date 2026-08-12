# First-party Umami proxy for https://gg.chtnnhfoundation.org/

Proxies `https://gg.chtnnhfoundation.org/stats/*` → `https://umami.chtnnhfoundation.org/*`
so the docs tracker stays same-origin (harder for ad blockers to strip).

## Prerequisites

1. DNS for `gg.chtnnhfoundation.org` is **proxied** (orange cloud) on Cloudflare.
2. SSL/TLS mode **Full** (GitHub Pages origin is HTTPS).
3. Umami website exists for `gg.chtnnhfoundation.org`.

## Deploy

```bash
cd workers/umami-proxy
npm install
npx wrangler deploy
```

CI: workflow **Umami proxy** (`.github/workflows/umami-proxy.yml`) on pushes to
`workers/umami-proxy/**`. Needs secrets `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ACCOUNT_ID`.

## Local

```bash
npm run dev
# then hit http://127.0.0.1:8787/stats/script.js
```
