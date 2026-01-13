# Object initializers

Object initializers let you set writable instance fields and properties as part of a `new` expression.

## Syntax

```chic
let widget = new Widget(1, 2)
{
    Name = "demo",
    Enabled = true,
};
```

## Rules

- Each entry must bind to an instance field or property on the constructed type.
- A property must have a `set`/`init` accessor to be assigned in an initializer.
- Duplicate assignments to the same member are rejected.
- When a constructor call is ambiguous or does not match any overload, the compiler reports a constructor overload diagnostic.

## Collection initializers

Collection initializer blocks (`new Bucket { 1, 2, 3 }`) are parsed as a list of element expressions and are validated against the target’s supported `Add` pattern.

## Reference

- Language spec: `SPEC.md#object-construction--initializers`
