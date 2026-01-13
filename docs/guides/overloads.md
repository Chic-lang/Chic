# Overload resolution

Chic supports overloads for functions, methods, extensions, and constructors. When more than one candidate matches a call, the compiler uses deterministic ranking rules and reports focused diagnostics when the call is ambiguous or no overload matches.

## Tips for writing unambiguous calls

- Prefer explicit arguments (or named arguments) when optional parameters exist.
- Avoid overload sets that differ only by small type changes unless the call site always provides an explicit type.
- If a call becomes ambiguous, qualify the receiver (`Type.Member` vs `instance.Member`) or use named arguments to make intent explicit.

## Debugging overload errors

- Validate binding without producing artifacts: `chic check <file>` or `chic check <project>`.
- Look for the overload diagnostic codes and the candidate list in the error output.

## Example

```chic
namespace Demo;

public static class Calculator
{
    public static int Scale(int value) => value;
    public static int Scale(int value, int factor = 2) => value * factor;
}

public static class Program
{
    public static int Main()
    {
        let exact = Calculator.Scale(5);
        let explicit = Calculator.Scale(5, factor: 3);
        return exact + explicit;
    }
}
```
