# chic-lang.com website

This repo hosts the chic-lang.com website under `website/` (Next.js App Router, SSR-capable).

## Prereqs

- Node.js 20+ (CI uses Node 20)
- npm

## Local development

```sh
cd website
npm ci
npm run dev
```

Then open `http://localhost:3000`.

## Checks (required before PRs)

```sh
cd website
npm run lint
npm run typecheck
npm test
npm run test:e2e
```

- Unit tests: Vitest
- E2E + a11y smoke: Playwright + axe

## Content

### Blog

- Blog posts live in `website/content/blog/*.md`.
- Each post uses frontmatter: `title`, `date`, optional `author`, `tags`, `description`.

### Docs

- The docs area renders a curated subset of repo markdown files.
- Curated list: `website/src/content/docs.ts`
- Pages link back to the source file on GitHub.

If you add a new doc to the curated list, keep it stable (prefer existing repo files under `docs/`).

## Production build and start

```sh
cd website
npm run build
PORT=3000 npm start
```

`npm start` runs the Next.js standalone server output.

## Docker

Build from the repo root:

```sh
docker build -f website/Dockerfile -t chic-lang-com .
docker run --rm -p 3000:3000 -e NEXT_PUBLIC_SITE_URL=http://localhost:3000 chic-lang-com
```

Optional local preview with Compose:

```sh
docker compose -f website/docker-compose.yml up --build
```

Note: if your local Docker is configured with a missing credential helper, you can temporarily bypass it with:

```sh
DOCKER_CONFIG=/tmp/docker-config docker build -f website/Dockerfile -t chic-lang-com .
```

## Deployment notes

- Set `NEXT_PUBLIC_SITE_URL` so RSS links use the correct domain (defaults to `https://chic-lang.com`).
- This site is SSR-capable and is not a static export.

