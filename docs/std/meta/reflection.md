# Std.Meta Reflection

Chic exposes compile-time reflection through `Std.Meta.Reflection.reflect<T>()` and `typeof(T)`. Both intrinsics run in CTFE, perform no allocations, and return stable handles for any `public` type that carries reflection metadata.

## Quick start

```chic
namespace Sample;

public struct Point { public int X; public int Y; }

public const Std.Meta.TypeDescriptor Descriptor = Std.Meta.Reflection.reflect<Point>();
public const ulong Size = Descriptor.Layout?.Size ?? 0;
public const ulong OffsetOfY = Descriptor.Layout?.Fields.Head.Tail?.Head.Offset ?? 0;
```

## Descriptor fields (v2 schema)

- **Handles:** `TypeHandle` bundles the full `Name` plus a stable `TypeId` so metadata can be compared without allocations. `typeof(T)` returns a `TypeHandle`.
- **Identity:** `TypeDescriptor` exposes `Namespace`, `Name`, `FullName`, `TypeId`, `Visibility`, `Kind`, and `IsGeneric`.
- **Surface:** ordered `GenericArguments` and `Bases` (both `DescriptorList<TypeHandle>`), `Attributes`, optional `UnderlyingType` (enums), and the ordered `Members` list of `Std.Meta.MemberDescriptor`.
- **Members:** `MemberDescriptor` records `Name`, `Kind`, `DeclaringType`, `Visibility`, and `Attributes` plus a discriminated detail payload:
  - `FieldDescriptor` (`FieldType`, `IsStatic`, `IsReadonly`, optional `Offset`, optional accessors).
  - `PropertyDescriptor` (`PropertyType`, `HasGetter`/`HasSetter`/`HasInit`, optional getter/setter/init descriptors).
  - `MethodDescriptor` (`ReturnType`, `Parameters`, `IsStatic`, `IsVirtual`/`IsOverride`/`IsAbstract`, `IsAsync`).
  - `ConstructorDescriptor` (`Parameters`, `IsDesignated`, `IsConvenience`).
- **Parameters & attributes:** `ParameterDescriptor` captures the `ParameterType`, `Mode` (`value`/`in`/`ref`/`out`), `HasDefault`/`DefaultValue`, and `Attributes`. `AttributeDescriptor` carries the name plus positional/named compile-time arguments (no runtime attribute objects).
- **Layout:** `TypeLayoutDescriptor` captures computed `Size`, `Align`, and per-field descriptors (`Name`, `Offset`, `Type`, `IsReadonly`). Offsets and sizes match the ABI metadata used by the backends.
- **Collections:** descriptor lists use the allocation-free `Std.Meta.DescriptorList<T>` to keep CTFE deterministic.

## Artefacts and stability

- Every build emits a versioned JSON sidecar `<artifact>.reflect.json` (`version = 2`) and embeds the same data into `.clrlib` archives and backend metadata sections (`@__chic_reflection` / `chic.reflect`).
- Reflection is read-only and deterministic: it cannot add types at runtime, primitive names stay identifier-like in the grammar, and the LLVM/WASM backends expose identical metadata. Accessor thunks are only generated when the member is safely addressable without boxing; otherwise they remain empty and callers fall back to metadata-only inspection.
