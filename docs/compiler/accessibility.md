# Accessibility lattice

This note describes how accessibility is enforced throughout the Chic frontend (type checking + MIR lowering).

## Assembly identity
- An “assembly” is a manifest package (`package.name` in `manifest.yaml`). Two packages are different assemblies even if they share namespaces.
- When lowering, every type/function/constructor needs its declaring package recorded so `internal`/`private protected` can be evaluated.

## Visibility rules (C#-like)
- `public`: always accessible.
- `private`: only the declaring type (and its nested types, when nesting is supported).
- `internal`: declaring package only.
- `protected`: declaring type and its derived types (any package), subject to the protected-instance rule.
- `protected internal`: union of `protected` **or** `internal`.
- `private protected`: intersection of `protected` **and** `internal` (must be derived **and** in the same package).

### Protected-instance rule
- Instance protected access inside `Derived : Base` is allowed through `this` or through receivers whose compile-time type is `Derived` (or a further derived type).
- Access via an arbitrary `Base` reference inside `Derived` is illegal: `other.Protected()` fails when `other` is only known to be `Base`.
- Static protected members ignore receiver expressions but still require the accessing type to be derived from the declaring type.

## Application sites
- **Member access**: all field/property/method accesses (static + instance), including object initializers and `base.Member(...)`, must run the lattice with the receiver’s static type (for instance members) and the current package/type.
- **Type references**: parameter/return/field/property types, base lists, interface implementations, extension targets, attribute payloads must not reference types that are less visible than the container’s visibility at the use site.
- **Constructor selection**: candidate constructors are filtered by accessibility before overload resolution.
- **Inheritance/overrides**: public types cannot derive from less-accessible bases; override targets must be accessible; `private protected` blocks cross-package overriding.
- **Namespace/static items**: `internal` for globals and statics is package-scoped, not namespace-scoped.

## Diagnostics
- Prefer a single primary diagnostic (“member is not accessible because it is X”) with notes for protected/internal/package context rather than cascades or ambiguous-overload noise.
- When access fails, avoid emitting follow-on “unknown member”/“ambiguous call” errors for the same site; treat accessibility as the root cause.
