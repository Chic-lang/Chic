# Troubleshooting

This page lists common setup and build issues when working with the bootstrap toolchain.

## `toolchain.runtime must be specified in manifest.yaml`

Every package build requires an explicit runtime selection. Add `toolchain.runtime` to your `manifest.yaml`.

See: `docs/manifest_manifest.md` and `docs/tooling/runtime_selection.md`.

## `clang` is missing

Some native/LLVM workflows require `clang` to be available on your machine.

## Import errors (`using` directives)

Chic uses `import` directives. `using` directives are not supported.

See: `docs/guides/using_directives.md`.

## Next steps

- Getting started guide: `docs/getting-started.md`
- CLI overview: `docs/cli/README.md`

