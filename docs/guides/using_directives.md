# Import directives

Chic supports `import` directives, `global import` directives, and `import static` imports that
keep modules concise without sacrificing clarity. Every directive block must appear at the start of
the file or namespace before any other item. Mixing directives with declarations produces an error
so the lookup order is always predictable. `using` directives are not supported; use `import`.

### Implicit `Std` Prelude

- The `Std` namespace is implicitly imported into every compilation unit (including `#![no_std]`)
  so `Option`, `Result`, spans, and platform wrappers are available without `import Std;`.
- Explicit `import Std;` remains valid for clarity, but aliasing `Std` to another namespace is
  rejected (`IMPORT0002`) because it would shadow the implicit prelude.
- Other namespaces still require explicit `import` directives or fully qualified names.

### Namespace Imports

Namespace imports pull public types from another namespace into scope:

```chic
namespace Std.Logging { public struct Logger { } }

import Std.Logging;

public struct Service
{
    private Logger logger; // `Logger` is resolved from Std.Logging.
}
```

The compiler searches aliases, the enclosing type, the current namespace chain, the imported
namespaces, and finally the global namespace. When two namespaces would resolve to the same type
name, the compiler reports an ambiguity diagnostic that lists each candidate so the author can
qualify the reference.

### Aliases

Alias directives bind a short identifier to a namespace or type:

```chic
import IO = Std.Platform.IO.Serializers;

public struct EventWriter
{
    public IO.JsonSerializer Serializer;
}
```

Aliases participate in the normal lookup order and are prohibited from forming cycles. Because they
are resolved before the namespace/import search, they provide a precise way to refer to long or
nested namespaces without relying on implicit rules.

Aliasing `Std` to any other namespace is rejected (`IMPORT0002`) because `Std` is implicitly imported
for every compilation unit.

### Static Imports

Static imports expose a type’s `static` members without repeating the qualifying type name:

```chic
namespace Math
{
    public class Numbers
    {
        public static int Seed = 5;
        public static int Increment(int value) => value + 1;
    }
}

import static Math.Numbers;

public struct Calculator
{
    public int Compute() => Increment(Seed);
}
```

Only members declared `static` are imported; attempting to use an instance member still results in a
diagnostic pointing back to the member declaration. If multiple `import static` directives introduce
the same member name, the compiler reports an ambiguity and asks the author to qualify the access or
remove the conflicting directive.

### Global Import Directives

Prefixing a directive with the `global` keyword makes it available everywhere in the compilation
unit, regardless of which namespace contains the declaration:

```chic
global import Std.Platform.IO.Serializers;
global import Alias = Std.Async.Tasks;
global import static Std.Diagnostics.Trace;

namespace Feature
{
    // No additional import needed: all three directives are visible here.
    public struct Pipeline
    {
        public Alias.Task Run(string input)
        {
            Trace("Starting pipeline");
            return Alias.Task.CompletedTask;
        }
    }
}
```

Global directives must appear at the very top of a source file before any namespace or type
declarations and cannot be nested inside namespaces or types. All global directives across the
compilation are collected and applied to every file, making it easy to centralise imports in a
single `global_imports.cl`. Conflicting alias targets between a global directive and a file-scoped
alias are rejected with a diagnostic; otherwise resolution order is global directives first,
followed by file-scoped directives and then enclosing namespaces.
