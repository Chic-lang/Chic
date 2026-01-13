# Manifest (`manifest.yaml`)

`manifest.yaml` defines a Chic package: its sources, dependencies, build output kind, and required toolchain/runtime selection.

## File Location

- The manifest lives at the package root as `manifest.yaml`.
- Package roots live under `packages/<name>/manifest.yaml`; nested manifests (for example `packages/<name>/src/manifest.yaml`) are not supported.
- `chic build`, `chic run`, and `chic test` will look for `manifest.yaml` in the current directory if no path is provided. Passing a directory will search for `manifest.yaml` inside it; passing `manifest.yaml` explicitly selects that project file.
- Workspaces may contain multiple packages; a workspace manifest (`manifest.workspace.yaml`) can provide shared defaults.

## CLI Usage

- `chic build` — builds the `manifest.yaml` in the current directory.
- `chic build manifest.yaml -c Release` — explicit project file.
- `chic build path/to/project --framework wasm32` — directory argument; discovers `path/to/project/manifest.yaml`.
- `chic run manifest.yaml` — build + run a specific project.
- `chic test` — builds and runs tests for the current project (automatically includes `tests/` when present).
- `chic init --template app MyApp` — scaffolds a console application with `manifest.yaml`, `src/`, `tests/`, docs, and a CI stub.

## Minimal example

```yaml
package:
  name: geometry
  namespace: Geometry   # default namespace prefix for inferred modules

build:
  kind: exe             # exe (default) or lib

sources:
  - path: src
    namespace_prefix: Geometry

dependencies:
  logging:
    path: ../logging    # local path dependency

toolchain:
  runtime:
    kind: native
    package: runtime.native
    abi: rt-abi-1
    path: ../packages/runtime.native
```

### Sections

- `package`: Required metadata. `namespace` seeds the inferred namespace for files under `sources`.
- `build`: Output kind (`exe` or `lib`) and build-related settings.
- `sources`: Lists source roots and their namespace prefixes. Omitting `sources` defaults to a single `{ path: "src", namespace_prefix: package.namespace }` entry.
- `dependencies`: Maps logical dependency names to local paths or other supported sources.
- `toolchain.runtime`: Required runtime selection used by the build.

### Runtime Selection

- Every manifest must declare `toolchain.runtime`. Builds fail if the runtime is omitted.
- `kind` chooses `native` vs `no_std`; `package` identifies the runtime package (defaults live under
  `packages/runtime.*`).
- `abi` selects the runtime ABI tag (current expectation: `rt-abi-1`); mismatches fail fast with an actionable diagnostic.
- `path` overrides runtime package discovery.

Keep this document aligned with `SPEC.md` whenever the manifest format evolves.
