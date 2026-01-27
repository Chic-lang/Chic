# App Template Structure (`chic init --template app`)

- **Purpose:** Scaffold a runnable Chic console application without external shims.
- **Files:**
  - `manifest.yaml` — sets `package.name: {{project_name}}` and `build.kind: exe`.
  - `src/App.ch` — `Main(string[] args)` prints a greeting via `Std.Console` and uses a pure `SelectTarget` helper for args/default handling.
  - `tests/AppTests.ch` — sample `testcase` coverage for the helper to demonstrate testing patterns.
  - `README.md` — quick usage + layout summary.
  - `docs/README.md` — placeholder for project documentation.
  - `.github/workflows/ci.yml` — optional GH Actions job running `chic build` and `chic test` (configurable via `CHIC_BIN`).
- **Placeholders:** `{{project_name}}` (manifest + docs) and `{{project_namespace}}` (namespaces) default to the output directory name or `MyApp` when unspecified. Namespace tokens are sanitised to valid identifiers.
- **Layout expectations:** Sources live in `src/`, tests in `tests/`, docs under `docs/`. The manifest follows existing Chic conventions (`package` + `build` sections; `build.kind` aliases `exe`/`app`/`bin`).
