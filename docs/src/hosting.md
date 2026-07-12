# Hosting this book

Published at **https://gg.chtnnhfoundation.org/** via GitHub Pages (mdBook + Actions).

## Preview locally

```bash
cargo install mdbook --locked
mdbook serve docs --open
```

## Deploy

Pushes to `main` that change `docs/` (or `.github/workflows/docs.yml`) deploy automatically. You can also run the **Docs** workflow manually.

## Custom domain DNS

Create a CNAME for `gg.chtnnhfoundation.org` → `chtnnh.github.io`, then enable HTTPS in the repo’s Pages settings once the certificate is ready.

Operator details: [`docs/README.md`](https://github.com/chtnnh/git-gist/blob/main/docs/README.md).
