# Record types

Record structs are value-semantic aggregates with concise declaration syntax.

## Syntax

```chic
namespace Demo;

public record struct Point(int X, int Y);
```

## Semantics

- Record structs are value types and have struct-like layout.
- Fields are immutable by default; assignment happens during construction.
- Equality and hashing follow the record’s declared fields (see `SPEC.md` for the full rules).

## Reference

- Language spec: `SPEC.md`
