# TODO

Short, near-term work items that aren’t tracked as GitHub issues yet.

## Rules

- Keep this list small (prefer issues for anything non-trivial).
- Each item should become a GitHub issue once someone is ready to pick it up.

## Open items

- [ ] Coverage TODO list (open a tracking issue)
  - Generate a fresh report (`cargo xtask coverage --min 95 --output coverage/coverage.json`).
  - Create/update `tests/TODO.md` with modules below the threshold + a short maintenance note.
  - Ensure the docs reference `tests/TODO.md` as the canonical coverage worklist.

- [ ] Self-hosting readiness inventory (open a tracking issue)
  - Catalogue bootstrap-only features used in `src/**` (macros, unsafe, trait bounds, async, etc.).
  - Map each feature to Chic equivalents or gaps, linking to the spec where relevant.
  - Capture baseline regressions targets (size/perf/coverage) and keep them updated.
  - Define cutover criteria for a Chic-hosted toolchain (what “good enough” means).
  - Stand up parity tests/harnesses (CLI smoke, parser/MIR snapshots) to detect drift.
  - Identify components that need early Chic implementations (runtime primitives + core std surface).
