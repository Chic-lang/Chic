# CI/CD

This repo uses GitHub Actions (`.github/workflows/ci.yml`) with **draft PR gating** and **path-based targeting** to keep CI fast while preserving required check contexts on non-draft PRs.

## Draft PRs vs Ready for review

- Draft PRs run a **lightweight** job: `PR (draft) lightweight checks`.
  - Rust/Chic: `cargo fmt -- --check` (only when Rust/Chic-related paths changed)
  - Website: lint + typecheck (only when `website/**` changed)
  - VS Code: lint (only when `chic-vscode/**` changed)
- When a PR is marked **Ready for review**, the full required PR checks run.

The workflow listens for PR events including `ready_for_review`, so converting Draft → Ready triggers a full run.

## Path targeting

The `Changed paths` job uses `dorny/paths-filter` to classify changes and gates jobs accordingly.

At a high level:

- Docs-only changes avoid heavy compilation/test work.
- Website-only changes run website checks and docker smoke, while Rust-required jobs **no-op** quickly.
- Changes under `packages/**` run targeted Chic package builds (and best-effort tests) for impacted packages.
- Rust compiler / CI changes run the full Rust-required checks.

Note: Some jobs (e.g. `build`, `native-runtime`, `metrics`, `vscode`) are required by main branch protection. For non-draft PRs, these jobs must still report a successful check context even when work is not relevant, so they may run a fast \"Skipped\" step.

## Local verification

### Rust (mandatory loop)

```sh
cargo fmt -- --check
cargo build --all --all-targets
cargo test --all --all-targets --no-run
```

### Chic packages (best-effort)

```sh
./target/debug/chic build packages/<pkg>
./target/debug/chic test packages/<pkg>
```

### Website

```sh
cd website
npm ci
npm run lint
npm run typecheck
npm test
npm run build
PORT=3000 npm start
```

Docker smoke (from repo root):

```sh
docker build -f website/Dockerfile -t chic-lang-com:ci .
docker run --rm -p 3000:3000 -e NEXT_PUBLIC_SITE_URL=http://localhost:3000 chic-lang-com:ci
```

## Repo settings

Auto-delete merged branches is controlled by the repository setting `delete_branch_on_merge`.

To enable (requires permissions):

```sh
gh api -X PATCH repos/Chic-lang/Chic -f delete_branch_on_merge=true
```

