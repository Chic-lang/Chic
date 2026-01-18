FROM node:20-bookworm-slim AS deps
WORKDIR /app/website

COPY website/package.json website/package-lock.json ./
RUN npm ci

FROM node:20-bookworm-slim AS builder
WORKDIR /app

COPY website /app/website
COPY docs /app/docs
COPY SPEC.md README.md /app/

COPY --from=deps /app/website/node_modules /app/website/node_modules
RUN cd /app/website && npm run build

FROM node:20-bookworm-slim AS runner
ENV NODE_ENV=production
ENV PORT=3000
ENV HOSTNAME=0.0.0.0

# Copy the standalone server output into /app/website
WORKDIR /app/website
COPY --from=builder /app/website/.next/standalone/website ./
COPY --from=builder /app/website/.next/static ./.next/static
COPY --from=builder /app/website/public ./public

# Copy the curated docs sources into /app so server-side doc rendering works.
WORKDIR /app
COPY --from=builder /app/docs ./docs
COPY --from=builder /app/SPEC.md ./SPEC.md
COPY --from=builder /app/README.md ./README.md
COPY --from=builder /app/website/content ./website/content

WORKDIR /app/website
EXPOSE 3000
CMD ["node", "server.js"]

