# Native runtime package

The native runtime is provided by the `runtime.native` package under `packages/runtime.native/`. The compiler links against the selected runtime archive when building executables.

## Where it lives

- Runtime sources: `packages/runtime.native/src/`
- Runtime manifest: `packages/runtime.native/manifest.yaml`

## How it is selected

Runtime selection is configured in each project’s `manifest.yaml` under `toolchain.runtime`. See `docs/tooling/runtime_selection.md`.

## Outputs

Build artifacts are written under the configured artifacts root (default: `./obj` and `./bin`) and are partitioned by target, configuration, backend, and runtime identity.
