# Chic Init – App Template

`chic init --template app` scaffolds a runnable console application with a `Main` entrypoint, manifest, tests, docs, and CI stub.

## Quickstart

1. Create the project: `chic init --template app MyApp` (use `--name` to override the embedded project name).
2. Inspect `manifest.yaml` to confirm `build.kind: exe` and the package name/namespace.
3. Build and run:
   - `chic build`
   - `chic run`
4. Execute tests: `chic test` (discovers `tests/` automatically).

## Generated Layout

- `manifest.yaml` — package metadata (`package.name`) and executable build kind (`build.kind: exe`).
- `src/App.cl` — `Main(string[] args)` writes a greeting and delegates to `SelectTarget` for argument/default handling.
- `tests/AppTests.cl` — sample `testcase` coverage for the helper.
- `README.md` — usage instructions.
- `docs/README.md` — placeholder for project docs.
- `.github/workflows/ci.yml` — GitHub Actions job running `chic build` and `chic test` (expects `chic` on `PATH` or via `CHIC_BIN`).

Placeholders (`{{project_name}}`, `{{project_namespace}}`) default to the output directory name; namespaces are sanitised to valid identifiers when the name contains separators.

## Customisation

- Rename the package or namespace in `manifest.yaml` to control default source roots and inferred namespaces.
- Extend `Main` with argument parsing or logging; keep the return type `int` to propagate exit codes.
- Add new source files under `src/` and mirror test coverage under `tests/`.
- Update the CI workflow to match your toolchain provisioning (pinning `CHIC_BIN` or downloading a release).

## Troubleshooting

- **Missing/renamed `Main`:** Executable builds expect a visible `Main` entrypoint. Removing or renaming it will surface a startup descriptor error at build/run time.
- **Multiple entrypoints:** Only one `Main` is supported per executable crate today. Split executables into separate manifests if you need multiple apps.
- **CI/toolchain availability:** The scaffold assumes `chic` is available. Set `CHIC_BIN` in the workflow or add an installation step to fetch your preferred build of the toolchain.
