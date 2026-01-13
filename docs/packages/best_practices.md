# Package best practices

Practical guidance for Chic package authors and consumers.

## For package authors

- Align namespaces with `manifest.yaml` (`package.namespace` or `package.name`) and keep `@friend("...")` prefixes narrow and explicit.
- Keep exports intentional: only mark APIs `@export` when they are meant to be public.
- Keep manifests complete: `package.name`, `package.namespace`, `sources`, and explicit `dependencies`.

## For package consumers

- Declare every package you import under `dependencies:` so builds are reproducible.
- Prefer local `path` dependencies during development.
- Watch build diagnostics for missing packages, conflicts, and cycles.

## Workspaces

- Multi-package workspaces compose via path dependencies; keep dependency graphs shallow when possible.
- Keep public APIs explicit and intentional; avoid exporting symbols you do not mean to support.
