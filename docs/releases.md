# Releases

This repo supports two release tracks:

1) **Tagged releases** (`v*` tags)
2) **Canary releases** (auto-updated from `main`)

## Tagged releases (stable)

When a tag matching `v*` is pushed, `.github/workflows/release.yml` runs and publishes a GitHub Release with:

- `chic` release binaries (packaged tarballs)
- VS Code extension artifact (`.vsix`)
- A staged Homebrew formula artifact (and optionally opens a Homebrew tap PR when secrets are configured)

Use tagged releases for stable, versioned distribution.

## Canary releases (from main)

On successful CI for `main`, CI publishes/updates a prerelease GitHub Release named/tagged `canary`.

Artifacts are intended for:
- Consumers who want \"latest\" builds
- CI workflows that want to reuse released artifacts

Current canary assets include:
- `chic-canary-<os>-<arch>.tar.gz` (+ `.sha256`)
- When `packages/**` change on `main`: `chic-<package>-canary-linux-<arch>.clrlib` (+ `.sha256`) for directly changed packages (Linux)

## Notes

- Canary releases are **prereleases** and may contain breaking changes.
- The `canary` tag is force-updated to the latest `main` SHA when publishing.
