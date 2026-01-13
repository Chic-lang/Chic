# Type Aliases

Type aliases give an existing type a new name without creating a new runtime type. They are purely
compile-time sugar: the alias and its target share the same layout, ABI, and overload resolution.

## Syntax

Declare aliases at namespace scope with `typealias`:

```chic
public typealias AudioSample = UInt16;
typealias Milliseconds = int;
typealias Buf = Span<byte>;
```

- Visibility mirrors other top-level items. Use `public` to export the alias; omit it for an
  internal-only name.
- Attributes are allowed for tooling/metadata but cannot change runtime behaviour.
- Generic aliases reuse the standard generic parameter list, and must be invoked with matching
  arity.

## Usage

Aliases work anywhere a `TypeExpr` is expected:

```chic
public struct Mixer
{
    public AudioSample Sample;
    public Buf Window;
}

public AudioSample Process(AudioSample input) => input;
```

- Pointer/nullability suffixes are preserved through the alias (e.g., `AudioSample?` or `Buf*`).
- Overloads and type checks see the canonical underlying type; aliases do not create distinct slots.
- ABI is unchanged: `sizeof(AudioSample)` equals `sizeof(UInt16)`, and calls use the same calling
  convention as the target type.

## Alias vs. New Struct

Use a type alias when you want a clearer name for an existing shape and no behavioural differences.
Reach for a struct or record when you need:

- additional invariants or validation;
- custom methods, traits, or operator overloads;
- a distinct nominal type for overload resolution.
