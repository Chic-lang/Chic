# Statement Grammar Tests

The `statements` test module is intentionally split into focused siblings so
each statement construct stays small and easy to extend:

| Module | Coverage |
| ------ | -------- |
| `conditionals.rs` | `if`/`return`/`yield`/testcases |
| `loops.rs` | `for`/`foreach`/iterator-specific statements |
| `locals.rs` | local declarations/functions + attribute validation |
| `resources.rs` | `region`, `using`, `fixed`, `atomic`, and `unsafe` blocks |
| `exceptions.rs` | `try`/`catch`/`finally` flow plus filters |
| `switches.rs` | `switch` sections, labels, guards, and `goto case` |
| `telemetry.rs` | Recovery telemetry toggling/verification |

## Adding a new test

1. Pick the module that matches the statement you are exercising (for example,
   new `foreach` fixtures belong in `loops.rs`, while new `goto` behaviors go
   into `switches.rs`).
2. Use `FunctionFixture` from `tests/grammar/common.rs` to parse the source and
   access the statement list without repeating boilerplate. Only reach for
   `parse_ok` directly when asserting on non-function items (e.g., testcases).
3. Prefer precise AST assertions (`StatementKind`, `SwitchLabel`, guards, etc.)
   over broad pattern matches so regressions pinpoint the affected construct.
4. If the test needs telemetry, wrap it with `telemetry_guard()` from
   `common.rs` to avoid cross-test interference.
5. Update this table if your new coverage area does not fit an existing module.
