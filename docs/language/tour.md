# Language tour

This tour introduces the core Chic syntax and the way you typically structure code. It’s intentionally example-driven; see `SPEC.md` for the complete language definition.

## Hello world

```chic
namespace Hello;

import Std.Console;

public static class Program
{
    public static int Main(string[] args)
    {
        Console.WriteLine("Hello, Chic!");
        return 0;
    }
}
```

## Namespaces and imports

- Use `namespace ...;` at the top of a file.
- Use `import Foo.Bar;` to bring a namespace into scope.
- `global import Foo;` applies to the whole compilation unit (see `docs/guides/using_directives.md`).

## Variables: `let` and `var`

- `let` declares an immutable binding.
- `var` declares a mutable binding.

```chic
let x = 10;
var y = 0;
y = y + x;
```

## Functions

Chic supports free functions and methods.

```chic
namespace Demo;

public int Add(int a, int b) { return a + b; }
```

## Types: structs, enums, classes

### Structs

Structs are value types:

```chic
public struct Point
{
    public int X;
    public int Y;
}
```

### Enums

Enums represent a closed set of variants:

```chic
public enum Color
{
    Red,
    Green,
    Blue,
}
```

### Classes

Classes are reference types:

```chic
public class Counter
{
    private int _value;

    public init(int start) { _value = start; }
    public int Next(ref this) { _value = _value + 1; return _value; }
}
```

## Error handling

Use `Std.Result<T, E>` for explicit error values and `throw` for exceptions. Many APIs are designed to return `Result` so failures can be handled explicitly at the call site.

See also: `SPEC.md#exceptions` and `docs/guides/unsafe_contract.md`.

## Next steps

- Create a project: `docs/getting-started.md`
- Learn the build manifest: `docs/manifest_manifest.md`
- Learn the CLI surface: `docs/cli/README.md` and `chic help`

