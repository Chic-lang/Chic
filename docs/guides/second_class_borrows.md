Chic's borrow qualifiers (`in`, `ref`, `out`) are **second-class**: they exist solely to
describe how parameters and receivers are passed. The compiler no longer accepts these qualifiers
on locals, fields, pattern bindings, or return types. This guide outlines the updated behaviour,
and the diagnostics you may see when borrows escape their intended scope.

## Overview

- `in/ref/out` are valid only on parameters and receivers.
- Borrows are call-scoped temporaries: they are created for a call and end when that call ends.
- The type checker emits **`[CL0031] borrow escapes scope`** when a borrowed parameter is returned, stored, or captured by a closure/async state machine.

## Diagnostic reference

```
[CL0031] borrowed `ref` parameter `value` escapes from `Borrow::Return` by returning it
  --> samples/borrow.ch:6:16
   |
 6 |     return value;
   |                ^
   |
note: consider returning an owned value instead, for example by copying or cloning the data
  --> samples/borrow.ch:6:16
```

The diagnostic appears in three scenarios:

1. Returning a borrowed parameter or receiver.
2. Storing a borrowed parameter into a field, struct, local, or static.
3. Capturing a borrowed parameter inside a closure or async lambda.

Each note proposes an owned alternative (copy/clone, storing an owned value, or capturing a clone).

## Common fixes

- **Replace stored borrows** with owned copies. For example, change
  `cache.Store(ref value);` to `cache.Store(value.Clone());` or redesign the API to accept `in`
  by value.
- **Return owned values** from helper methods instead of forwarding the borrow, or convert the
  helper into a generic that operates on `in`/`ref` parameters supplied by the caller.
- **Capture clones in closures** (`let captured = value.Clone(); return () => captured;`) so the
  borrow does not leave the call frame.
- **Remove field/local qualifiers.** `ref string _cached;` should become either a value field or a
  reference stored inside a wrapper type that manages the lifetime explicitly.

## Further reading

- [Chic Specification – Borrow Lifetimes](../../SPEC.md#32-borrow-lifetimes)
- [MIR Design Notes – Pattern Binding Semantics](../mir_design.md)
