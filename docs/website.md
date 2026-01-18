# chic-lang.com website

This repo hosts the chic-lang.com website under `website/` (Next.js App Router).

## Prereqs

- Node.js 20+ (CI uses Node 20)
- npm

## i18n routing (required)

All public pages live under **locale-prefixed paths**:

- `/en-US` (default)
- `/es-ES`
- `/fr-FR`
- `/it-IT`
- `/ja-JP`
- `/pt-BR`
- `/ru-RU`
- `/tr-TR`
- `/zh-CN`

Implementation notes:

- Routes live under `website/src/app/[locale]/...`.
- `website/middleware.ts` enforces locale prefixes and redirects `/` to a locale (Accept-Language best-effort, otherwise `en-US`).
- Invalid locale segments 404.

## UI translations

UI strings come from `next-intl` message catalogs:

- `website/src/messages/<locale>.json`
- Loading logic: `website/src/i18n/messages.ts`

## Docs + blog content (MDX, per-locale)

Docs and blog are authored as **localized MDX files**:

- Docs: `website/content/docs/<locale>/**/*.mdx`
- Blog: `website/content/blog/<locale>/*.mdx`

Both support frontmatter:

- Docs: `title`, optional `description`, optional `sourcePath`
- Blog: `title`, `date`, optional `author`, `tags`, `description`

### Fallback behavior (incomplete translations)

If a localized doc/blog file does not exist yet:

- The site renders the `en-US` content **but keeps the locale in the URL**.
- A small notice is shown: “This page is not yet translated. Showing English (US).”

This is deterministic and test-covered (`website/src/i18n/fallback.test.ts`).

## Related links + contact/help blocks

Docs/blog pages support Microsoft-style blocks via frontmatter (no React code required):

```yaml
relatedLinks:
  - title: Getting started
    href: /docs/getting-started
    description: Build the chic CLI and run a first build.
contactBlock: true
```

- `relatedLinks` renders a consistent “Next steps” area.
- `contactBlock` renders a consistent “Get help / feedback” block (defaults to `true` if omitted).
- Prefer locale-agnostic internal hrefs (e.g. `/docs/...`); the renderer localizes links per request.

## SEO + canonical URLs

- Every page includes canonical + `hreflang` alternates across all supported locales (even when content falls back).
- Base URL is controlled via env vars:
  - Prefer `SITE_URL` (server-only).
  - `NEXT_PUBLIC_SITE_URL` is also supported.
  - Defaults to `https://chic-lang.com`.

## RSS

Per-locale RSS feeds:

- `/<locale>/blog/rss.xml` (e.g. `/en-US/blog/rss.xml`)

## Local development

```sh
cd website
npm ci
npm run dev
```

Then open `http://localhost:3000/en-US/`.

## Checks (required before PRs)

```sh
cd website
npm run lint
npm run typecheck
npm test
npm run test:e2e
```

- Unit tests: Vitest (includes i18n fallback + jest-axe a11y smoke across locales)
- E2E + a11y smoke: Playwright + axe

## Production build + start

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
docker run --rm -p 3000:3000 -e SITE_URL=http://localhost:3000 chic-lang-com
curl -fsS http://127.0.0.1:3000/en-US/ >/dev/null
```

Optional local preview with Compose:

```sh
docker compose -f website/docker-compose.yml up --build
```
