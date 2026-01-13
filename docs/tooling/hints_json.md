# Hints JSON Artifact

`chic build --emit hints.json` produces a structured list of obligations, suggestions, and
machine-actionable fix-its discovered during compilation. Agents consume this artifact to satisfy
typed program holes, complete partially-specified APIs, and reason about outstanding work.

## File Layout

```json
{
  "version": "0.1.0",
  "obligations": [
    {
      "id": "hole://src/service.cl:87#0",
      "kind": "TypedHole",
      "type": "bool",
      "constraints": ["must_call(LogAudit)"],
      "candidates": [
        { "symbol": "Auth::Allow", "score": 0.92 },
        { "symbol": "Auth::CheckAcl", "score": 0.88 }
      ]
    }
  ],
  "diagnostics": [
    {
      "code": "SHAPE001",
      "message": "Shape mismatch: expected [Dim<B>, Dim<H>], found [Dim<B>, Dim<H/2>]",
      "suggestions": [
        "insert call to Tensor::broadcast(axis=1, factor=2)",
        "adjust schedule tile width to preserve dimension"
      ]
    }
  ]
}
```

- `obligations` enumerates every outstanding requirement the compiler detected (typed holes,
  unverifiable contracts, trait obligations awaiting implementation, etc.).
- `diagnostics` mirrors compiler warnings that include structured fix-its.

## Identifiers

- Obligation IDs follow the form `category://file:line#ordinal`. They remain stable across
  rebuilds as long as the source span and AST structure do not change.
- Candidates include a `score` field to hint at the compiler’s preferred fix, but agents should
  treat the list as unordered.

## Integration Notes

- The schema is intentionally minimal and does not duplicate full diagnostics. Consumers should
  join data with `mir.json` when richer context (e.g., effect sets) is required.
- Future revisions will add provenance for suggested transformations (e.g., “insert `TensorView`
  before `TensorCopy`”) once the MIR diffing API stabilises.
