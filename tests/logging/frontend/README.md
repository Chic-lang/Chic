# Logging frontend snapshots

- `mod.rs` defines the shared `COMMANDS` list and `frontend_filter()` used by the text and JSON suites.
- `text.rs` and `json.rs` invoke `log_snapshot_test!` with snapshots stored under `snapshots/frontend_{text,json}.snap`.
- Snapshots are scrubbed by `tests/logging/harness.rs` to normalise timestamps, elapsed_ms, and temp paths.

## Regenerating snapshots

Run only the frontend cases to keep turnaround quick:

```
UPDATE_EXPECT=1 cargo test --test logging -- frontend::
```

Snapshots will be rewritten in `tests/logging/frontend/snapshots/`. Review changes before committing.
