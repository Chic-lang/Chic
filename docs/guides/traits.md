# Traits

Traits are Chic’s primary abstraction for shared behavior. A trait defines a set of required members, and an `impl` block provides an implementation for a concrete type.

## Basic pattern

```chic
namespace Demo;

public trait Formatter
{
    int Render(ref this);
}

public impl Formatter for int
{
    int Render(ref this) { return 5; }
}

public static class Reports
{
    public static int ToValue<TFormatter>(ref TFormatter formatter)
        where TFormatter : Formatter
    {
        return formatter.Render();
    }
}
```

## Notes

- Use `where T : TraitName` to express trait bounds on generics.
- Trait objects (`dyn TraitName`) are supported for dynamic dispatch where needed; see `docs/compiler/traits.md` for details.
