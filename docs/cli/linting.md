## Chic Linting (`chic lint`)

`chic lint` runs the Clippy-style lint engine over Chic sources. The same lints also run during `chic check/build/test`, but the `lint` command fails the build when any error-level lint is emitted.

### Categories and defaults
- Categories: `style` (warn), `correctness` (error), `perf` (warn), `pedantic` (allow).
- Rule defaults override categories when present (e.g., `dead_code` is an error by default).
- Levels: `allow|warn|error|deny|off` (case-insensitive, `deny` == `error`).

### Configuration discovery (YAML)
- Files: `lint.yaml`, `chiclint.yaml`, or `manifest.yaml` with a top-level `lint:` section.
- Discovery walks upward from the first input’s directory, applying root-most layers first (workspace overrides beat workspace defaults).
- `extends` allows explicit inheritance inside a config file (`extends: ../lint.base.yaml`).
- `CHIC_LINT_CONFIG=<path>` appends an override layer after discovery.
- Schema:

```yaml
lint:
  categories:
    style: warn
    correctness: error
    perf: warn
    pedantic: allow
  rules:
    dead_code: error
    unused_param: warn
```

### Suppression and precedence
- `@allow(<lint>|<category>|all)` can be attached to namespaces, classes/structs/impls/extensions, functions/constructors/testcases, or individual parameters.
- Attribute aliases: `@dead_code` or `@unused_param` are equivalent to `@allow(dead_code)` / `@allow(unused_param)`.
- Precedence: nearest `@allow` wins (parameter → function/ctor/method → container/namespace → config). Unrecognised allows are ignored.

### Lint catalogue (initial set)
- `dead_code` (`LINT001`, correctness, error): flags user-defined functions/constructors/testcases that are unreachable from public APIs, exports, stdlib entry points, or explicit calls. Suggests removing the item or marking it with `@allow(dead_code)`.
- `unused_param` (`LINT002`, style, warn): flags unused parameters and suggests prefixing them with `_` or adding `@allow(unused_param)`. Parameters already prefixed with `_` are ignored.

### CLI behaviour
- `chic lint <file>`: prints lint diagnostics (with fix hints) and exits non-zero on any error-level lint.
- `chic check/build/run/test`: lints run automatically; diagnostics are printed alongside parse/type/MIR output. Set `CHIC_DIAGNOSTICS_FATAL=1` to fail the command on any diagnostics, including lints.
