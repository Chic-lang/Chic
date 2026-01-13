# Chic Formatter

The `chic format` command rewrites Chic source files deterministically. It
honours project settings from `manifest.yaml` by default and can be driven
directly from the CLI for ad-hoc checks or editor integration.

## CLI usage

- `chic format [<file|dir> ...]` — format specific paths (recurses into
  directories).
- `chic format` — format sources discovered from `manifest.yaml` in the current
  directory.
- `chic format --check` — do not write changes; exit non-zero if reformatting is
  needed.
- `chic format --diff` — print a unified diff for files that would change.
- `chic format --stdin [--stdout]` — format a snippet provided on stdin.
- `chic format --config <path>` — load formatter settings from a specific YAML
  file instead of `manifest.yaml`.

When `--write` is enabled (default) the formatter updates files in place. Use
`--check`/`--diff` to integrate with CI without modifying the working tree.

## Manifest configuration

Formatter settings live under the `format:` key in `manifest.yaml`. All fields
are optional; unspecified values fall back to deterministic defaults.

```yaml
format:
  version: 1
  enabled: true
  enforce: warn              # off | warn | error
  max_line_length: 120
  indent:
    size: 4
    use_tabs: false
  newline: lf                # lf | crlf
  trailing_newline: true
  trim_trailing_whitespace: true

  braces:
    style: allman            # allman | k&r
    require_for_single_line_if: false
    require_for_single_line_loops: false

  if:
    else_on_new_line: true
    space_before_parentheses: true
    wrap_conditions: if_long # never | if_long | always

  switch:
    case_indent: 1
    braces_style: allman
    blank_line_between_cases: false
    align_case_labels: true

  usings:
    sort: true
    group: system_first      # none | system_first | std_first | custom
    blank_line_between_groups: true

  ordering:
    access: [public, internal, protected, private]
    # type/member ordering lists are accepted for future sorting rules.

  files:
    one_top_level_type_per_file: true
    require_filename_match: true
    action: suggest          # suggest | apply
    naming: TypeName         # TypeName | namespace+TypeName
```

Defaults favour an Allman-style brace layout with four-space indentation and LF
newlines. If formatting is disabled (`enabled: false`), `chic format` acts as a
no-op.

## Enforcement in build/check/test

- When a `manifest.yaml` is present and `format.enforce` is not `off`, `chic
  check`, `chic build`, `chic run`, and `chic test` run a `--check` pass before
  compiling.
- Default severity is `warn` locally and `error` when `CI`/`CHIC_CI` is set
  (via `format.enforce` in the default config). Set `enforce: off` or
  `enabled: false` to disable.
- Violations surface as `FMT0001` diagnostics (category `format`) with a span
  covering the file and a suggestion to run `chic format`. File-organisation
  issues emit `FMT0100` (multiple top-level types) and `FMT0101` (filename
  mismatch) when the corresponding options are enabled.
- Ordering rules emit `FMT0200` (type ordering) and `FMT0201` (member ordering)
  when the configured order is not respected.
- With `enforce: error`, the command fails before compilation; `warn` emits
  diagnostics but still completes the build/test/run.

## Behaviour guarantees

- Deterministic output for a given input + configuration.
- Idempotent: running `chic format` twice yields the same output.
- Comments and XML doc comments are preserved alongside the surrounding code.
- Trailing whitespace and newline style are normalised according to the
  manifest.

File-organisation and ordering knobs are parsed and validated today; additional
enforcement and fixers will light up behind these switches as they land.
