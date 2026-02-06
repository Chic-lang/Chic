# Git Hook Quality Gates

This repository uses repo-managed hooks under `.githooks/`.

## Install

Run once per clone:

```bash
python3 scripts/git_hooks/install.py
```

This sets `core.hooksPath=.githooks`.

## What runs

### pre-commit

Runs formatter/linter gates for each detected stack:

- Rust: `cargo fmt --all -- --check` plus CI-aligned `cargo xtask lint-*` guards (`lint-ll1`, `lint-runtime-shim`, `lint-shim-state`, `lint-stdlib-rust-tests` are blocking; `lint-runtime-calls`/`lint-runtime-symbols` are warning-only like CI)
- Node projects (`**/package.json`): optional format-check script (`format:check`/`fmt:check`/`prettier:check`/`style:check`), `npm run lint`, optional `npm run typecheck`
- .NET solutions/projects: `dotnet format <target> --verify-no-changes`

### pre-push

Runs, in order:

1. Branch/PR policy checks
2. The same formatter/linter gates as `pre-commit`
3. Dependency freshness checks
4. Tests + coverage gate (`>=95%` line coverage)

Dependency freshness checks:

- Rust: `cargo outdated --workspace --exit-code 1`
- Node: `npm outdated --json`
- .NET: `dotnet list <sln|csproj> package --outdated --include-transitive --format json`

Coverage gate:

- Chic: `chic test --workspace` and `chic coverage --workspace --min 95`
- Rust: `cargo llvm-cov --workspace --all-targets --summary-only --json`
- Node: expects `coverage/coverage-summary.json` with `total.lines.pct`
- .NET: parses Cobertura output from `dotnet test --collect:XPlat Code Coverage`

Timeout tuning (optional):

- `CHIC_TEST_TIMEOUT_SEC` (default `900`)
- `CHIC_COVERAGE_TIMEOUT_SEC` (default `900`)

If any check fails, push is denied with actionable failure output.
