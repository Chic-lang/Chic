# Build Outputs (obj/bin) and Artifact Hygiene

Chic builds produce two classes of output:

- **Intermediates** (object files, metadata sidecars, caches): written under an `obj/` tree.
- **Final artifacts** (executables/libraries/archives): written under a `bin/` tree.

By default, the workspace writes intermediates under `./obj/` in the repo root. For release-quality workflows, prefer an explicit output directory so build outputs never pollute source folders.

## CLI: `--artifacts-path`

Most `chic build/run/test` commands accept:

- `--artifacts-path <dir>`: writes intermediates under `<dir>/obj/<target>/<configuration>/<backend>/<runtime>` and linked artifacts under `<dir>/bin/...`.

Example:

```sh
./target/debug/chic build --artifacts-path ./out
```

## Repository rules

- Never commit build outputs: `obj/`, `bin/`, `tmp/`, logs, `*.o`, `*.ll`, caches.
- If you need to keep a build product for debugging, copy it to a local folder outside the repo or use `--artifacts-path`.

