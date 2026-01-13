# Inheritance & Access Guide

Chic mirrors C# inheritance semantics: classes are sealed by default, abstract classes cannot be instantiated, and overrides must be explicit.

## Quick rules
- Use `virtual` on a base member to allow overrides; derived classes must write `override`. Silent shadowing is rejected.
- `sealed override` stops further overrides. `virtual sealed` on the base declaration is invalid.
- Abstract classes and abstract members require an override before the derived type becomes concrete; `new Abstract()` is illegal.
- Constructors chain with `: self(...)` or `: super(...)` and obey the same accessibility rules as other members.

## Access modifiers at a glance
- `public`: visible everywhere.
- `internal`: package-scoped (manifest boundary).
- `private`: declaring type only.
- `protected`: declaring type and derived types (any package) + protected-instance rule.
- `protected internal`: `protected` **or** `internal`.
- `private protected`: `protected` **and** `internal` (derived + same package).

### Protected-instance rule

```chic
public class Base { protected void Touch() { } }
public class Derived : Base
{
    public void Ok() => this.Touch();          // allowed
    public void AlsoOk(Derived other) => other.Touch(); // receiver is derived
    public void Illegal(Base other) => other.Touch();   // error: receiver not known to be Derived
}
```

### Cross-package expectations

```chic
// Package Core
public class Shape
{
    internal int Size;
    protected internal int Metadata;
    private protected int Token;
}

// Package App (references Core)
public class Circle : Shape
{
    public int Ok() => this.Metadata;  // protected path is valid
    public int NoInternal() => this.Size;  // error: different package
    public int NoPrivProt() => this.Token; // error: requires same package
}

public class Snooper
{
    public int Fail(Shape s) => s.Metadata; // error: not derived
}
```

## Base calls and overrides
- `base.Member(...)` targets the immediate base implementation and must respect the same visibility checks as normal access.
- Override binding requires matching name/parameters/return/nullability; mismatches produce diagnostics instead of silently choosing an overload.

## Tips for library authors
- Keep package boundaries aligned with intended assemblies; namespace overlap does not grant `internal` visibility.
- Avoid exposing less-visible types from public/protected APIs; the compiler will flag signature leaks.
