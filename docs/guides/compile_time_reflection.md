# Compile-Time Reflection & Quasiquotes

Chic ships a native reflection surface so macros and const-evaluated helpers can reason about source code without leaving the language.

## Inspecting Types with `reflect<T>()`

```chic
public const Std.Meta.TypeDescriptor VecOfInts =
    Std.Meta.Reflection.reflect<Std.Collections.Vec<int>>();

public const bool VecIsPublic = VecOfInts.Visibility == Std.Meta.VisibilityDescriptor.Public;
public const string FirstField = VecOfInts.Members.Head.Head.Name;
public const ulong VecTypeId = VecOfInts.TypeId ?? 0uL;
public const Std.Meta.TypeHandle Handle = typeof(Std.Collections.Vec<int>);
```

`Std.Meta.Reflection.reflect<T>()` executes inside CTFE and emits a `Std.Meta.TypeDescriptor` for any `public` type that opted into metadata. The descriptor is immutable, allocator-free, and available on both LLVM/WASM backends. Non-public or metadata-free items trigger deterministic diagnostics; value arguments are rejected. Use `typeof(T)` when you only need a stable `Std.Meta.TypeHandle`.

### Descriptor shape (v2)

- Identity: `Namespace`, `Name`, `FullName`, `TypeId`, `Visibility`, `Kind`, and `IsGeneric`.
- Type surface: ordered `GenericArguments`/`Bases` (both `TypeHandle` lists), `Attributes`, optional `UnderlyingType` (enums), and the ordered `Members` list.
- Members/parameters: `MemberDescriptor` discriminates `Field`/`Property`/`Method`/`Constructor`/etc. with detail payloads (field types + offsets, property accessors, method flags, constructor flavour). Parameters capture the `ParameterType`, `Mode` (`value`/`in`/`ref`/`out`), optional default value, and attributes.
- Layout: `Layout` carries computed `Size`, `Align`, and per-field descriptors (`Name`, `Offset`, `Type`, `IsReadonly`). Collections remain `Std.Meta.DescriptorList<T>` to avoid hidden allocations in CTFE.

### Sidecar and stability

- Every build emits `<artifact>.reflect.json` next to the primary output and inside `.clrlib` archives. The current schema `version` is `2`; ordering of `types` and `members` is deterministic and matches the in-memory descriptor layout exposed through `@__chic_reflection` / `chic.reflect`.
- Reflection is read-only: it cannot add or mutate types at runtime, and the LL(1) grammar never reserves primitive names or contextual keywords to make reflection work. Accessor thunks are generated only when safe (no boxing, mutability obeyed); otherwise the descriptor exposes metadata only.

## Capturing Syntax with `quote(expr)`

`quote(expr)` snapshots Chic syntax into the `Std.Meta.Quote` struct:

```chic
public const Std.Meta.Quote Deferred =
    quote(query.Where(x => ${quote(x % 2 == 0)}));
```

Every quote records:

- `Source` and `Sanitized` strings (the sanitized form replaces `${...}` with placeholders such as `__chic_quote_slot0`);
- `Span` (absolute byte offsets) and `Hygiene` (anchor + deterministic seed);
- `Captures`, a deduplicated list of identifier references inside the sanitized tree;
- `Interpolations`, each a `{ Placeholder, Value: Quote, Span }` triplet; and
- `Root: QuoteNode`, a tree of `QuoteNodeKind` values that mirrors the sanitized AST.

Interpolation expressions must evaluate to another `Std.Meta.Quote`. The compiler reports a CTFE diagnostic when a different type is returned, preventing bogus splice points.

Use the `Root` tree plus `Captures`/`Interpolations` to build macros entirely in Chic—no Rust glue required.

## Macro Expansion Guardrails

Attribute and derive macros now expand until a fixed point is reached. Work that injects additional macros (for example, an attribute emitting a helper struct decorated with `@derive(Equatable)`) is automatically reprocessed in the same compilation session. The engine halts after 32 passes and reports `macro expansion exceeded … passes` if a macro continually re-attaches itself or otherwise fails to converge, preventing runaway compilation.

For practical guidance, pair this document with `SPEC.md` §6, which details every field exposed by `Std.Meta.TypeDescriptor`, `Std.Meta.Quote`, `QuoteNode`, and the fixed-point rules used by the macro engine.
