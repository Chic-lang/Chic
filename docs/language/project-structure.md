# Project structure

Chic projects are built from a `manifest.yaml` at the package root. A repository can contain multiple packages under `packages/`, and an optional `manifest.workspace.yaml` can provide shared defaults.

## Package layout

A typical package looks like:

- `manifest.yaml` — package metadata, sources, dependencies, and toolchain/runtime selection.
- `src/` — Chic source files (`.ch`).
- `tests/` — Chic testcase files (optional).
- `docs/` — package-specific docs (optional).

For the full manifest reference, see `docs/manifest_manifest.md`.

## Building a package

From the package directory:

```sh
chic build
```

From the repository root:

```sh
chic build packages/<name>
```

## Dependencies

Dependencies are declared in `manifest.yaml` under `dependencies`. The compiler resolves names from the manifest dependency graph; packages that are not listed are not visible.

## Runtime selection (required)

Every package that builds must specify `toolchain.runtime` in `manifest.yaml`. If it is missing, `chic build` fails with an actionable error.

