# Runtime selection

Chic builds link against a selected runtime package (for example `runtime.native` or `runtime.no_std`). Runtime selection is explicit and is configured per project via `toolchain.runtime` in `manifest.yaml`.

## Manifest keys

```yaml
toolchain:
  runtime:
    kind: native           # or no_std
    package: runtime.native
    abi: rt-abi-1
    path: ../packages/runtime.native   # optional override
    policy:
      require_native_runtime: true     # defaults: exe=true, libs=false
```

- `kind` must match the runtime package’s `runtime.provides.kind`.
- `abi` must match the compiler’s expected runtime ABI tag.
- `path` is optional; when omitted, runtimes are resolved under `packages/<package>` relative to the workspace root.

For the full manifest schema (including optional keys), see `docs/manifest_manifest.md`.
