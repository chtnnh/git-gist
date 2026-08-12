# Docs site (Docusaurus)

User guide for **git-gist** (`gg`), published at https://gg.chtnnhfoundation.org/.

## Local preview

```bash
npm install
npm start
```

## Production build

```bash
npm run build
npm run serve
```

## Versioning

- **Released** docs are frozen under `versioned_docs/` and listed in `versions.json`. The newest release is the default at `/`.
- **HEAD** (unreleased) is `docs/`, served at `/head` — switch via the version dropdown.
- Before tagging `vX.Y.Z`: `./scripts/docs-version.sh` from the repo root, then commit the freeze.
- Optional local gate: `git config core.hooksPath .githooks` (rejects tag pushes without a matching freeze).

## Analytics

Production builds inject Umami **first-party** (`/stats/script.js` + `data-host-url=/stats`) when `UMAMI_WEBSITE_ID` is set. The Cloudflare Worker in [`workers/umami-proxy`](../workers/umami-proxy) proxies that path to Umami (see [Hosting → Analytics](./docs/hosting.md)).

## Deploy

GitHub Actions (`.github/workflows/docs.yml`) builds `website/` and deploys to **GitHub Pages** on pushes to `main` that touch `website/` (or the workflow file).

- Site: https://gg.chtnnhfoundation.org/
- Fallback: https://chtnnh.github.io/git-gist/
- Custom domain file: `static/CNAME`

### One-time GitHub setup

1. Repo **Settings → Pages → Build and deployment → Source**: **GitHub Actions**
2. Enable **Enforce HTTPS** once DNS has propagated for `gg.chtnnhfoundation.org`.

### Manual deploy

```bash
gh workflow run docs.yml
```
