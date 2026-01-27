# {{project_name}}

This is a Chic console application generated via `chic init --template app`.

## Layout

- `manifest.yaml` — project metadata and build settings (`build.kind: exe` for executables).
- `src/App.ch` — entrypoint with `Main(string[] args)` plus a helper for argument/default handling.
- `tests/AppTests.ch` — sample testcases demonstrating how to exercise pure functions.
- `docs/` — space for project documentation.
- `.github/workflows/ci.yml` — optional CI scaffold that runs `chic build` and `chic test`.

## Build, Run, Test

- `chic build` — compile the application.
- `chic run` — build and execute `Main`.
- `chic test` — discover and run testcases.

## Customisation

- Adjust `package.name` or `build.kind` in `manifest.yaml` as needed.
- Add additional source files under `src/` and testcases under `tests/`.
- Extend `Main` with argument parsing or logging as your app grows.
- Swap out or extend the CI workflow if you provision the `chic` toolchain differently.
