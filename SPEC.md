# Chic Language Specification

*Status:* Draft  
*Maintainer:* Bootstrap compiler team (temporary bootstrap implementation in Rust)

> This document captures the authoritative working specification for the Chic programming language while the compiler is being prototyped in Rust. Every semantic or syntactic change to the language **must** be reflected here first. The build agent (`AGENT.md`) links back to this document and expects it to stay current.

---

## 1. Philosophy

- **Surface syntax:** C#-style declarations, namespaces, and block structure. No Rust keywords such as `impl`.
- **Semantics:** Rust-like ownership and borrowing, deterministic destruction, and monomorphized generics.
- **Paradigms:** Data-oriented & functional by default (free functions, structs, enums, pattern matching). Object-oriented features remain optional.
- **Performance:** Zero-cost abstractions, ahead-of-time compilation, `no_std` capable, `panic = abort` default for freestanding targets.
- **Interop:** First-class C ABI support with header-driven bindings and explicit layout control.
- **Macros:** Attribute-style compiler extensions (e.g. `@derive`, `@memoize`).
- **Simplicity charter:** Clarity and predictability outrank raw expressivity or micro-optimisation. The grammar remains LL(1), MIR lowering stays single-pass, and new features must demonstrate linear compile-time cost (or provide explicit justification) before landing.
- **Type inference:** Local inference is supported (for example, `var total = Add(1, 2);`) without requiring an explicit annotation at the call site.

Each language feature is expected to be explainable in a short paragraph and to compile within a single MIR pass without introducing speculative recompilation.

| Area | Action | Effect |
| --- | --- | --- |
| Simplicity | Keep grammar LL(1), inference local-only | Faster compiler, smaller mental model |
| References | Restrict borrows to call scope | Eliminates lifetime syntax and reduces borrow complexity |
| Reflection | Provide `reflect<T>()` and `quote(expr)` | Enables compile-time metaprogramming without runtime cost |
| Containers | Inline intrinsic container operations | Zero dispatch overhead in hot loops |
| Memory model | Default to stack allocation, explicit heap | Deterministic destruction and performance |
## Diagnostics & UX (Section 3.9)

- **Severity taxonomy:** `error`, `warning`, `note`, `help`. Every diagnostic carries a structured `DiagnosticCode` `{ code, category }`, where `code` is namespaced (`PARSE00042`, `TYPE00017`, `LINT00008`, `MIRL00003`, `WASM00002`, etc.) and `category` mirrors the subsystem (`parse`, `typeck`, `lint`, `mir-lowering`, `mir-verify`, `backend`).
- **Span model:** Spans are byte ranges plus a `file_id`. Files register with a `FileCache` (path + source) so line/column math stays stable across CLI, LSP, and tests; unknown file IDs render without snippets.
- **Renderers:** `human` (Rust-style multi-line with coloured spans when TTY), `toon` (emoji header + human snippets), `short` (single-line `path:line:col: severity[CODE]: message` with note counts), and `json` (machine-readable).
  - Defaults: `human` for TTY stderr, `short` otherwise; `NO_COLOR` disables ANSI even on a TTY.
  - Suggestions render as `help:` lines, with optional `span`/`replacement`. Notes follow snippets in emitted order.
- **JSON schema (version `1.0.0`):**

```json
{
  "version": "1.0.0",
  "severity": "error",
  "code": { "code": "PARSE00001", "category": "parse" },
  "message": "unknown identifier",
  "primary_span": {
    "file": "main.cl",
    "start": 42,
    "end": 45,
    "line_start": 3,
    "column_start": 12
  },
  "labels": [{ "message": "not found", "span": { "...": "..." }, "is_primary": true }],
  "notes": ["stage: parse"],
  "suggestions": [{ "message": "replace with `value`", "span": { "...": "..." }, "replacement": "value" }]
}
```

- **Authoring guidance:** Keep messages concise and actionable; attach one primary label to the root cause and secondary labels for contributors. Provide replacements for mechanical fixes. Stage notes (`stage: parse`, `stage: typeck`, `stage: lint`) are appended by the reporting layer so downstream tools can group diagnostics.
- **CLI flag:** `--error-format <human|json|toon|short>` is accepted by `check`, `lint`, `build`, `run`, `test`, `mir-dump`, and `header`. The flag overrides the TTY-based default and is preserved by LSP/test harnesses.

## Control Flow & Reachability

- Chic treats unreachable code as a hard error (`E0400`, category `reachability`). Any statement the compiler can prove will never execute fails the build.
- The reachability pass walks the control-flow graph after MIR lowering, honours non-returning constructs (`return`, `throw`, `panic`, and runtime intrinsics marked never-returning), and folds obvious constants so branches guarded by `if (false)`, `while (false)`, or zero-trip loops (`for (int i = 0; i < 0; i++)`) are rejected. Switches with compile-time-known discriminants mark all other arms unreachable. After an unconditional exit the remaining statements in the block are unreachable.
- Typical triggers: code after `return`/`throw`, constant-false branches, loop bodies that cannot execute, and switch arms proven impossible by exhaustiveness.
- Diagnostics use the Rust-style renderer with file/line/column, snippet, and carets pointing at the first unreachable statement. Secondary labels highlight the exit point or constant condition when available.

Example:

```
error[E0400]: unreachable code
  --> packages/std/src/numeric/uint16.cl:14:9
   |
10 |     if (parsed.HasValue)
11 |     {
12 |         return new UInt16(parsed);
13 |     }
14 |     throw new FormatException("Input span was not a valid UInt16.");
15 |     DoSomething(); // unreachable
   |     ^^^^^^^^^^^^^ unreachable code
   |
note: this code is unreachable because control flow always exits before it
```

Developer guidance: unreachable code is treated as a bug. Remove statements after non-returning exits, delete or gate constant-false branches, and simplify zero-trip loops instead of leaving them dormant. The compiler will not accept silently unreachable code in user code or the standard library.

## 2. Core Building Blocks

### 2.1 Top-Level Functions

```chic
namespace Math;

public double Hypot(double x, double y)
{
    let xx = x * x;           // immutable
    var yy = y * y;           // mutable
    yy += xx;
    return sqrt(yy);
}
```

- `let` introduces immutable bindings; `var` introduces mutable bindings.
- Functions live at namespace scope; utility classes are not required. When no namespace is
  declared, functions reside in the root module and can be imported from other files via
  `import` just like namespaced symbols.
- The `Std` namespace is implicitly imported into every compilation unit (including `#![no_std]`),
  making `Std.Option`/`Std.Result`, spans, platform wrappers, and other standard items available
  without an explicit `import Std;`. Explicit imports remain valid, but aliasing `Std` to another
  namespace is rejected with `IMPORT0002` because it would shadow the implicit prelude.
- Namespace declarations support both **file-scoped** and **block-scoped** forms. A file-scoped declaration (`namespace Math;`) applies to the remainder of the file, while a block-scoped declaration wraps its members in braces:

  - Only a single file-scoped declaration may appear in a compilation unit and it must be placed before any other declarations (aside from `import` directives). Once declared, every top-level item in the file is implicitly nested inside that namespace.
  - File-scoped namespaces may not carry arbitrary attributes. Crate-level attributes use `#![...]` (`#![no_std]`/`#![std]`). Namespace-scoped `@no_std`/`@nostd` attributes are not supported.
  - Additional block-scoped namespaces compose with the file-scoped base. For example:

    ```chic
    namespace Root.App;

    namespace Services { ... }          // becomes Root.App.Services
    namespace Diagnostics.Logging { ... } // becomes Root.App.Diagnostics.Logging
    ```

  - Block namespaces can still be nested as usual; each inner declaration appends its identifier to the active namespace path regardless of whether the file began with a file-scoped namespace.
  - Separate compilation units may declare different namespaces (or even different roots) within the same build. The workspace assembler wraps each file-scoped namespace before type checking so symbols remain qualified, which means there is no restriction that every input share the same `namespace` declaration—mixing `Root`, `Root.Services`, and `Diagnostics` modules in a single invocation is supported as long as type names remain unique.

```chic
namespace Utilities.Logging
{
    public struct Record { public string Message; public int Level; }

    namespace Diagnostics
    {
        public enum Kind { Info, Warning, Error }
    }
}
```

- Nested block namespaces automatically append their identifier segments to the enclosing path (`Utilities.Logging.Diagnostics` in the example above). `using` directives may precede either form and may also appear inside a block-scoped namespace ahead of other members.
- Namespace ownership and `@friend` escape hatches:
  - Each package owns a namespace prefix declared in `manifest.yaml` (`package.namespace`, defaulting to `package.name`). Source files must declare namespaces that start with this prefix unless explicitly authorised.
  - Additional prefixes are granted via file-scoped `@friend("Allowed.Namespace")` directives or the manifest `package.friends` list. `@friend` is package-level (not consumer-configurable) and may appear multiple times per file.
  - Friend prefixes must be non-empty, dot-separated identifier segments and cannot match the package’s own prefix. Empty/invalid prefixes raise `PKG0002`; redundant self-prefixes raise `PKG0001`; duplicates are ignored with `PKG0004` warnings.
  - Declaring a namespace outside the package/friend allowlist produces `PKG0003` with a fix-it to align the namespace with the manifest prefix. Conflicts with other packages’ owned prefixes will surface once the package graph is available.
  - Files without an explicit `namespace` are inferred from the source root and path (`<package.namespace>.<source-root-relative-path>`); deviating from the inferred path requires an explicit `namespace` and, if outside the prefix, an authorised `@friend`.
- Package imports: File-scoped `@package("Name")` directives bind to entries under `manifest.yaml:dependencies`. Missing entries emit `PKG0100` with a fix-it to add the dependency; resolution honours semver ranges, git/path sources, lockfiles (`manifest.lock`), and offline caches (`CHIC_OFFLINE`/`CHIC_PACKAGE_OFFLINE`).

#### Advanced Function Patterns

Chic treats lambdas and closures as first-class values. A closure expression such as
`(int value) => value + delta` lowers to a lightweight struct that stores the captured
environment and exposes an `Invoke` method. The runtime re-exports these shapes through the
`std::functional::Func<TArgs..., TResult>` family so closures can cross API boundaries just
like any other value.

Lowering materialises a compiler-visible struct for every lambda. Inside a function
`Root::Use`, the example above becomes a synthetic type `Root::Use::lambda#0` whose fields
mirror the captured locals (`delta` here). The compiler also emits a thunk
`Root::Use::lambda#0::Invoke` that receives the captured values followed by the call-site
arguments, so the thunk body executes like any ordinary Chic function. Non-capturing
lambdas reuse their thunks across invocations and coerce directly to `fn` pointers without
allocating an environment. Capturing lambdas retain their environment struct; calling
`.to_fn_ptr()` allocates a lightweight runtime handle that stores the captured values and
returns a pointer to an adapter thunk. The handle owns its environment and runs the closure’s
drop glue when the pointer falls out of scope, so escaped callbacks remain memory safe.

```chic
import std.functional;

public Func<int, int> MakeAdder(int delta)
{
    return (int value) => value + delta;
}

public int Demo()
{
    let add_one = MakeAdder(1);
    return add_one(41); // 42
}
```

Captured locals are moved into the closure object. Attempting to borrow a captured `ref` or
`out` binding past the creator’s scope triggers the borrow checker, and capturing a type that
lacks the required auto traits (`ThreadSafe`, `Shareable`) yields the same diagnostics that
apply to async state machines. Closures may themselves be generic; each instantiation
monomorphises to a unique environment struct.

Interop with C-style function pointers uses the `fn` type syntax. A parameter written as
`fn(void*, void*) -> int` expects a pointer with the Chic calling convention
unless annotated with `@extern`. Non-capturing lambdas and plain function names satisfy these
signatures directly; the compiler emits a zero-sized shim so the pointer can be handed to
foreign code.

Function pointer types may appear anywhere a normal type annotation is accepted. Locals,
parameters, and fields can be declared with `fn` signatures, and the compiler preserves the
selected calling convention through MIR lowering:

```chic
fn(int, int) -> int callback = Add;
fn @extern("C")(int) -> int c_abs = abs_extern;

public int Invoke(fn(int, int) -> int func, int x, int y)
{
    return func(x, y);
}
```

The MIR builder records these annotations as `Ty::Fn` values, ensuring that Chic-compatible
functions and `@extern` imports can be passed, stored, and invoked through uniform pointer
semantics.

When a scope contains multiple overloads that share the same base name, the resolver keeps all
candidate symbols and selects the one whose signature exactly matches the expected `fn` type. The
disambiguation happens during MIR coercions so the final program records an unambiguous function
symbol, and mismatched overload sets trigger diagnostics that enumerate the available signatures.

```chic
@extern("C") @link("c")
private static extern void qsort(
    void* base,
    usize count,
    usize elem_size,
    fn @extern("C")(void*, void*) -> int comparator);

public static unsafe void SortInts(int* base, usize len)
{
    let compare: fn @extern("C")(void*, void*) -> int =
        (void* lhs, void* rhs) =>
        {
            let lhs_i = (int*)lhs;
            let rhs_i = (int*)rhs;
            return (*lhs_i) - (*rhs_i);
        };
    qsort(base, len, sizeof(int), compare);
}
```

Closures with captured state may still cross the boundary by calling `.to_fn_ptr()`, which
allocates a runtime handle to store the captured values and returns a pointer to an adapter
thunk. The thunk consults the handle to rebuild the environment each time it runs, and the
borrow checker enforces that the handle outlives every consumer. Attempting to convert a
closure whose environment does not outlive the callback target still produces a diagnostic
explaining the escape.

##### Dynamic Imports and `@extern`

Chic augments `@extern` with a parameter list modelled after .NET’s `DllImport`. The grammar
is documented in detail in [`docs/runtime/dynamic_ffi.md`](../runtime/dynamic_ffi.md); the summary
below captures the normative behaviour:

- `@extern(convention = "system", library = "user32", alias = "MessageBoxW", binding = "lazy",
  optional = false, charset = "utf16")` describes how a Chic stub maps to a platform symbol.
- `library` is optional. When omitted the compiler assumes the symbol is provided by the static
  linker (`@link`). When present the runtime uses `dlopen`/`dlsym` (`LoadLibrary`/`GetProcAddress`)
  according to `binding`.
- `binding = "lazy" | "eager" | "static"` controls when resolution occurs. `static` is only valid
  when no `library` argument is supplied; combining the two produces a diagnostic and falls back to
  `lazy`. Optional bindings return the default value (zero/null) and log `[FFI-W0001]` when the
  library or symbol is missing; required bindings terminate the process with a structured FFI
  diagnostic.
- `alias` defaults to the Chic identifier (`Function` in `namespace::Function`). The LLVM backend
  still sanitises the Chic symbol name for the stub so namespace collisions do not leak into the
  external name.
- All argument validation happens at parse time. Specifying `binding` without `library`, or setting
  `optional=true` without a `library`, or pairing `binding="static"` with a `library`, produces
  diagnostics.

The CLI forwards this metadata to the runtime via `--ffi-search`, `--ffi-default`, and
`--ffi-package`. Each flag is described in [`docs/runtime/dynamic_ffi.md`](../runtime/dynamic_ffi.md):
search paths and custom filename patterns are embedded into the generated module so lazy/eager
bindings use the same resolution order that the build requested. Packaged libraries are copied into
`<artifact>.deps/` and `.clrlib` archives for distribution and are probed automatically by the
runtime.

The metadata section emitted by codegen includes a line per extern definition in the form\
`extern:Namespace::Func=convention=system;binding=lazy;library=user32;alias=MessageBoxW`.
Runtimes and tooling rely on that data for eager prefetch, packaging (`--ffi-package`),
and debugging missing dependencies.

#### Local Functions (Section 3.24)

Chic reserves the `function` keyword for declaring nested functions inside any block.
The syntax mirrors namespace functions: the keyword is followed by an explicit return type,
an identifier, optional generic parameters/`where` clauses, a parameter list, and either a
block body or expression-bodied form.

```chic
public int Outer(int seed)
{
    @memoize
    async function int Helper<T>(int value) where T : Copy
    {
        return value + seed;
    }

    return Helper(seed);
}
```

- `function` may only appear in statement position. Using it at namespace/type scope is a
  parse error.
- Modifiers such as `async`, `constexpr`, and `unsafe` precede the keyword (`async function …`).
  The parser records additional modifiers (e.g., `static`) for future phases but they have no
  effect yet, matching the behaviour of free functions.
- Attributes written immediately before the statement are treated as function attributes.
  Built-ins that only apply to statements (for example `@pin`) still produce diagnostics.
- Local functions must provide a body (block or `=>` expression). A bare `;` is rejected so
  nested declarations cannot forward-declare work.
- The AST reuses `FunctionDecl` for local functions and wraps it in
  `StatementKind::LocalFunction`, ensuring downstream passes can treat them like named closures.

Local functions inherit the lexical scope of their enclosing method and may capture surrounding
locals or generic parameters. The compiler treats them like named closures:

- Referencing the local function produces a value. If the function does **not** capture anything the
  value lowers to a plain function symbol that takes exactly the parameters written in source.
  Capturing local functions synthesise an environment struct and generate a thunk that accepts the
  environment pointer as a hidden first argument followed by the user parameters.
- Captures obey the borrow checker. Moving a `ref` binding into a local function consumes the
  binding, and capturing a mutable local enforces exclusivity just like lambdas. The environment is
  snapshotted when the local function value is instantiated, so subsequent mutations are visible to
  later calls.
- Locals can be declared anywhere in the block (before or after their first use). The lowerer
  predeclares every local function in a block before lowering statements so we can resolve calls
  made earlier in the body.
- Parent generics and `where` constraints remain in scope. Local functions can introduce additional
  generic parameters of their own, and the type checker validates them exactly like namespace
  members.
- Async, generator, and throwing local functions follow the same diagnostics as their top-level
  counterparts. Awaiting a local async function is mandatory unless it is immediately returned.
- LLVM emits non-capturing locals as `internal` functions so they are never exported. Capturing
  locals materialise an environment type named `Parent::local_env#N` and a thunk named
  `Parent::local$N::Name`, which receives the environment pointer as its first parameter. The WASM
  backend mirrors the same calling convention, so runtimes can treat local functions like thin
  wrappers around closures.
- Debuggers and tooling can identify locals by their mangled names: `local$N` increments for each
  declaration in the parent body. A companion environment type (`local_env#N`) contains the captured
  fields.

Additional examples and debugging tips live in `docs/guides/local_functions.md`.

When the compiler lowers an `@extern` item it also records the metadata needed by dynamic loaders.
Each extern contributes an `extern:Namespace::Function=…` line to the `.chic.meta` payload,
capturing the calling convention, binding mode (`static`, `lazy`, or `eager`), library name,
alias, charset hint, and whether the entry is optional. LLVM lowering honours the recorded alias:
if `alias="MessageBoxW"` is provided, the backend emits references to `MessageBoxW` instead of the
sanitised Chic symbol. These metadata entries allow tools such as `chic header`, runtime
packagers, and future dynamic loaders to reason about platform-specific bindings without
re-parsing source code.

#### Converting closures to `fn` pointers

Non-capturing lambdas convert implicitly to function pointer types. They behave the same as named
functions and retain the Chic calling convention:

```chic
fn() -> int make() => 7;

public int Invoke(fn() -> int callback)
{
    return callback();
}

public int Main()
{
    fn() -> int ptr = make; // implicit conversion succeeds
    return Invoke(ptr);
}
```

Closures with captures must opt in explicitly. Calling `.to_fn_ptr()` materialises the adapter thunk
described above and returns a structured `fn` value when the borrow checker can prove that the
closure's environment outlives every consumer of the pointer. The runtime representation stores
`invoke`, `context`, `drop_glue`, `type_id`, `env_size`, and `env_align` so backends can load the
thunk + environment pointer and schedule the correct drop glue (via `chic_rt_drop_invoke` and
`chic_rt_closure_env_free`) when the pointer is destroyed:

```chic
public int RegisterCallback(fn() -> int target)
{
    return target();
}

public int Main()
{
    var state = 5;
    var callback = () => state + 1;
    // Borrow checker verifies `state` lives long enough for the callback.
    return RegisterCallback(callback.to_fn_ptr());
}
```

Attempting to convert a capturing closure without using `.to_fn_ptr()` produces a diagnostic, and
`.to_fn_ptr()` itself fails when the closure would outlive its environment. These rules ensure that
both Chic and interop targets see the same ABI-stable `fn` signature.

### 2.1.1 Type name resolution (qualified vs unqualified)

- Fully-qualified names (`Foo.Bar.Type`) always resolve directly. Unqualified names are resolved with this precedence: **(1)** alias imports (`import Alias = Foo.Bar;`), **(2)** the current type and its nested types, **(3)** the current namespace chain from innermost to outermost, **(4)** namespace imports (`import Foo;`), **(5)** the implicit `Std` prelude import, **(6)** any already-qualified name provided in the source.
- The same resolver is used for base/interface lists, constructor `super(...)` targets, generic arguments, and ordinary type annotations; no context requires a manually assembled fully-qualified string.
- If multiple candidates exist at the same precedence tier, the compiler emits a single ambiguity diagnostic that lists each candidate’s fully-qualified name and package. Use a qualification (`Foo.Bar.Type`) or an alias import (`import Type = Foo.Bar.Type;`) to disambiguate.
- Missing bases/interfaces are errors rather than silently-accepted placeholders; visibility rules (`internal`, `protected`, etc.) are checked after resolution.

### 2.2 Data Types Without OOP

```chic
namespace Geometry;

public struct Point { public int X; public int Y; }

public enum Shape
{
    Circle { public double Radius; },
    Rect   { public int W; public int H; }
}

public int Area(in Shape s)
{
    switch (s)
    {
        case Circle c: return (int)(3.141592653589793 * c.Radius * c.Radius);
        case Rect r:   return r.W * r.H;
    }
}
```

- Generic structs follow the same syntax; type parameters are declared after the name and may
  carry `where` constraints. Nested type definitions (such as helper structs or iterators) can be
  declared inside a struct and declare their own generic parameters.

```chic
public struct Vec<T>
{
    public struct Enumerator<U>
    {
    }
}
```

- `enum` is a tagged union with destructuring-friendly `switch`.
- Entire libraries can be built with `struct`, `enum`, and free functions.

#### Collections: HashMap / Dictionary

Chic ships a Chic-native hash map under `Std.Collections.HashMap<K, V, THasher = DefaultHasher>`
(`Dictionary` is an alias when exposed by the stdlib). The container is deterministic across
LLVM/WASM and never relies on Rust shims.

- **Ownership:** The map owns keys and values. Inserts/entry operations move data in; removals,
  `take`, and drains move data out. Drop glue for keys and values runs exactly once; rehashing
  never double-drops.
- **Key requirements:** `K` must provide equality and hash glue (`__eq_glue_of`, `__hash_glue_of`,
  typically via `GetHashCode` or `@derive(Hashable)`). Construction fails fast if equality glue is
  missing. Custom hashers must be deterministic across backends.
- **Hashing:** Default hasher is deterministic FNV-1a derived (`Std.Hashing.DefaultHasher`). Users
  may supply any `Std.Hashing.IHasher`; no implicit random seeding occurs.
- **Borrow model:** Shared borrows (`in this`) allow `len`, `capacity`, `is_empty`, `contains_key`,
  `get`, `keys`/`values` iteration. Unique borrows (`ref this`) are required for mutation
  (`insert`, `replace`, `remove`, `clear`, `reserve`, `shrink_to`, `retain`, `drain`, `drain_filter`,
  `entry`). Rehashing invalidates iterators/entry guards; mutation APIs that may rehash require a
  unique borrow so invalidation is enforced by the type system.
- **Iteration order:** Not sorted but deterministic for a fixed insertion sequence, hasher, and
  capacity evolution. LLVM and WASM share the same order.
- **API surface:** `new`, `with_capacity`, `with_hasher`, `len`, `capacity`, `is_empty`, `reserve`,
  `shrink_to`, `insert`, `replace`, `remove`, `contains_key`, `get`/`get_mut` (only when borrow-safe),
  `get_ptr`, `retain`, `clear`, `iter`, `keys`, `values`, `values_mut`, `into_iter`, `drain`,
  `drain_filter`, plus `entry` with `OccupiedEntry`/`VacantEntry` and
  `and_modify`/`or_insert`/`or_insert_with`/`or_default`.
- **Entry semantics:** Entry guards tie to a unique borrow. `and_modify` removes then reinserts after
  mutation (rehashing the updated value). `or_insert`/`or_insert_with`/`or_default` insert on miss.
  Keys are not mutated in place; changing a key requires removal + reinsertion via the entry helpers.
- **Borrow-safe lookups:** The map does not expose long-lived references. `get` clones/copies when
  clone glue exists; if the value is `Copy` it is copied, otherwise a deterministic exception is
  raised. `get_mut` is exposed only when the borrow rules guarantee exclusivity.
- **Complexity:** `insert`/`remove`/`contains_key`/`get` are expected O(1) amortised under a
  reasonable load factor; `reserve`/`shrink_to`/`clear` are O(n) when they reallocate or scan the
  table. Drains are linear in the number of yielded elements.

### 2.2.1 Type Aliases

- **Syntax:** Namespace/file scope declarations use `typealias Identifier = TypeExpr;`. Generic
  aliases reuse the existing type-parameter list syntax: `typealias VecOf<T> = Std.Collections.Vec<T>;`.
  `public`/`internal` visibility mirrors other top-level items; `public` aliases are exported. Attributes
  are permitted only for tooling/metadata (no runtime effect) and must preserve LL(1) parsing by
  preceding the keyword.
- **Scoping & resolution:** Alias names live in the containing namespace and are discovered alongside
  other symbols. `using` directives (including alias usings) participate in lookup, and shadowing
  follows normal rules (innermost scope wins; ambiguous resolutions report diagnostics).
- **Semantics:** A type alias is a pure compile-time name substitution. It does not introduce a new
  nominal type, cannot change layout/ABI, and preserves trait constraints, pointer/ref qualifiers,
  and nullability on the target type. Expansion happens before type checking, layout computation, and
  code generation so overload resolution and metadata see the canonical underlying type.
- **Export/metadata:** Public aliases are emitted as alias entries (name → canonical target) in
  reflection sidecars/`.clrlib` metadata and any generated headers. When a C header is produced and
  the target has a C representation, the exporter emits a deterministic `typedef`/mapping comment
  rather than a new wrapper.
- **Diagnostics:** Cycles are illegal (`typealias A = B; typealias B = A;` → error) and expansion
  stops on the first detected loop. Unknown target types are errors. Generic alias arity must match
  the declaration; mismatches and unresolved generic targets surface the standard type resolution
  diagnostics.

### 2.3 Optional OOP

```chic
namespace Shapes;

public interface IShape
{
    double Area(in this);
    void Move(ref this, int dx, int dy);
}

public class Circle : IShape
{
    public double Radius;

    public double Area(in this) => 3.141592653589793 * Radius * Radius;
    public void Move(ref this, int dx, int dy) { /* mutate center fields */ }

    public void dispose(ref this) { /* release resources deterministically */ }
}
```

- Use classes or interfaces only when dynamic polymorphism is beneficial.
- Interfaces may declare default method bodies. Implementors always win when they provide an override; otherwise Chic falls back to the inline default, then to the best matching namespace-level extension block that marked the member as `default`. Ambiguities or cycles are compile-time errors (**DIM0003/DIM0004**), never silent picks.
- Adding a new interface member requires either (a) a default body or (b) an intentional breaking-change entry in `CHANGELOG.md`. The compiler does not patch older binaries; downstream builds must recompile to pick up new slots.
- Chic installs vtable/default metadata at load time (`@__chx_iface_defaults` / `chic.iface.defaults`). If a precompiled library misses a required slot, trait calls fail immediately with `trait ... vtable ... is missing slot ...` instead of attempting to patch the dispatch table.
- `extension` blocks may target interfaces to provide helper APIs or retroactive defaults. Each block can carry `when` constraints so defaults only activate when predicates hold. Conflicting active defaults for the same member are rejected during type checking.
- The compiler records every `(implementer, interface, method, symbol)` binding that relied on a default in `MirModule::interface_defaults`. Backends expose this metadata as `@__chx_iface_defaults` (LLVM global) and the `chic.iface.defaults` WebAssembly custom section so runtime tooling and reflectors can reason about which implementers inherited which defaults.

#### Generic Parameter Variance

Interfaces and delegates may annotate their type parameters with variance keywords that mirror the CLR surface:

```chic
public interface IProducer<out TResult>
{
    TResult Produce();
}

public interface IConsumer<in TInput>
{
    void Consume(TInput value);
}
```

- `out` marks the parameter covariant, permitting `IProducer<Derived>` to flow into an expected `IProducer<Base>` as long as `Derived : Base`. Covariant parameters may only appear in output positions: method returns, property getters, or parameters marked `out`/`in this`. Using them in `ref` or input-only slots (`Value`, `in`, or property setters) yields the `TCK022` variance diagnostic.
- `in` marks the parameter contravariant, permitting `IConsumer<Base>` to satisfy a requirement for `IConsumer<Derived>`. Contravariant parameters are limited to input-only positions. Returning or exposing them via getters/`out` bindings is rejected; `ref` counts as both input and output, so contravariant parameters cannot flow through `ref` either.
- Parameters that omit a modifier remain invariant. All class, struct, enum, and trait generics are likewise invariant—variance modifiers on these declarations are a parse error.
- Delegates reuse the same syntax and enforcement: variance modifiers are permitted on delegate generic parameters, validated against the `Invoke` signature, and honoured during conversions (method groups, lambdas, function pointers, or delegate-to-delegate assignments).
- The variance analysis runs over the full signature, so modifiers combine with Chic’s parameter binding keywords: `in this` is treated as output, `ref this` forces invariance, and property initialisers are considered input.
- Metadata and interop surfaces carry the declared variance (and the `TypeFlags::FALLIBLE` bit) so downstream tools can reason about assignments. LLVM binaries embed the tables inside `@__chic_type_metadata`, the WASM backend writes the same byte stream to the `chic.type.metadata` custom section, and reflection descriptors serialise annotated parameter strings (e.g., `out TResult : IShape`). Runtime helpers expose the encoded variance through `RuntimeTypeMetadata` and the `RuntimeGenericVariance` enum; see `docs/compiler/generics.md` for the pipeline and regression matrix.

##### Async & Threaded Usage

Async state machines and thread payloads reuse the same variance analysis. A contravariant
channel parameter, for example, can safely accept any subtype so long as the auto-trait
requirements for the captured payload are met:

```chic
public interface Channel<in T>
{
    Task SendAsync(T message);
}

public async Task PumpAsync<T>(Channel<in T> sink, T payload)
    where T : @thread_safe
{
    await sink.SendAsync(payload);        // `payload` survives suspension → ThreadSafe required.
    std.thread.spawn(() => sink.SendAsync(payload)); // thread boundary enforces the same trait.
}
```

- The contravariant `Channel<in T>` interface restricts `T` to input positions, so the
  variance checker accepts the declaration and the type checker protects callers
  from flow-breaking assignments (`TCK022`).
- Because `payload` survives an `await` *and* is moved into `std.thread.spawn`, the type
  checker demands `ThreadSafe`. Omitting the `where T : @thread_safe`
  constraint produces `TCK035`, pointing back to both the async capture and the thread
  spawn so authors understand every frontier that needs the guarantee.
- When the compiler cannot prove the trait (for example, because `T` is another type
  parameter that lacks a constraint), it emits `TCK037` with suggestions to add the
  missing `where` clause or wrap the payload in `std.sync::Mutex`/`RwLock`.
- Shared borrows that cross awaits additionally require `Shareable`. The borrow checker
  reports the suspension site and recommends guarding the value with a lock or switching to
  owned payloads.

##### Diagnostics & Tooling

| Code   | Description | Surfaces | Spec / Doc anchors |
|--------|-------------|----------|--------------------|
| `TCK022` | Variance modifier used outside interfaces/delegates or in an illegal position (e.g., covariant parameter in setter). | Parser + type checker (`typeck::arena::tests::diagnostics::variance_*`). | §2.3 Generic Parameter Variance (this section). |
| `TCK035` | Auto-trait (`ThreadSafe`/`Shareable`) requirement definitively violated for async/thread payloads or trait bounds. | Type checker + async lowering (`docs/compiler/thread_safety.md#diagnostics-table`). | §2.7.1 Concurrency guarantees; Thread Safety doc. |
| `TCK037` | Auto-trait requirement could not be proven (typically missing `where` clauses). | Type checker. | §2.7.1 Concurrency guarantees; Thread Safety doc. |
| `LCL0001` | Typed locals of the form `Type name = expr;` are not valid; declare block locals with `let`/`var` instead. | Parser. | §2.17 Variable Declarations. |
| `LCL0002` | Attempt to mutate an immutable `let` binding (assignment, `ref`/`out`, `++`, etc.); suggests changing to `var`. | Borrow checker. | §2.17 Variable Declarations. |
| `MM0102` | Thread payload not `ThreadSafe` when calling `std.thread.spawn`/`Task.Run`. | MIR lowering (`std.thread`). | §2.7.1 Concurrency guarantees; `docs/compiler/thread_safety.md`. |
| `EH0001` | Fallible temporary dropped without routing through a handling site (`throw`, `?`, pattern match, etc.). | `mir::passes::fallible_drop`. | §4.3 Fallible Types & Diagnostics. |
| `EH0002` | Fallible value may exit a scope (return/unwind/async drop) without being handled. | `mir::passes::fallible_drop`. | §4.3 Fallible Types & Diagnostics. |

Diagnostics should link back to the relevant spec section (and to supporting documentation when it exists) so tooling can surface “Read the spec” entry points consistently.

#### Constructor Forms

Chic classes declare constructors with the Swift-inspired `init` surface. C#-style `TypeName(...)` members are rejected: declaring any method whose name matches the containing type (including generic arity) yields a compile-time error with a `use 'init'` fix.

```chic
public class Button : Control
{
    public int Width;
    public int Height;

    // Designated initializer.
    public init(int width, int height)
    {
        self.Width = width;
        self.Height = height;
    }

    // Delegating convenience initializer using the Swift surface.
    public convenience init() : self(100, 32) { }

    // Designated initializer that chains into the base type.
    public init(int width) : super(width)
    {
        self.Width = width;
        self.Height = 24;
    }
}
```

- Every class must expose at least one *designated* `init`. Convenience initialisers are rejected when no designated candidate exists.
- Declaring `public TypeName(...)` (without `init`) or any method named after the containing type is invalid and produces a constructor diagnostic that points at the member name.
- Convenience initialisers must delegate via `: self(...)`. The parser emits an error otherwise.
- Designated initialisers may call the base type with `: super(...)` before executing user code.
- Constructors implicitly receive a unique `self` parameter. Stored fields must be written along **all** return paths; the MIR builder runs a dataflow pass that reports whichever fields remain uninitialised.
- Backends lower `init` bodies as normal functions tagged with `FunctionKind::Constructor`, so they are available to both LLVM and WASM code generation without additional call conventions.

#### Virtual Dispatch and Base Calls

- Instance methods are `sealed` by default. Mark a method `virtual` to allow derived classes to replace the implementation. Virtual members participate in Chic’s vtable layout; only instance methods (not `static`) can carry the modifier.
- Derived classes must use `override` when replacing a virtual method. Signature compatibility requires matching name, parameter types/modifiers, and return type (including nullable state). The compiler rejects silent shadowing.
- `sealed override` stops further overriding in subclasses. Combining `virtual sealed` on the base declaration is rejected; Chic mirrors C#’s “sealed override” surface instead.
- Abstract classes may declare `abstract virtual` members; derived types must provide an `override` before they become concrete. Constructors remain non-virtual.
- Within an overriding body, `base.Member(args…)` invokes the immediate base implementation. The parser treats `base` as a contextual keyword inside classes that specify a base type; using it elsewhere (or inside `static` members) yields a diagnostic so IDE tooling can highlight misuse.
- Properties follow the same rules: `virtual`/`override` modifiers apply to the property declaration and cover both accessors unless an accessor is explicitly marked `virtual`/`override`.
- MIR lowering records vtable slots for every virtual member and emits `CallVirtual` instructions for dispatch sites. Overrides reuse the original slot to preserve ABI stability; sealed overrides simply reuse the slot without emitting additional entries. Class layouts reserve a `$vtable` pointer at offset 0 so runtime code can load the dispatch table before touching user fields, and `.clrlib` manifests expose a `class_vtables` section (type name, slots, version hash) so stale binaries spot ABI drift during incremental builds.
- Borrow and ownership checking treat `base` calls as regular method invocations on the implicit `self` borrow, so the same lifetime and mutability rules apply.

#### Object Creation & Initialisers

- The `new` operator constructs class instances and value types. The canonical form is `new TypeName(arg1, arg2, …)`; the argument list is optional when the selected constructor takes no parameters.
- Type arguments follow the identifier (`new Vec<int>(capacity: 4)`); the parser rejects `new` expressions that omit required generic parameters.
- The compiler resolves the constructor overload using the same rules as other invocations (named/positional arguments, generics, accessibility). Structs without an explicit `init` surface expose an implicit parameterless constructor that zeros every field.
- Object initialisers extend the `new` expression with a trailing block: `new Point(x, y) { Z = 5, Label = "origin" }`. Member assignments inside the braces execute *after* the constructor body and in source order. Each entry must target a field or property on the constructed type (or an accessible base member); missing or duplicate members trigger diagnostics.
- Property setters marked `init` remain callable inside the initialiser block but reject writes elsewhere. Required members must appear either in the constructor argument list or the initialiser block—omissions raise the “required member not assigned” error described below.
- Collection initialisers (`new List<int> { 1, 2, 3 }`) desugar to repeated calls to `Add`. Mixing member assignments and collection elements is an error.
- Value type initialisers behave identically to class initialisers: the struct storage is first zeroed, the designated constructor runs (when present), then the member assignments execute with by-reference semantics so definite-assignment analysis can see partially initialised fields.
- MIR lowering evaluates the constructor arguments left-to-right, emits the allocation/stack slot, invokes the selected `init`, and then lowers each initialiser entry to a dedicated assignment. Borrow checking tracks the aggregate across the sequence so uses before full initialisation are rejected.
- Code generation relies on the same MIR form: LLVM and WASM backends receive the constructed lvalue and the ordered assignment list, ensuring parity across targets and eliminating the need for runtime reflection helpers.

#### Named Arguments

Chic supports C#-style named arguments across free functions, methods (instance and static), constructors, and generic invocations:

```chic
public class Logger
{
    public static void Write(int level, string message) { /* … */ }
}

public int Accumulate(int initial, int delta, bool saturate) { /* … */ }

public int Use(Logger logger)
{
    Logger.Write(message: "booting", level: 2);
    return logger.Accumulate(saturate: true, delta: 4, initial: 0);
}
```

- Arguments are evaluated left-to-right in the order they are written, preserving Chic’s expression semantics, and are then bound to parameters by name.
- Positional arguments must appear before the first named argument. Mixing named and positional arguments after the split (for example, `Call(x: 1, 2)`) produces a lowering diagnostic.
- Each parameter may be supplied at most once. Duplicated names (e.g. `Call(value: 1, value: 2)`) trigger targeted diagnostics with precise spans.
- Unknown names are rejected with an error keyed to the offending argument.
- MIR lowering reorders the evaluated operands so that backends always see the canonical parameter ordering. Debuggers and IDE tooling observe the final call site with the resolved parameter names instead of the source order.
- Static method calls and constructor invocations resolve the containing type before matching names, so overload resolution and diagnostic messages reference the fully qualified symbol (for example `Sample::Factory::Create`).
- Optional and defaulted parameters (once available) participate naturally: positional arguments continue to fill the leading required slots, and named arguments may skip over optional parameters by specifying later ones explicitly.
- Calls routed through function pointers or unresolved overload groups still accept named arguments; the compiler requires a unique match before code generation so ambiguous or unresolvable invocations emit diagnostics instead of falling back to positional binding.

These rules mirror the expectations from modern C# while retaining Chic’s explicit ownership model and evaluation guarantees, making named arguments safe to use in both high-level APIs and systems-facing code.

#### C#-Style Properties

Properties provide C#-like field encapsulation while staying compatible with Chic's explicit `this` parameter model:

```chic
public class Person
{
    public string Name { get; init; }

    private int _age;
    public int Age { get => _age; private set => _age = value; }

    public string DisplayName
    {
        get { return $"{Name} ({Age})"; }
        private set { _cachedDisplayName = value; }
    }

    public string Inline => $"{Name} – {Age}";

    private string _cachedDisplayName;
}
```

- A property declaration appears in class and interface bodies: `<visibility> [modifiers] <type> <name> { <accessors> }`. The `static`, `virtual`, `override`, `sealed`, and `abstract` modifiers follow the same rules as methods. Interfaces may only use declaration-style accessors (no bodies).
- Accessors are introduced with `get`, `set`, or `init` followed by one of three bodies:
  - `;` declares an **auto-accessor** and triggers generation of a hidden backing field named `__property_<Name>`.
  - `=> expression;` is an expression-bodied accessor. `get` expressions return the value; `set`/`init` expressions evaluate for their side-effects and implicitly end with `return;`.
  - `{ ... }` is a block-bodied accessor. Setters receive an implicit `value` parameter typed to the property; the block may reference `value` directly.
- Expression-bodied properties (`public T Foo => expr;`) are sugar for a property with a single `get` accessor using `expr`.
- At least one accessor is required. `set` and `init` are mutually exclusive for a given property.
- Accessor-specific visibility may be supplied using Chic access modifiers (for example, `public string Name { get; private set; }`). The accessor visibility must be at least as restrictive as the property visibility.
- `init` accessors behave like setters that may only be invoked during construction. The compiler rejects `init` calls outside designated constructors.
- Static properties share their backing storage across the declaring type and never receive an implicit receiver. Accessors behave like static methods (`get_`/`set_`/`init_`) and the compiler automatically elides the `this` parameter that instance accessors use.
- The lowered surface produces methods named `get_<Name>`, `set_<Name>`, and `init_<Name>` alongside the hidden field (when auto-accessors are present). This naming matches the exported C ABI and Chic metadata so C# and other interop consumers can bind to property accessors directly.
- Property declarations may carry attributes and XML documentation in the same manner as other members. Accessor-level attributes are reserved for future work.

#### Static Fields & Properties

- Static members are scoped to the declaring type. A program observes at most one storage slot per member regardless of how many instances of the type exist at runtime.
- Interfaces may not declare statics and the `required` modifier is rejected on static fields or properties because initialisation happens eagerly when the module is loaded.
- Field initialisers must be compile-time constants. When no initialiser is provided the compiler zero-initialises the storage. Static auto-properties synthesise a hidden backing field that inherits the `static` modifier, so getters and setters operate on the shared slot automatically.
- Access uses the `Type.Member` notation and is subject to the same visibility rules as instance members. A `readonly` static field may only be assigned from within the declaring type (either in an initialiser or in static constructors) to preserve immutability.
- Property accessors are lowered to static methods (`get_/set_/init_`). Call sites never pass a receiver and the compiler ensures auto-properties forward to their generated backing field.

#### Required Members

- Apply the `required` modifier to instance fields or properties when callers must initialise them. Required members are permitted on classes and structs; interfaces and static members reject the modifier.
- Constructors must assign every required member along all paths (either directly or through property accessors). If a convenience initializer delegates with `: self(...)`, it inherits the requirement from the target.
- Object and struct initialisers enforce the contract: an initializer such as `new Holder { }` fails when `Holder` exposes a required field or property that is not mentioned.
- Required members participate in inheritance. Derived types must honour required members declared on their base classes before construction completes.
- Auto-properties with `required` typically combine with `init;` accessors so callers can satisfy the requirement without exposing a mutable setter.

```chic
public struct Point
{
    public required int X;
    public int Y;
}

public class Holder
{
    public required int Value { get; init; }
}

// OK: every required member is assigned
var point = new Point { X = 10 };
var holder = new Holder { Value = 42 };

// Error: missing `X`
var invalid = new Point { Y = 5 };
```

#### Dependency Injection Attributes

Chic reserves a trio of surface attributes for the native dependency-injection pipeline described in [docs/runtime/di_design.md](docs/runtime/di_design.md). The frontend parses these annotations as raw metadata; a staged pass in the macro expander then interprets them and stamps the semantic model (classes, constructors, properties, and individual parameters) so later compiler passes can validate registrations and wire the runtime container without reflection.

- `@service` marks a concrete class as a container-managed service. Optional arguments configure metadata:
  - `lifetime: Transient | Scoped | Singleton | ThreadLocal` (defaults to `Transient`).
  - `named: "Identifier"` registers the implementation under a well-known name as well as the type.
- `@module` attaches to classes that group container registrations (for example, static `Configure(ContainerBuilder)` helpers). The attribute does not accept arguments; duplicates are rejected during staging. The bootstrapper currently accepts any class shape, but the DI type checker will eventually enforce that modules expose static entry points only.
- `@inject` decorates constructors, properties, or individual parameters that should participate in DI. Optional arguments refine resolution:
  - `lifetime: …` requests a specific scope for the dependency (useful when injecting factories).
  - `named: "Identifier"` resolves a specific named registration.
  - `optional: true` suppresses errors when the service is missing, producing `null`/`None` instead.

Both `name: value` and `name = value` forms are accepted for attribute arguments. The staged attribute pass emits diagnostics for unknown lifetimes or duplicate attributes, ensuring later passes receive a consistent metadata model.

```chic
@service(lifetime: Singleton, named: "UserRepo")
public class Repository
{
    private readonly HttpClient _client;

    @inject
    public init(@inject HttpClient client)
    {
        _client = client;
    }

    @inject(optional = true, named = "logger")
    public ILogger Logger { get; init; }
}

@module
public sealed class Registrations
{
    public static void Configure(ContainerBuilder builder)
    {
        builder.Register<Repository>();
    }
}
```

Later compiler work teaches the type checker, MIR builder, and runtime to consume this metadata; the bootstrap parser already records precise spans so those passes can surface targeted diagnostics.

The current bootstrap implementation validates DI metadata during type checking:

- Each `@service` class is recorded in the MIR manifest alongside its dependencies so later stages can emit registration tables.
- Constructor and property injection sites must resolve to known services unless marked `optional`; missing registrations raise `DI0001` diagnostics.
- Singleton services cannot depend on narrower lifetimes (e.g., `Scoped`), producing `DI0002` diagnostics. Using the experimental `ThreadLocal` lifetime triggers `DI0003` until runtime support lands.

### 2.4 Extension Methods

```chic
namespace Geometry.Extensions;

public extension Point
{
    public string ToText(in this) => $"({this.X}, {this.Y})";
    public void Translate(ref this, int dx, int dy)
    {
        this.X += dx;
        this.Y += dy;
    }
}

public extension<T> Wrapper<T>
{
    public T Get(in this) => this.Value;
}
```
- Extension declarations live at namespace scope and may introduce generic parameters (`extension<T> Wrapper<T>`). A `where` clause can follow the target type when constraints are required; `when` clauses must use the form `Self : Interface` (**DIM0002**).
- Prefix an extension method with `default` to author a namespace-scoped default implementation (`public default double Area(in this) => 0;`). Default methods must include a body; declarations that end with `;` remain helper-only, and they are only legal on interface targets (**DIM0001**).
- Append an optional `when` clause after the target to gate the block on constraints (`public extension Shape when Self : IRenderable, Self : IAsyncRenderable { ... }`). Each clause follows `TypeExpr : ConstraintType` semantics and must succeed before the defaults become eligible during dispatch.
- The first parameter must be named `this`. The parser treats `this` as a `Self` placeholder; type checking rewrites it to the target type (including generics) so MIR lowering and codegen see concrete layouts. The receiver may be passed by value, `in`, `ref`, or `out`, and the borrow checker enforces the chosen mode.
- Inside the body the receiver behaves exactly like the implicit `this` in class methods. Any other use of `Self` in the signature—return type, nested generic arguments, or pointer projections—is rewritten to the underlying target before code generation.
- Extension methods participate in the normal overload pipeline. Native instance members win ties, and ambiguous matches emit diagnostics instead of silently choosing an extension.
- Callers either qualify the namespace (`Geometry.Extensions.PointExtensions.ToText(in point)`) or bring it into scope with `using Geometry.Extensions;` and then write `point.ToText()`.
- Targets must be visible structs or classes. Attempting to extend enums, interfaces, or unknown symbols produces targeted errors.

#### Object Construction & Initializers

`new` expressions allocate instances of structs or classes and optionally run initializer blocks. The surface matches modern C# syntax while adopting Chic naming/namespace rules:

```
new Namespace.Widget<int>(capacity: 4) { Size = 42, Owner = CreateOwner() }
new Buffer<int> { 1, 2, seed }
```

- **Type operand.** The parser accepts any `TypeExpr`, including namespace-qualified names, generic instantiations, tuple types, pointer projections (`*`, `ref`, `ref readonly`), and nullable suffixes. The operand’s span is recorded independently from the overall expression so diagnostics can highlight invalid or inaccessible types precisely.
- **Constructor arguments.** Argument lists reuse the call-expression grammar (positional/named arguments, `ref`/`in`/`out` modifiers, inline bindings). The entire parenthesised span is captured and attached to `ExprNode::New`, enabling constructor-resolution diagnostics to underline the exact call.
- **Initializer blocks.** An optional `{ ... }` block may follow the argument list (or the type when no arguments exist):
  - *Object initializers* consist exclusively of `Identifier = Expression` entries. Each entry records the span of the name, value, and `name = value` tuple so missing/duplicate-member diagnostics can quote the right range.
  - *Collection initializers* consist exclusively of expressions (`new List { 1, 2, seed }`). Mixing named entries with bare expressions is rejected with a parser error; contributors should split the initializer or convert expressions into explicit assignments.
  - Braces are always required around the initializer block, even when empty; `new Foo { }` is valid, whereas `new Foo ;` is not.
- **Eligible targets.** Only classes and structs that are visible at the call site may be constructed. Attempting to use `new` with interfaces, traits, abstract classes, or unknown identifiers produces targeted diagnostics during type checking. Value types may omit parentheses when no constructor arguments are supplied (`new Point { X = 1 }`).
- **Collection semantics.** Collection initializer lowering (Section 3.67.3) will expand each element expression into a call to `Add` (or the appropriate pattern). The parser enforces the structural requirements now so later stages can assume homogenous entries.
- **Constructor selection.** Constructor overload resolution mirrors Chic call semantics. Positional arguments bind left-to-right, named arguments must follow positional ones, modifiers (`ref`, `in`, `out`) must match the declared parameter mode, defaults are honoured, and accessibility rules apply (`public`, `internal`, `protected/internal`, `private`). Violations yield:
  - `[TCK130]` when the target type is not constructible (e.g., interface/trait),
  - `[TCK131]` when no accessible constructor matches the arguments,
  - `[TCK132]` when multiple overloads match.
- **Runtime lowering.** Reference types allocate storage via `chic_rt_object_new(type_id)` before invoking the selected constructor. The helper consults the registered type metadata, zeroes the allocation, and returns a pointer aligned to the recorded layout. Value types reserve stack storage (emitting a `Deinit` marker so the borrow checker tracks the partially initialised aggregate) and reuse the same place throughout constructor calls and initializer rewrites. Constructors always receive the constructed receiver as an implicit `out self` argument—either the heap pointer returned by the allocator or the stack location for structs—so user code can assign to `self.Member` consistently across invocation forms (`self(...)`, `base(...)`, or `new Type(...)`). When no constructor exists and no arguments are supplied, the allocator/stack reservation result flows directly into the initializer block (if present), producing the default zero-initialised instance.
- **WASM executor parity.** The WASM runtime imports `chic_rt.object_new` and now mirrors LLVM’s allocation path: it zero-fills the heap block, writes the `$vtable` pointer recorded in `TypeMetadataEntry`, and invokes constructors so auto-property backing fields and required-member defaults are in place before accessors run. `tests/codegen_exec/error.rs::wasm_properties_execute_correctly` validates the behaviour.
- **Object initializer semantics.** Each `name = value` entry must refer to an instance field or property on the constructed type (or its base classes). Writes observe the same accessibility lattice used by the constructor. Additional guarantees:
  - Duplicate entries are rejected (`[TCK137]`).
  - Fields must be writable and instance (`readonly` or `static` mutations emit `[TCK135]`/`[TCK134]` respectively).
  - Properties must expose a `set` or `init` accessor; attempting to assign to getter-only properties raises `[TCK135]`.
  - Value-type initializers must cover every `required` member across the struct and its base types; the checker reports `[TCK136]` with the missing member list when the object/collection initializer fails to do so.
- **Init-only setters.** Accessors marked `init` are only writable during object initialization and obey property-level visibility. They count as satisfying required-members checks and suppress `[TCK135]` so long as the initializer entry appears.

##### Examples

```chic
namespace ObjectInitBasic;

public class Window
{
    public int Width;
    public int Height { get; init; }
}

public struct Dimensions
{
    public required int Width;
    public required int Height;
}

public class Builder
{
    public Window Configure() => new Window { Width = 800, Height = 600 };

    public Dimensions Defaults() => new Dimensions { Width = 4, Height = 2 };
}
```

- The class initializer assigns both a mutable field and an `init`-only property; `[TCK135]` would only surface if `Height` was missing or attempted outside the initializer block.
- The struct initializer covers every `required` member. Omitting either entry triggers `[TCK136]` naming the missing member.

```chic
namespace ObjectInitCollection;

public class Bucket
{
    public int Sum;

    public void Add(int value)
    {
        self.Sum += value;
    }
}

public class Builder
{
    public Bucket Build()
    {
        return new Bucket { 1, 3, 5 };
    }
}
```

Each collection element lowers to `Bucket.Add(element)` after construction. Failing to provide an `Add` member surfaces during MIR lowering/codegen; see `tests/object_initializers.rs` for the integration regression that executes the program on both WASM and LLVM backends.

The richer metadata (keyword span, type span, argument span, initializer entry spans) is surfaced through `Expression::as_new_expr()` for every AST node that stores an expression. Downstream phases rely on this data to implement constructor resolution, definite assignment, diagnostics, and MIR lowering.

#### Array initialization

Chic supports C#-style single-dimensional array construction with explicit element types. The grammar remains LL(1); unsupported sugar emits targeted diagnostics with a suggested replacement.

- **Supported forms:** `new T[n]` (length required), `new T[] { e1, e2, ... }` (length inferred from the element count), and `new T[n] { e1, e2, ... }` (length must match the initializer count). Jagged arrays are expressed as arrays-of-arrays (`new int[][] { new int[] { 1 }, new int[] { 2, 3 } }`) and follow the same rules per dimension. Collection-expression literals are first-class: `[e0, e1, ...]` (optional trailing comma) produces an owned container without requiring `new`.
- **Array literal semantics:** `[e0, e1, ...]` evaluates each element left-to-right exactly once. Container selection is deterministic:
  - If a prefix type is supplied (`T[] [e0, e1]` or `Vec<T> [e0, e1]`), that container is constructed.
  - Otherwise, the literal constructs a `T[]`. Conversions to `Span<T>`/`ReadOnlySpan<T>` borrow from that array (the allocation is explicit in MIR and follows normal lifetime/async rules).
  - Empty literals (`[]`) require a contextual element type; otherwise emit `TYPE0705 cannot infer element type for empty array literal`.
  - Nested literals are allowed and follow the same rules (`int[][] [[1], [2,3]]`).
- **Type rules:** element types must unify to a single `T` using the standard implicit conversions (numeric widening only; no implicit downcasts). Heterogeneous elements that cannot unify produce a diagnostic that cites the conflicting indices and types. Length expressions remain `usize`-convertible. When a length and initializer are both supplied, the length must be a compile-time constant that exactly matches the initializer count; mismatches produce a diagnostic instead of a runtime check.
- **Unsupported sugar (diagnostics):** multi-dimensional rank specifiers (`new int[2,3]`, `[, ]` suffixes) remain rejected with “rectangular arrays are not supported; use jagged `T[][]`.” Brace-only initialisers (`let xs: int[] = { 1, 2 };`) are rejected with a replacement suggesting `[1, 2]`. `new T[]` without a length or initializer is rejected.
- **Evaluation order and defaults:** the length expression (when present) is evaluated once, then initializer elements (or literal elements) are evaluated left-to-right. Arrays are default-initialised before any element stores: value types zero-initialize, nullable references become `null`, and aggregates run their default constructors. Initializer writes then overlay those defaults in order. If evaluation of an element fails, already-written elements are scheduled for drop and the array does not advance its length past the last successfully initialised slot. Vec-backed literals reserve capacity once, then push elements in source order.
- **Backend parity:** LLVM and WASM observe identical layout, length, and initializer behaviour. Bounds and length diagnostics are emitted during lowering; no hidden runtime reflection or dynamic construction participates in array creation.

### 2.5 Primitive Types

Chic follows the C# family of built-in type names. These aliases map to fixed-width, two's-complement or IEEE-754 representations and are always available without qualification. Signed and unsigned variants share identical bit widths.

| Keyword  | Bits | Underlying form | Range / semantics |
|----------|------|-----------------|-------------------|
| `sbyte`  | 8    | signed integer  | -128 to 127 |
| `byte`   | 8    | unsigned integer| 0 to 255 |
| `short`  | 16   | signed integer  | -32,768 to 32,767 |
| `ushort` | 16   | unsigned integer| 0 to 65,535 |
| `int`    | 32   | signed integer  | -2,147,483,648 to 2,147,483,647 |
| `uint`   | 32   | unsigned integer| 0 to 4,294,967,295 |
| `long`   | 64   | signed integer  | -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807 |
| `ulong`  | 64   | unsigned integer| 0 to 18,446,744,073,709,551,615 |
| `float16`| 16   | IEEE-754 binary16 | ≈3–4 decimal digits precision; signed zero and NaN payloads preserved |
| `float`  | 32   | IEEE-754 binary32 | ≈6–7 decimal digits precision |
| `double` | 64   | IEEE-754 binary64 | ≈15 decimal digits precision |
| `float128` | 128 | IEEE-754 binary128 | ≈33–34 decimal digits precision; 1 sign bit, 15-bit exponent (bias 16383), 112-bit significand |
| `decimal`| 128  | base-10 decimal   | 28–29 significant digits (`±7.9 × 10²⁸`) |
| `bool`   | 1    | logical          | `false` (0) or `true` (1); layout ABI uses 1 byte |
| `char`   | 32   | unsigned scalar  | Unicode scalar value (U+0000–U+10FFFF, excluding U+D800–U+DFFF) |
| `int128` | 128  | signed integer   | −2¹²⁷ to 2¹²⁷−1 |
| `uint128`| 128  | unsigned integer | 0 to 2¹²⁸−1 |

Binary floating-point types (`float16`, `float`, `double`, `float128`) follow IEEE 754-2019 semantics with default rounding mode *nearest, ties to even*. Unsuffixed real literals continue to infer `double`; explicit suffixes select `float16`/`float`/`double`/`float128` via `f16`/`f32`/`f64`/`f128`. NaN payloads and signed zero are preserved through parsing, MIR, codegen, and runtime formatting. `float128` maps directly to IEEE 754-2008 binary128 (1 sign bit, 15-bit exponent with bias 16383, 112-bit significand); the LLVM backend lowers this to `fp128` while the WASM backend downcasts to `f64` when `CHIC_FLOAT128=emulate` (default on wasm) and otherwise emits a diagnostic. `float16` is represented as IEEE binary16; LLVM lowers it to `half`, and the WASM backend currently rejects half-precision literals with a diagnostic because the execution/runtime path does not yet emulate half instructions.

Additional built-ins such as `nint` and `nuint` will be specified alongside the richer type system, but the aliases above are stable and should be used for numeric interoperability. All primitive names participate in the type system exactly like user-defined identifiers (they are not reserved words) and can appear in generic arguments, pointer types, and `ref`/`in` signatures.

#### Ref types

Chic exposes *first-class* references via the `ref` and `ref readonly` type constructors:

- `ref T` denotes a mutable alias to a storage location containing `T`. The alias is pointer-sized, may not be `null`, and can only be created when the compiler can prove a unique borrow of the source (for example, from a `ref`/`out` parameter, a mutable receiver, or another `ref T` value).
- `ref readonly T` denotes a read-only alias to `T`. Shared borrows (`in` parameters, temporaries, `ref readonly` expressions) automatically produce this type. A mutable `ref T` can always be assigned to `ref readonly T`, but the reverse conversion is rejected because it would allow mutation through a shared borrow.
- Ref types are ordinary type expressions, so they can appear in field declarations, properties, locals, and return types: `public ref readonly Span<byte> Data => ref readonly _buffer;`.
- `ref` values are produced by the unary `ref`/`ref readonly` operators. The parser accepts either form, but most code only needs `ref`—the compiler infers `ref readonly` whenever the source is borrow-only.
- Assignments require both the alias kind *and* the referent type to match. Attempting to store `ref readonly int` inside a `ref int` local (or returning it from a `ref int` method) emits a lowering diagnostic explaining that mutable refs require unique borrows. Similarly, `ref readonly string` cannot be converted to `ref readonly int` because the referent types differ.
- Ref values behave like other second-class borrows: they are pointer-sized, participate in auto traits/ABI metadata, and can flow across calls. It is the caller’s responsibility to ensure the referenced storage outlives the alias.

Example:

```chic
public ref readonly int Front(ref Span<int> span)
{
    ref readonly int head = ref span[0];
    return head;
}

public ref int Value(ref readonly MutexGuard<int> guard)
{
    // ERROR: guard exposes a read-only reference, so a mutable `ref` cannot be formed.
    ref int alias = ref guard.Value;
    return ref alias;
}
```

#### Numeric literal syntax

Chic follows C#/Rust conventions for numeric literal forms and suffixes.

- Decimal literals have no prefix; binary and hexadecimal forms use `0b`/`0B` and `0x`/`0X` respectively. A leading zero without a prefix keeps the literal in base 10—octal literals are not recognised.
- Digit separators (`_`) may appear between digits in the integral, fractional, and exponent portions of a literal. Separators cannot be the first or last character of any portion and may not sit directly next to the decimal point, exponent sign, or suffix. Separators are allowed immediately after a base prefix (`0x_FF`, `0b_0101`) to match Rust.
- Literal suffixes are case-insensitive and expand the C# surface area with Rust-style width specifiers:

| Suffix(es) | Primitive type | Notes |
|------------|----------------|-------|
| `i8`, `i16`, `i32`, `i64`, `i128`, `isize` | `sbyte`, `short`, `int`, `long`, `int128`, `nint` | `isize` resolves to the platform pointer-sized signed integer. |
| `u8`, `u16`, `u32`, `u64`, `u128`, `usize`, `u` | `byte`, `ushort`, `uint`, `ulong`, `uint128`, `nuint`, `uint` | A bare `u` is treated as `u32`. |
| `l`, `L` | `long` | Alias for the `i64` suffix. |
| `ul`, `uL`, `Ul`, `UL`, `lu`, `lU`, `Lu`, `LU` | `ulong` | Alias for the `u64` suffix. |
| `f16` | `float16` | Half-precision binary float. |
| `f`, `f32` | `float` | Single-precision binary float. |
| `d`, `D`, `double`, `f64` | `double` | Unsuffixed real literals also default to `double`. |
| `f128`, `q`, `quad` | `float128` | Quad-precision binary float. |
| `m`, `M` | `decimal` | Base-10 decimal literal (see below). |

- Suffixes that do not match the literal form (for example `1.0u32`, `0b1010m`, or `1e10f32` on a hexadecimal literal) trigger diagnostics during lexing. The unary minus is a separate token—`-1` is parsed as the negation of the positive literal `1`.
- Type inference for unsuffixed literals remains unchanged: integers default to `int`/`uint` depending on context, reals default to `double`, and literal suffixes override the default target when present.
- Literal suffixes persist through MIR and backend lowering. LLVM selects the matching IR type (`i8`…`i128`, `half`, `float`, `double`, `fp128`) when suffixes are present, while the WASM backend chooses between `i32.const`, `i64.const`, `f32.const`, and `f64.const`, inserting `f32.demote_f64`/`f64.promote_f32` conversions when the contextual type differs. Half-precision literals are rejected on WASM until emulation lands; quad-precision literals downcast to `f64` when `CHIC_FLOAT128=emulate` (otherwise they raise a diagnostic). The WASM backend currently rejects integer literals wider than 64 bits with a diagnostic.
- Diagnostics: invalid suffix/base combinations are rejected during lexing, while `[TCK120]` reports suffix/type mismatches when binding a literal and `[TCK121]` reports literal overflow relative to its suffix-specified width.
- Explicit suffixes must match the destination type; for example, `1u8` cannot initialize an `int` without an explicit cast. Values that exceed the suffix's range (such as `300u8`) are rejected during semantic analysis.

Examples:

```cl
let mask   = 0b_1111_0000u8;
let amount = 1_000_000i64;
let ratio  = 1.0e-3f32;
let big    = 0x_FFFF_FFFF_FFFFu64;
let money  = 12_345.67m;
```

#### Numeric helper APIs

`Std.Int32`, `Std.UInt32`, `Std.Int64`, `Std.UInt64`, `Std.IntPtr`, and `Std.UIntPtr` expose a common set of helper APIs that replace earlier runtime shims:

- **Checked arithmetic:** `TryAdd`, `TrySubtract`, `TryMultiply`, and (for signed types) `TryNegate` accept two operands plus an `out` parameter. They return `true` on success and leave the `out` value initialised; overflow or an invalid argument returns `false` and zeroes the destination. Pointer-sized variants dispatch to the appropriate 32- or 64-bit helpers using `NumericInfo.PointerBitWidth`.
- **Bit operations:** `LeadingZeroCount`, `TrailingZeroCount`, and `PopCount` operate in the native width of the type. `RotateLeft`/`RotateRight` perform modular rotates, `ReverseEndianness` swaps byte order, and `IsPowerOfTwo` reports whether a value is a strict positive power of two (zero and negative inputs return `false`).
- **Formatting:** `Format(string format, string culture)` is the primary surface; `ToString()` delegates to `Format(null, null)` and the overloads `ToString(string format)`/`ToString(string format, string culture)` simply pass through. Supported specifiers are:
  - Integers/pointers: `G`/`g` (general), `D`/`d` (decimal with optional zero-padding width), `X`/`x` (hexadecimal using two’s-complement for signed values, defaulting to the native width when negative), and `N`/`n` (grouped decimal with optional fractional precision, default `0`). Binary specifiers are not supported. Pointer-sized integers honour the active pointer width for grouping/hex padding.
  - Floating-point and `decimal`: `G`/`g` (general), `F`/`f` (fixed-point), `N`/`n` (fixed-point with grouping), and `E`/`e` (exponential) with optional precision digits (`"F2"`, `"E3"`, etc.). Null/empty formats map to `G`.
  - Cultures are provided as strings: `""`/`null`/`"invariant"` use invariant separators (`.` decimal, `,` grouping), `"en-US"` mirrors invariant, and `"fr-FR"` uses `,` as the decimal separator and a space as the grouping separator. Unknown cultures raise `ArgumentException`. Invalid specifiers raise `FormatException`. `ISpanFormattable`/`IUtf8SpanFormattable` expose matching `TryFormat` overloads that accept both the format and culture strings and return `false` when the destination buffer is too small.
- **Decimal intrinsics:** `Std.Numeric.Decimal.Intrinsics` exposes scalar/SIMD arithmetic (`Add`/`Sub`/`Mul`/`Div`/`Rem`/`Fma`) with explicit rounding modes and vectorisation hints, returning `DecimalIntrinsicResult` so callers can observe status/variant metadata. `Std.Numeric.Decimal.Fast::{Sum,Dot,MatMul}` provide span-based aggregations backed by typed const/mut pointer wrappers to the decimal ABI.

Each helper is implemented directly in Chic’s standard library (`Std.Numeric.*`) so the arithmetic, bit operations, and formatters execute entirely in Chic code. MIR lowering recognises these idioms and emits the corresponding target instructions (for example `llvm.uadd.with.overflow`, `ctpop`, `rotl`, WASM `i32.popcnt`) without routing through Rust shims or bespoke runtime intrinsics, keeping the numeric surface Chic-native end to end.

Generic math follows the static-abstract interface model mirrored from .NET 7: operator interfaces (`IAdditionOperators`, `ISubtractionOperators`, `IMultiplyOperators`, `IDivisionOperators`, `IModulusOperators`, `IUnaryPlusOperators`, `IUnaryNegationOperators`, `IIncrementOperators`, `IDecrementOperators`, `IComparisonOperators`, `IEqualityOperators`) surface the `op_*` members backing language operators. Aggregate interfaces layer those contracts: `INumberBase<TSelf>` is the root; `INumber<TSelf>` adds arithmetic/comparison/increment/decrement, identity/min/max, and parse/format helpers; `IBinaryNumber<TSelf>` extends `INumber<TSelf>` with `IsPowerOfTwo`; `IBinaryInteger<TSelf>` layers in bitwise/shift operators plus the count/rotate/endian helpers; and `ISignedNumber<TSelf>` carries signed-only behaviour (`NegativeOne`, unary `-`, `Abs`). `Std.Numeric.Int128` and `Std.Numeric.UInt128` are the reference patterns for implementing the full static-operator surface. Generic operator binding resolves against these interfaces—there is no runtime “Numeric” registry or helper type involved in operator resolution.

The runtime-visible intrinsic surface is the set of per-type helpers on the numeric structs themselves (`Std.Int32.TryAdd`, `Std.UInt64.RotateLeft`, `Std.IntPtr.TryAdd`, etc.). Backends recognise those symbols through metadata (`runtime::numeric::NUMERIC_INTRINSICS` plus the pointer-aware view) and lower directly to target instructions, while the implementations stay Chic-native in the stdlib helpers (`NumericArithmetic`, `NumericBitOperations`, `NumericFormatting`). No extra runtime adapter APIs are required.

| Type | Range/precision | Notes |
| --- | --- | --- |
| `decimal` | `±79228162514264337593543950335` with scale `0..28` | Fixed-point base-10, ties-to-even rounding, total ordering, no NaN/Infinity. |

Pointer-sized helpers (`IntPtr`, `UIntPtr`, `nint`, `nuint`) and the decimal surface all live under the same `Std.Numeric` namespace. `Std.Numeric.UIntPtr` now mirrors the signed pointer API with `Parse`/`TryParse` (string + UTF-8 span), `TryAdd`/`TrySubtract`/`TryMultiply`, bit operations, rotation, endianness swapping, and formatting overloads that respect the active pointer width. `Std.Numeric.Decimal` (and the SIMD-friendly `Std.Numeric.Decimal.Fast` wrappers) provide the high-level decimal APIs, so there is no parallel `Std.Decimal` namespace to maintain. All numeric structs carry the same `readonly`/`@Intrinsic`/`@StructLayout(LayoutKind.Sequential)` metadata, which keeps codegen, reflection, and auto-trait analysis aligned across CLR and Chic targets without leaning on Rust runtime shims.
`Std.Numeric.UIntPtr` centralises the sanctioned pointer/int conversions:
`FromPointer<T>` / `FromConstPointer<T>` pair with `.AsPointer<T>` /
`.AsConstPointer<T>` to round-trip raw pointers through opaque handles, and the
`AddressOf<T>` / `AddressOfConst<T>` helpers expose `@expose_address` pointers
as raw `nuint` values. Both the parameters and the return types require
`@expose_address`, so provenance-erasing conversions cannot occur accidentally;
`PointerFromAddress<T>` / `PointerFromConstAddress<T>` plus `.Zero` provide typed
null pointers without spelling out casts. Tests (`tests/numeric_structs.rs`,
`mir::builder::tests::unsafe_pointers.rs`) enforce these requirements alongside
the broader unsafe contract (§4.4).
Unchecked pointer-sized casts live in `Std.Numeric.NumericUnchecked` and are intentionally split into explicit helpers to avoid overload ambiguity: `ToNintNarrow`/`ToNintWiden`/`ToNintFromPtr` plus `ToNintFromInt32`/`ToNintFromInt64`, and the corresponding `ToNuint{Narrow,Widen,FromPtr}` helpers. Formatting/parsing and pointer-int bridges must call these named helpers rather than relying on implicit or overloaded casts so LLVM/WASM agree on pointer-width semantics without raw pointer shims.

#### Decimal arithmetic

`decimal`—surfaced alongside the other `Std.Numeric` structs—is a 128-bit base-10 floating point value type intended for financial and other precision-sensitive workloads. The in-memory representation mirrors the CLR layout: a 96-bit unsigned significand, an 8-bit scale (`0`–`28`) indicating the decimal shift, a sign bit, and reserved padding. The bootstrap compiler stores the payload in a 16-byte struct with 128-bit alignment on all supported targets; ABI lowering passes the value by reference when the platform cannot move it in registers. The type implements the full generic-math surface (`INumber<T>`, `INumberBase<T>`, `ISignedNumber<T>`, `I*Operators`) via static abstract operators.

- Constructing a `decimal` from an integral type (`sbyte`…`uint64`, `int128`, `uint128`) is implicit when the destination can represent the operand exactly. Conversions in the opposite direction require an explicit cast or the appropriate `TryFrom` helper; overflow inside a `checked` context raises `OverflowException`, while `unchecked` truncates towards zero.
- Arithmetic operators defined on `decimal` (`+`, `-`, unary `-`, `*`, `/`, `%`) perform decimal rounding to the nearest representable value using banker's rounding (ties-to-even). Overflows raise `OverflowException`; division or remainder by zero raises `DivideByZeroException`.
- Comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`) use logical comparisons on the normalised significand and scale. Distinct encodings of the same numeric value compare equal after normalisation; no NaN or infinity sentinels exist.
- `decimal` participates in constant folding and macro evaluation. Compile-time evaluation uses the same rounding and overflow semantics as runtime execution; literal expressions that overflow emit diagnostics during constant evaluation.
- Parsing/formatting mirror the other numerics: `Parse`/`TryParse` accept decimal and fractional digits, optional underscores, and optional `e`/`E` exponents, trimming leading/trailing whitespace. Invalid text raises `FormatException`; out-of-range values raise `OverflowException`. `ToString`/`Format` plus span-based overloads support `G/g`, `F/f`, `N/n`, and `E/e`, honouring culture separators.
- Mixed-mode arithmetic between `decimal` and the binary floating-point types widens to `decimal` when the destination is `decimal` or when both operands carry the `decimal` literal suffix. Otherwise the compiler issues a diagnostic prompting the programmer to pick a target type explicitly; implicit conversions from `float`/`double` to `decimal` are rejected to avoid silent precision loss.

##### Literals and inference

Decimal literals reuse the real literal grammar but require an `m` or `M` suffix:

```
0m          // zero with scale 0
1.50m       // scale = 2
123_456m    // underscores allowed between digits
9.9e-4M     // exponent adjusts the scale before suffix is applied
```

- A decimal literal must contain at least one digit before and after the decimal separator when a fractional part is present. Exponents (`e`/`E`) shift the scale before the suffix is processed; literals that would exceed the allowed scale (`> 28`) or magnitude emit diagnostics.
- Unsuffixed real literals continue to default to `double`. Type inference picks `decimal` only when the literal carries the `m`/`M` suffix or when the contextual expected type is `decimal`.
- Literal suffixes interoperate with numeric separators (`_`) in the same manner as other numeric tokens. Separators may not appear adjacent to the suffix or around the decimal point.

The compiler lowers decimal literals into canonical significand/scale pairs at parse time so constant folding, code generation, and runtime helpers all observe the same representation.

Reference and value types are **non-nullable by default**. To opt into nullable semantics, append `?` to the type name: `string?`, `Vec<int>?`, or `MyRecord?`. CLR-style metadata treats these shapes as optional payloads whose layout matches `Option<T>` when `T` is nullable-aware. The `null` literal is a dedicated token that may only flow into nullable storage; attempting to assign it to `string` or `Vec<int>` emits a borrow-checker error. Nullable slots are default-initialised to `null`, whereas non-nullable slots continue to participate in definite-assignment analysis.

Additional syntax rounds out the surface model:

- `ref?`/`out?` parameters accept nullable references without forcing the callee to coerce the payload up front. The binding modifier records nullability alongside the parameter mode so name resolution, layout, and borrow checking can thread the metadata through.
- Conversions from `T` to `T?` are implicit, while converting from `T?` to `T` requires either a null-coalescing expression (`value ?? fallback`) or an explicit diagnostic suppression. The borrow checker tracks a three-state lattice (`null`, `non-null`, `unknown`) so dereferencing a `T?` value without first proving it non-null results in a targeted error.
- Nullable locals and temporaries carry a `null` sentinel alongside the payload. `StorageLive` resets nullable bindings back to the `null` state so the control-flow graph never observes partially-moved payloads.
- Every `T?` lowers to a two-field struct `{ HasValue: bool, Value: T }`. Writing `null` clears `HasValue` without mutating the payload; any successful assignment to a non-null value sets the flag and copies the payload by value. Value types therefore gain space for the discriminant, while reference types continue to store the pointer inside `Value` so existing ABI rules apply.
- The borrow checker keeps the flag and payload in lock-step: projecting through `Value` automatically requires a proven non-null state, and the checker refuses to move a nullable binding out of scope without first ensuring the sentinel is cleared or consumed.
- `null` participates in overload resolution like other literals. If multiple nullable overloads are viable, Chic prefers the candidate whose payload most closely matches the contextual type (for example, `string?` before `object?`).

These rules ensure the language can reason about null-safety without requiring a separate `Option<T>` surface form.

#### Strings

Chic distinguishes between owned and borrowed textual data:

- `string` is an owned UTF-8 buffer with layout `{ ptr: *mut u8, len: usize, cap: usize, inline_data: [u8; 32] }`. Inline strings set the high bit of `cap` and store their payload in `inline_data`; heap strings clear the high bit and store their payload in an allocation referenced by `ptr`. Moving a `string` transfers ownership; cloning duplicates the payload; dropping releases the allocation when heap-backed. The runtime guarantees `len <= (cap & !INLINE_TAG)` and validates UTF-8 on every mutating operation.
- `str` is an immutable UTF-8 slice with layout `{ ptr: *const u8, len: usize }`. `str` values never own their storage and instead borrow data from a `string` (or a static literal) subject to the borrow checker’s lifetime rules.
- `string.AsSpan()` and `str.AsSpan()` expose `ReadOnlySpan<char>` views that decode the UTF-8 payload into Unicode scalars; `string.AsUtf8Span()` preserves the raw byte view for interop. Substring/slicing helpers are expressed in terms of span slicing so the compiler no longer needs bespoke string logic.

Conversions follow Rust-like expectations:

- Borrowing a `string` via `in`/`ref` parameters or type inference yields a `str` tied to the borrow’s lifetime.
- Converting a `str` into an owned `string` requires an explicit constructor (`string::from(slice)`), which lowers to a runtime clone.
- String literals have type `str` with `'static` lifetime. Assigning a literal to a `string` performs an owned clone at runtime; binding directly to `str` keeps the shared segment.

#### Literal Forms

- **Character literals** use single quotes and accept the escape repertoire shared with strings: `\\0`, `\\'`, `\\"`, `\\`, and the standard control escapes (`\\a`, `\\b`, `\\f`, `\\n`, `\\r`, `\\t`, `\\v`). Unicode scalars are expressed via `\\uXXXX` (four hex digits) or `\\UXXXXXXXX` (eight hex digits); both forms are validated to ensure they denote a Unicode scalar value (U+0000–U+10FFFF excluding surrogate code points U+D800–U+DFFF). Each literal lowers to a single Chic `char` and is rejected when it would require multiple UTF-16 code units (for example, entering a lone high surrogate or supplying multiple user-perceived grapheme clusters).
- **Regular string literals** enclose text in double quotes and support the escapes above together with `\\x` (1–4 hex digits). Newlines must be escaped; the lexer rejects raw line breaks and invalid escape sequences with precise diagnostics.
- **Verbatim string literals** prefix the sequence with `@`, treat backslashes literally, and may span multiple lines. A doubled quote (`""`) emits a single `"`.
- **Interpolated string literals** prefix the sequence with `$` and allow `{ expression [, alignment] [: format] }` segments alongside literal text. Braces are escaped with `{{` and `}}`. During lowering the compiler emits structured interpolation segments (`text`, `str`/`string` copies, `bool`, `char`, signed/unsigned integers up to 64 bits, and `float`/`double`). Alignment and format clauses are normalised up front and preserved through MIR so the LLVM/WASM backends can dispatch to dedicated runtime helpers. Unsupported operand types or malformed format specifiers produce diagnostics, while 128-bit integers remain rejected on WASM until a wider lowering strategy is implemented.

Verbatim and interpolated prefixes can be combined in either order. Regardless of the source form, literal parsing normalises escape sequences to Unicode scalar values, folds the resulting text to NFC, and preserves the cooked payload through MIR so constant folding, diagnostics, and runtime interop operate on normalised strings. Unterminated strings, stray braces, and malformed escape sequences are diagnosed during lexing.

The runtime exposes FFI-safe projections:

- `string` → `struct chic_string { uint8_t *ptr; size_t len; size_t cap; uint8_t inline_data[32]; };`
- `str` → `struct chic_str { const uint8_t *ptr; size_t len; };`
- `char` → `typedef chic_char uint16_t; struct chic_char_span { const chic_char *ptr; size_t len; };`

Both layouts are encoded in MIR type metadata so LLVM/WASM backends can lower field accesses, ABI passing conventions, and runtime intrinsics consistently.
Diagnostic snippets render with grapheme-aware carets to avoid splitting multi-scalar characters (emoji flags, combined marks) mid-glyph.

#### Characters

`char` is a 16-bit unsigned value type that stores a single UTF-16 code unit. The compiler enforces “single code unit” invariants at every construction site:

- Literals must resolve to exactly one UTF-16 code unit. Attempts to embed multi-code-unit scalars (for example `'🇺🇳'` or `'\U0001F60A'`) produce a diagnostic.
- Escape sequences decode to raw code units during lexing. `\uXXXX` accepts any 16-bit value, including surrogate code units; `\UXXXXXXXX` rejects values above `0xFFFF`.
- Numeric casts between `char` and integral types are explicit and respect the 16-bit width. Values outside `0x0000`–`0xFFFF` are rejected during constant folding.

`sizeof(char)` evaluates to two bytes on all bootstrap targets, with 2-byte alignment. Casting between `char` and the integer primitives respects this width; surrogate code units are permitted as numeric values even though classification/casing helpers treat them as invalid scalars.

The standard library exposes the following interoperability rules:

- Indexing a `string` or iterating a `str` yields 16-bit code units. Enumerators surface UTF-8 decoding errors as panics—runtime support already guarantees the underlying storage contains well-formed UTF-8.
- `Span<char>` (and `ReadOnlySpan<char>`) are thin views over contiguous UTF-16 code units. Conversions between `string`/`str` and `Span<char>` decode UTF-8 storage into UTF-16 buffers for efficient traversal and interop.
- `Array<T>.AsReadOnlySpan()` produces a `ReadOnlySpan<T>` that borrows the backing buffer. The borrow checker synthesises a shared loan for the array while any view is live; attempts to mutate or uniquely borrow the array during that window are rejected with the standard conflicting-borrow diagnostic. Dropping or reassigning the view releases the loan, allowing subsequent mutation.
- Bridging to UTF-16 for interop uses runtime helpers that decode UTF-8 storage into code units; invalid UTF-8 payloads are rejected before reaching the runtime.
- Culture-aware casing and character classification live in `Std.Numeric.Char`; helpers operate on scalar values and report invalid status for surrogate code units.
- `Std.Numeric.Char` is the primitive wrapper for `char`, implements the numeric interfaces over an unsigned 16-bit value, and exposes classification (`IsDigit`, `IsLetter`, `IsWhiteSpace`), invariant casing helpers (`TryToUpperInvariant`, `TryToLowerInvariant`), conversion from raw code points, and `ToString(char)` for round-tripping into managed strings. See [docs/std_char.md](docs/std_char.md) for details.

Backends lower `char` as an unsigned 16-bit integer in ABI signatures and data layouts. This keeps string/spans ABI-compatible across LLVM/WASM while matching the UTF-16 code unit model.

### 2.5 Access Modifiers

Chic follows the C# visibility matrix and applies it uniformly to namespace items, nested types, and members. Visibility is encoded in the AST and enforced during MIR lowering and type checking. The keywords are:

| Modifier              | Applies to                               | Visibility scope |
|-----------------------|------------------------------------------|------------------|
| `public`              | All declarations                         | Visible to all assemblies. |
| `internal` (default)  | All declarations                         | Visible only inside the current Chic module/assembly. |
| `protected`           | Class members and nested types           | Visible to the declaring class and all derived classes (regardless of assembly). |
| `private`             | Namespace and type members               | Visible only to the declaring type (or file for namespace-level declarations). |
| `protected internal`  | Class members and nested types           | Union of `protected` and `internal`: accessible to derived classes or any code in the current assembly. |
| `private protected`   | Class members and nested types           | Intersection of `protected` and `internal`: accessible only to derived classes **within** the current assembly. |

Rules:

- Assembly/“same module” identity is the manifest `package.name`. Workspaces with multiple manifests create multiple assemblies even when namespaces overlap.
- Declarations without an explicit modifier default to `internal`.
- `protected`, `protected internal`, and `private protected` are rejected on value types (`struct`, `enum`, `union`) and free functions where inheritance does not apply.
- Namespace-level `private` declarations are visible only within the same compilation unit; other files must use `internal`.
- When a type appears in a signature (field, parameter, return type, base list, extension target, interface implementation, or attribute payload) the compiler verifies the referencing scope can legally access the type’s visibility. Violations emit diagnostics such as `field 'Value' references inaccessible type 'Access::Hidden' (private)`.
- `protected` access is granted to the declaring class, derived classes, and their nested types. Instance protected members may be used only through `this` or a receiver whose compile-time type is the accessing type (or a type derived from it); accessing `Base.Protected` through an arbitrary `Base` reference in `Derived` is rejected. The lowering pipeline records inheritance hierarchies so that `protected` checks continue to work after namespaces are flattened.
- `private protected` requires both: the consumer must derive from the declaring type **and** reside in the same assembly.

These semantics are enforced during layout registration and MIR lowering so that codegen and runtime never observe illegal references. Header generation, library packaging, and metadata emission only surface `public` declarations.

#### Cross-package examples

```chic
// Package: Core.Shapes
namespace Shapes;
public class Shape
{
    internal int InternalArea;
    protected int Color;
    protected internal int Metadata;
    private protected int Token;
}

// Package: App.Client (depends on Core.Shapes)
namespace Client;
import Shapes;

public class Circle : Shape
{
    public int Ok() => this.Color + this.Metadata;      // allowed (protected path)
    public int FailInternal() => this.InternalArea;     // error: internal is package-scoped
    public int FailPrivProt() => this.Token;            // error: private protected blocks cross-package derived access
    public int FailProtected(Shape other) => other.Color; // error: protected-instance rule (receiver not known to be Circle)
}

public class Snooper
{
    public int Breaks() => new Shape().Metadata;        // error: not derived, not same package
}
```

- **2025-12-28:** Clarified assembly = manifest package, added protected-instance rule, and documented cross-package behaviour for `protected internal` and `private protected`.

### 2.6 Value Unions

Chic adopts a first-class `union` construct inspired by the ongoing C# union design discussions, giving developers explicit control over overlapping layouts while staying within the language’s ownership/borrowing guarantees. Unions are value types: the entire storage moves by value, and the borrow checker enforces that only one *view* of the underlying bytes is active at a time.

#### Syntax

```
UnionDecl        :: = attributes? visibility? 'union' Identifier UnionBody
UnionBody        :: = '{' UnionMember* '}'
UnionMember      :: = attributes? visibility? UnionField ';'
                   |  attributes? visibility? 'struct' Identifier StructBody
UnionField       :: = UnionFieldModifier* Type Identifier
UnionFieldModifier :: = 'ref' | 'readonly'
```

- `UnionField` entries declare explicit views over the shared storage. Each field may be annotated with `readonly` to expose an immutable view of the underlying bytes.
- Inline `struct` definitions act as *implicit views*. Declaring `public struct Rgba { ... }` inside a union both introduces the nested type `Pixel.Rgba` and a view named `Rgba` whose type is that nested struct. The compiler qualifies the nested struct with the union’s namespace (`Pixel::Rgba`) and registers it in the type layout table just like a top-level struct.
- Attributes attached to the union or individual members participate in layout (`@repr(c)`, `@repr(packed)`) and tooling integration.

```chic
namespace Imaging;

public union Pixel
{
    // Inline views share the same storage buffer.
    public struct Rgba { public byte R; public byte G; public byte B; public byte A; }
    public struct Gray { public ushort Luma; }

    // Explicit field view with modifiers.
    public readonly Channels Channels;
}

public struct Channels
{
    public byte R;
    public byte G;
    public byte B;
    public byte A;
}
```

#### Layout & safety rules

- A `union` declares a fixed-size storage buffer along with one or more views of that buffer. Each view is indexed in declaration order and recorded in the MIR `TypeLayoutTable` with metadata describing the view’s type, modifiers, and layout participation.
- Only one view is considered *active* at a time. Constructing or assigning through a view activates it; reading from a different view without reinitialising the union produces a static diagnostic in safe Chic.
- Because Chic is 64-bit only, union layout follows the target triple’s natural alignment rules. The union size is the maximum of its view sizes rounded up to the largest alignment requirement among the views (or an explicit `@repr` attribute if supplied).
- `readonly` fields expose immutable views. They may participate in shared (`in`) borrows even when another readonly view is active, but cannot be used to mutate storage.
- `ref` fields project the union storage by reference. Assigning through the `ref` view does not change the active view, but borrowing rules treat it as a unique borrow covering the entire union payload.

#### Interaction with other features

- **Borrow checking:** The borrow checker tracks the active union view per local. Activating a new view releases existing borrows and invalidates reads of overlapping fields until the new view is dropped or reassigned. Attempting to borrow two distinct mutable views simultaneously is rejected.
- **Pattern matching:** `switch` statements recognise union patterns in the same way they recognise structs/enums. For inline views, the compiler rewrites `case Pixel.Rgba rgba:` into a view-specific projection that only succeeds when the active view matches.
- **Interop:** Unions honour representation attributes and may be exported through the C ABI. Generated C headers include `union` definitions with matching layout metadata.
- **Drop scheduling:** lowering records which view requires cleanup and later synthesises concrete
  `Deinit`/`Drop` statements so only the initialised view runs its destructor.

Future work will expand unions with explicit discriminants (`union Pixel { public view Rgba; public view Gray; }`) once the pattern-matching work in §1.10 is complete.

### 2.7 Data Parallelism

Chic targets multiple data-parallel execution styles:

- **SIMD** (Single Instruction, Multiple Data): Vector types (exposed later via `std.simd`) map to hardware vector registers with auto-vectorised loops when possible. Intrinsics will be provided for explicit control once MIR lowering is in place.
- **MIMD** (Multiple Instruction, Multiple Data): Tasks and threads run across cores via the async executor and thread pools (native Std executor + OS threads). Borrow rules apply per task, and the compiler enforces Rust-style ownership invariants at concurrency boundaries. Capturing or sending a value to another thread requires it to satisfy the language’s auto traits for thread safety (`ThreadSafe`, `Shareable`), preventing data races unless the developer opts into `unsafe`. Shared mutable state must flow through `std.sync` primitives (mutexes, atomics) that explicitly reintroduce mutability under synchronisation.
- **SIMT** (Single Instruction, Multiple Threads): GPU-style execution models are first-class. Kernels annotated with `@kernel` are lowered to GPU targets and compiled into PTX (Parallel Thread Execution) modules before the final backend assembles cubins for specific architectures. PTX serves as our virtual ISA, enabling JIT compilation on future GPU generations without rebuilding Chic applications. Host-side code schedules SIMT kernels via `std.gpu`, supplying launch parameters (grid/block sizes) and target selection (`cuda`, `metal`, `vulkan`) through `manifest.yaml`.
- **Wavefront SIMD** (AMD RDNA/CDNA): For AMD hardware we target the GPUOpen ISA described for RDNA (gaming) and CDNA (HPC). Kernels compile to AMD’s ISA packets (produced via LLVM AMDGPU backend) and are embedded alongside PTX so the runtime can pick `amdgpu` devices. Chic surfaces ISA selection with `@gpu_target(amdgpu)` attributes and derives wave size (32/64) at compile time.
- **Xe Threads** (Intel GPU): Intel’s Xe architecture (from Gen12 onwards) exposes Execution Units (EUs) arranged in subslices with hardware support for wide vector ALUs, DPAS/DPASW matrix instructions, and sampler/export pipes. We target Intel GPUs through Level Zero / oneAPI backends, emitting VISA or SPIR-V modules that the Intel stack compiles to Xe ISA. Developers opt in using `@gpu_target(intel_xe)`; the compiler selects subgroup sizes (typically 8/16/32) and maps `std.linalg` operations onto DPAS instructions when available.

Initial support focuses on CPU SIMD (x86_64 SSE/AVX, AArch64 NEON/SVE). Further hardware backends will be specified as the MIR and codegen work lands.

#### 2.7.1 SIMD vector types (language-defined)

Chic treats fixed-width SIMD vectors as first-class value types. The syntax is LL(1):

```
TypeExpr ::= "vector" "<" TypeExpr "," ConstIntExpr ">"
```

- `vector<T, N>` forms a concrete vector type whose element type is `T` and whose lane count is `N`.
- `T` must be one of: `bool` (mask vectors), `i8`/`u8`, `i16`/`u16`, `i32`/`u32`, `i64`/`u64`, `f32`, or `f64`.
- `N` must be a positive, const-evaluable integer expression. The compiler rejects zero or non-constant lane counts with `TYPE0701 VECTOR_LANES_MUST_BE_CONST`.
- The total width (`sizeof(T) * N`) must be one of {64, 128, 256} bits. Other widths emit `TYPE0702 VECTOR_WIDTH_UNSUPPORTED`.
- Layout: lanes are stored contiguously with no padding. `sizeof(vector<T, N>) == sizeof(T) * N`. `alignof(vector<T, N>) == max(alignof(T), vector_align(target, width_bits))` where `vector_align` is 16 bytes for 128-bit classes (SSE2/NEON/WASM v128), 32 bytes for 256-bit classes (AVX2/AVX-512), and falls back to `alignof(T)` when the backend scalarises. Struct and array layout use this alignment.
- Values are movable; `Copy` inference follows the element type (`vector<T, N>` is `Copy` when `T` is `Copy`).
- Literals use the constructor form `vector<float, 4>(1.0f, 2.0f, 3.0f, 4.0f)`. The argument count must match `N` and each argument must be implicitly convertible to `T`. There is no implicit array-to-vector conversion.
- Comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`) produce `vector<bool, N>` mask vectors. Mask lanes are `false`/`true` bytes (`0`/`1`) and support logical `&`, `|`, `^`, and `!`.
- Arithmetic and bitwise operators are lane-wise: `+`, `-`, `*`, `/` (float only), unary `-`, `%` (integer only), `&`, `|`, `^`, `~`, `<<`, `>>`. Mixed-element or mixed-lane operands are rejected.
- Reductions: `Sum`, `Min`, `Max`, `Any`, and `All` lower to Std intrinsics. `Any`/`All` operate on masks; the others operate on numeric vectors and return scalars of the element type (or `bool` for masks).
- Swizzles/shuffles: `Shuffle(vector<T, N> value, int[N] indices)` (exposed through `Std.Simd.Shuffle`) permutes lanes with wrap-around disallowed; out-of-range indices emit `TYPE0703 VECTOR_SHUFFLE_OOB`.
- Constructors and utilities live in `Std.Simd`: `Splat(T value)`, `FromElements(...)`, `Zero()`, `Load(ReadOnlySpan<T> src, int offset, SimdAlignment align)`, `Store(Span<T> dst, int offset, SimdAlignment align)`, `Blend(vector<T, N> a, vector<T, N> b, vector<bool, N> mask)`, and reductions (`Sum`, `Dot`, `HorizontalAdd`, `Min`, `Max`, `Any`, `All`).
- Feature gating:
  - Functions or types may declare ISA requirements via `@requires_isa("avx2")`, `@requires_isa("sse4.2")`, `@requires_isa("neon")`, `@requires_isa("sve2")`, or `@requires_feature("wasm_simd")`.
  - A stricter `@requires_simd` attribute forbids scalar fallbacks; if the target cannot satisfy the ISA list, type checking emits `TYPE0704 SIMD_BACKEND_UNAVAILABLE` with a suggested fallback (`@requires_simd` users must gate their code).
  - Without a strict attribute, the backend may scalarise to per-lane loops when the requested width is unsupported on the target. Scalarised code must preserve lane-wise semantics (including NaN/±0.0 behaviour) and keep layout stable.
- Backend mapping (normative):
  - **LLVM:** `vector<T, N>` lowers to LLVM vector types of the matching width. Shuffles map to `llvm.shufflevector`; reductions use the smallest set of native ops available for the target feature set.
  - **WASM:** `wasm_simd128` lowering and scalar fallbacks are not yet implemented. The backend rejects modules containing `vector<T, N>` with `TYPE0704` (or equivalent codegen error) until SIMD lowering lands. LLVM remains the supported path for SIMD today.
  - No Rust runtime symbols participate in SIMD semantics; all surface behaviour flows through `Std.Simd` and compiler-emitted intrinsics keyed on Std metadata.
- Diagnostics:
  - `TYPE0701 VECTOR_LANES_MUST_BE_CONST` — lane count is zero or not a const-evaluable integer.
  - `TYPE0702 VECTOR_WIDTH_UNSUPPORTED` — width is not 64/128/256 bits.
  - `TYPE0703 VECTOR_SHUFFLE_OOB` — shuffle indices fall outside `[0, N)`.
  - `TYPE0704 SIMD_BACKEND_UNAVAILABLE` — SIMD is required but the selected backend cannot lower vectors (or scalarisation is forbidden).
  - `TYPE0705 VECTOR_ELEMENT_UNSUPPORTED` — element type is outside the allowed set.
- Bootstrap status: the LLVM backend lowers `vector<T, N>` natively; the WASM backend rejects vectors until `wasm_simd128` lowering and scalar fallbacks are completed.

Please append dated change notes beneath this section as the specification evolves.

- **2025-12-23:** SIMD vectors are first-class (`vector<T, N>`), with lane-wise operators, mask semantics, Std.Simd constructors/utilities, target-feature gating (`@requires_isa`, `@requires_simd`), LLVM/WASM lowering rules, and deterministic scalar fallbacks when hardware SIMD is unavailable.
- **2025-12-24:** Bootstrap clarification — WASM backend currently rejects `vector<T, N>` (no scalar fallback yet); use LLVM for SIMD until WASM SIMD lowering lands.
- **2025-12-24:** Array literals (`[e0, e1, ...]` with optional trailing comma) are first-class, with deterministic container selection (contextual `Vec<T>`/`T[]`/`Span<T>`), prefix typing (`T[] [ ... ]` / `Vec<T> [ ... ]`), empty-literal inference diagnostics, and unified element typing rules. Existing `new T[] { ... }`/`new T[n]` forms remain supported.

#### 2.7.2 Concurrency guarantees

Chic deliberately aligns with Rust’s ownership-driven concurrency model. Any value that crosses an async suspension point, is moved into a thread/task, or is captured by a parallel closure must implement `ThreadSafe` (Rust’s `Send`). Shared references usable across threads must additionally implement `Shareable` (Rust’s `Sync`). These traits are auto-derived for types that contain only thread-safe members; the moment a field (or generic parameter) opts into non-thread-safe behaviour, the enclosing type stops implementing the traits. The borrow checker and type checker co-operate to reject:

- Moving a non-`ThreadSafe` type into `Task.Run`, `std.thread.spawn`, or other cross-thread entry points.
- Capturing `ref`/`mut` borrows across async `await` points when the referent is not pinned and protected by synchronisation.
- Accessing shared mutable state without holding an appropriate guard from `std.sync`.

These rules give Chic C#-familiar syntax but Rust-class compile-time guarantees: data races cannot compile, and cross-thread aliasing is explicit. The spec expects the type checker and borrow checker to deliver the enforcement, with diagnostics mirroring Rust’s guidance (“type `Foo` does not implement `ThreadSafe`”).

- **Auto-trait overrides:** Types derive `ThreadSafe` and `Shareable` automatically from their fields. Authors can opt in or out explicitly with attributes placed on the declaration:
  - `@thread_safe` forces a type (and any containing types) to be considered `ThreadSafe`.
  - `@not_thread_safe` forbids the trait, even if every field would otherwise satisfy it.
  - `@shareable` / `@not_shareable` provide the same control for the `Shareable` trait.
  These overrides are recorded in the type-layout table and consumed by the type checker and borrow checker. Overrides propagate transitively during auto-derivation.
- **Runtime enforcement hooks:** During lowering the compiler records which locals remain live at every `await`. The type checker emits trait constraints for those locals and parameters; the borrow checker refuses to suspend if an active borrow targets a value that is not `Shareable` (for shared borrows) or `ThreadSafe` (for pinned unique borrows). This mirrors Rust’s `Send`/`Sync` enforcement: an async function whose state machine would move across threads must satisfy these traits.
- **`std.sync` primitives:** Library types in `std.sync` (e.g., `Mutex`, `RwLock`, atomics) reintroduce mutability under synchronisation. They automatically implement both `ThreadSafe` and `Shareable`, so borrowing through a guard satisfies the checker without extra annotations.
- When a value captured across a suspension point or parallel boundary does not satisfy the required auto trait, the compiler emits an error such as `type Foo captured by bar in Demo::Task does not implement ThreadSafe`. Diagnostics include advice to wrap the value in `std.sync::Mutex`, `std.sync::RwLock`, or an atomic primitive so the intent is clear.
- **Native thread entry points:** `Std.Thread::Thread::Spawn` and `ThreadBuilder::Spawn` wire auto-trait constraints directly into MIR. When the payload arc is not `ThreadSafe`, lowering emits `ConstraintKind::RequiresAutoTrait(ThreadSafe, origin=ThreadSpawn)` which the type checker reports as `[MM0102] THREADSAFE_REQUIRED` with guidance to wrap the value or adjust its auto-trait annotations. If the active backend reports that threads are unavailable (WASM today), lowering records `ConstraintKind::ThreadingBackendAvailable`, producing `[MM0101] THREADS_UNAVAILABLE_ON_TARGET` so developers can gate the call or pick a different backend.
- **Shared ownership:** `Std.Sync::Arc<T>` provides thread-safe shared ownership built on the native runtime (`docs/runtime/arc.md` / `docs/runtime/weak_pointers.md`). It mirrors Rust’s semantics: atomic reference counts, downgrade/upgrade with `Weak<T>`, pinning helpers, and `ThreadSafe`/`Shareable` propagation based on the payload type. Diagnostics such as `[MM0102]` surface when non-thread-safe payloads are passed to `Thread::Spawn`.
- **Bootstrap enforcement status:** The bootstrapper encodes these semantics through layout-driven auto traits and async/thread constraint emission. Violations surface with diagnostic codes `TCK035` (trait missing), `TCK037` (trait unproven), `[MM0102]` (thread payload missing `ThreadSafe`), or `[MM0101]` (threads unavailable on the selected backend). The borrow checker guards pinned awaits with explicit `AwaitRequiresThreadSafe` errors when the runtime state machine cannot move safely. For a full walkthrough of the inference pipeline, diagnostics, and regression coverage, see [docs/compiler/thread_safety.md](docs/compiler/thread_safety.md).
- **Result propagation status:** Postfix `?` now lowers to a `match` over the result enum, returning early from the enclosing function and invoking `From::from` when error payloads differ. Operand/return mismatches emit diagnostics, and async lowering honours the early-return path; implementation details live in [docs/compiler/result_propagation.md](docs/compiler/result_propagation.md).

### Threading & Synchronisation Runtime

Chic ships a native thread runtime surfaced through `Std.Thread`:

- `ThreadStart` is the thread entry contract. Instances implement `void Run()` and are wrapped in `Std.Sync.Arc<T>` before crossing the FFI boundary so ownership extends to the new OS thread. `ThreadStartFactory.From` consumes any `ThreadStart` implementation while `ThreadStartFactory.Function(ThreadStartCallback)` adapts Chic delegates (including capturing lambdas).
- `Thread` models the operating system thread handle. `Thread::Spawn(Arc<T>)` clones the payload, passes a typed `ValueMutPtr` (sized/aligned for the arc handle) into the runtime (`ThreadStartDescriptor.Context`), and returns a `Thread` value exposing `Join`, `Detach`, `Sleep`, `Yield`, and `SpinWait`. Dropping a joinable `Thread` automatically detaches so destructors never block indefinitely. The stdlib validates pointer/size/align before dispatching to the runtime and surfaces `ThreadStatus::Invalid` for mismatched handles.
- `ThreadBuilder` is the explicit escape hatch for future runtime knobs (names, affinities, stack sizes). Today it forwards to `Thread::Spawn` but records the intent so CI/tests can enforce that only builder-based code opts into cross-crate inlining or non-default runtimes.
- `ThreadStatus` enumerates runtime results (`Success`, `NotSupported`, `Invalid`, `SpawnFailed`). The standard library throws a `Std::InvalidOperationException` when spawning fails, but call sites can also inspect the status directly to implement retries or platform fallbacks.
- Runtime callbacks (`@export("chic_thread_invoke")`, `@export("chic_thread_drop")`) now consume the full typed `ValueMutPtr` and reject null/misaligned handles before re-entering Chic code. `Arc<T>.IntoRaw()`/`FromRaw()` exchange `ValueMutPtr` handles so size/alignment metadata travels across LLVM/WASM instead of erasing provenance into `usize`; `Arc`/`Rc` raw constructors validate the layout before rehydrating handles.
- WASM backends currently stub the runtime functions: the invoke callback immediately drops the payload, the spawn/join/detach entry points return `ThreadStatus::NotSupported`, and the compiler emits `[MM0101]` so code paths can gate thread usage with `Target::supports_threads()` or `#if target(wasm32)`.
- Auto-trait enforcement happens before codegen: every `Thread::Spawn`/`ThreadBuilder::Spawn` call records both the backend-availability constraint and the `ThreadSafe` requirement. Diagnosing these constraints through `[MM0101]`/`[MM0102]` keeps runtime behaviour predictable and prevents non-thread-safe payloads from compiling on backends that do support threads.

#### 2.6.1 GPU Execution Semantics

- **Special Function Units (SFUs):** When generating PTX we map transcendental functions (`sin`, `cos`, `rcp`, `sqrt`) to SFU instructions when the target architecture provides them. The surface language exposes these as the usual `Math.Sin`, `Math.Cos`, etc.; the code generator selects SFU opcodes for single-precision paths and falls back to polynomial approximations otherwise.
- **Vector & Matrix Operations:** The GPU backend recognises `std.simd` vector types and `std.linalg` matrix abstractions, emitting fused multiply-add and tensor-core friendly instructions (e.g., `mma` PTX ops) where available. MIR carries aggregate shapes so the backend can pack scalar loops into native vector width.
- **Dedicated Hardware Instructions:** Certain APIs (`std.gpu.shadow_map`, future `std.graphics`) compile to GPU-specific instructions such as hardware Z-compare acceleration. These are surfaced through attribute-driven intrinsics (`@gpu_intrinsic("tex.depth_compare")`) to keep portability explicit.
- **Instruction-Level Parallelism:** The optimizer packs scalar MIR operations into vectorised PTX blocks when dependency analysis allows (instruction reordering, predication). We model this by grouping compatible instructions inside a kernel block and emitting combined PTX instructions (e.g., `vadd4`, `mad24`) automatically.
- **Memory & Synchronisation:** Kernels gain access to GPU address spaces through qualifiers (`@shared`, `@local`, `@const`). Ownership rules extend to GPU buffers: moves transfer buffer ownership, `ref` parameters map to device pointers with lifetimes tied to the kernel invocation.
- **AMD ISA Support:** AMD exposes separate instruction streams for scalar (`s_` prefix), vector (`v_` prefix), memory (`flat_/buffer_`), control-flow (`s_cbranch_*`, `s_setpc_b64`), and export (`exp`) instructions. Chic maps MIR operations into these categories:
  - Scalar instructions handle kernel-level control flow, uniform math, and state setup.
  - Vector instructions execute per-lane math; `std.simd` operations lower directly to RDNA/CDNA vector ALU ops.
  - Memory instructions remain explicit (GCN/CDNA “load-store” model). Our MIR builder records address space so the backend can emit the appropriate `global`, `shared`, or `scratch` reads/writes.
  - Control flow uses wave-friendly constructs; RDNA 4 instruction prefetch hints are represented via MIR metadata for future optimization.
  - Export instructions surface through `std.graphics` pipelines, writing color/depth outputs via `exp` packets.
- **Documentation & Tooling:** We follow GPUOpen references (ISA PDFs, machine-readable XML) so Chic’s backend can decode/encode instruction packets. Developers can inspect generated ISA via integrations with tools like Radeon GPU Analyzer (RGA) from the `chic mir-dump` and future `chic gpu-isa` commands.
- **Intel Xe ISA Support:** Intel publishes Xe ISA documentation (via GPUOpen-equivalent whitepapers and oneAPI specs) outlining EU instructions: EU scalar (`mov`, `add`, control), SIMD vector math, message send instructions for memory/samplers, and datapath matrix units (DPAS). Our backend annotates MIR with subgroup/wave semantics so we can emit the right VISA message descriptors, choose between SLM (shared local memory) and global memory, and leverage hardware features such as thread-level prefetch and barrier instructions (`barrier`, `fence`). Offline inspection is supported through Intel’s IGC/Shader Analyzer integration.

GPU targets remain optional; hosts without compatible devices fall back to CPU execution paths. Future profiles will document Metal/Vulkan/SPIR-V lowering once those backends are implemented.

#### 2.6.2 CPU Instruction Strategy

Chic generates native code for modern 64-bit CPUs from both Intel and AMD. While we rely on LLVM to handle final instruction selection (with the WASM backend providing the fast-iteration path), the spec captures which architectural features we expect to leverage:

- **Intel CPUs:** Baseline assumes SSE4.2 and BMI1/BMI2. Optimised builds enable AVX2 for wide vector math, FMA3 for fused multiply-add, and AVX-512/AMX where available (gated by target feature flags in `manifest.yaml`). We also plan to exploit Intel’s DL Boost (VNNI) for integer matrix workloads by lowering `std.linalg` ops accordingly.

  The bootstrap LLVM backend now multi-versions Chic functions when targeting x86_64 with more than one tier enabled. Each compiled function produces specialised variants (`__baseline`, `__avx2`, `__avx512`, `__amx`) and a dispatcher that selects the best candidate at runtime using `__cpu_indicator_init`/`__cpu_model` feature probes. `std.simd::f32x8::fma` lowers to fused multiply-add instructions on AVX2+ tiers (falling back to scalarised SSE code in the baseline variant), while `std.linalg::int8x64::dpbusd` emits AVX-512 VNNI instructions with a baseline trap to signal unsupported hardware. The same tier set can be configured per target through `--cpu-isa` or the `toolchain.cpu-isa` manifest entry.

- **Apple Silicon CPUs:** Every tiered build starts from a NEON + FP32 baseline. Additional feature buckets (“dotprod”, “fp16fml”, “bf16”, “i8mm”, “crypto”, “pauth”, “bti”, and “sme”) are exposed through `--cpu-isa`/`toolchain.cpu-isa`, generating clones such as `__dotprod` and `__i8mm`. The dispatcher relies on Darwin’s `sysctlbyname` interface (`hw.optional.arm.*`/`hw.optional.arm.caps`) to pick the best implementation at runtime. The LLVM backend lowers `std.simd::f32x4::fma` onto NEON FMLA, maps `std.linalg::int8x64::dpbusd` to Arm UDOT/I8MM code paths, and emits BF16 matrix multiplies (`bfmmla`) with optional SME streaming mode guards for M4-class processors.
- **AMD CPUs:** Zen-class processors supply SSE4.2, AVX2, FMA3, BMI1/BMI2, and (on Zen 4+) AVX-512 subsets. We surface these through the same feature toggles, ensuring codegen picks the right instruction mix (e.g., `vperm`, `vgather`, `vp2intersect`). Future work will expose AMD’s FP16 and BF16 acceleration once widely available.
- **Portability:** If a requested instruction set is not present at runtime, the runtime loader falls back to a compatible code path (fat binaries or JIT-selected functions). The compiler emits multi-versioned functions when the build profile requests both baseline and advanced ISA variants.
- **Linux AArch64 coverage:** Ampere Altra/One and NVIDIA Grace presets now enable multi-versioned functions for NEON dot-product/FP16FML, BF16/I8MM, and (when present) SVE/SVE2. The generated IR probes `getauxval(AT_HWCAP/HWCAP2)` to select the right tier at runtime, and `chic build` offers `--sve-bits <128|256|…>` to pin the effective SVE vector length for reproducible builds.
- **SVE lowering:** When the selected tier includes SVE2, `std.linalg.int8x64.dpbusd` lowers to `llvm.aarch64.sve.usmmla.nxv4i32` while BF16 matrix multiplies use `llvm.aarch64.sve.bfmmla.nxv4f32`. NEON-based fallbacks remain in place for dot-product, BF16, and SME paths so Apple-centric builds continue to rely on the existing instructions.

### 2.7 Arithmetic Operators

| Operator | Name        | Description                               | Example |
|----------|-------------|-------------------------------------------|---------|
| `+`      | Addition    | Adds two operands                         | `x + y` |
| `-`      | Subtraction | Subtracts the right operand from the left | `x - y` |
| `*`      | Multiplication | Multiplies operands                    | `x * y` |
| `/`      | Division    | Divides the left operand by the right     | `x / y` |
| `%`      | Modulus     | Remainder of integer division             | `x % y` |
| `++`     | Increment   | Increases a numeric variable by 1         | `x++`   |
| `--`     | Decrement   | Decreases a numeric variable by 1         | `x--`   |

Increment and decrement may be used in prefix or postfix position; semantics mirror C#/C-style languages and interact with borrow rules through the usual read/write checks (`++`/`--` require a unique `ref` to their operand).

### 2.8 Comments

- **Single-line:** Start with `//` and extend to end-of-line. The compiler ignores the remainder of the line.
- **Multi-line:** Start with `/*` and end with `*/`; contents are ignored. Nested `/* */` blocks are permitted and the lexer tracks depth so the outer comment is not terminated until its matching `*/`.
- **Documentation (`///`):** Three leading slashes begin an XML documentation comment. The comment continues until the end of the line, and multiple consecutive `///` lines are concatenated. The content must be well-formed XML and is attached to the next declaration (type, method, field, etc.). The bootstrap compiler now preserves these comments in the AST/HIR, surfaces them in the textual IR, and emits them in the metadata sidecar so downstream tooling can display or analyse the documentation without re-parsing source.

```chic
// Single-line comment before code
let answer = 42;

/*
 Multi-line comment explaining the computation.
 Nothing inside this block is compiled.
*/
var counter = 0;

/* Nested block comments are permitted.
   /* inner block */ still inside outer */

/// <summary>Computes the hypotenuse.</summary>
/// <param name="x">Opposite side length.</param>
/// <param name="y">Adjacent side length.</param>
public double Hypot(double x, double y) => sqrt(x * x + y * y);
```

### 2.9 Assignment Operators

| Operator | Example   | Equivalent        |
|----------|-----------|-------------------|
| `=`      | `x = 5`   | `x = 5`           |
| `+=`     | `x += 3`  | `x = x + 3`       |
| `-=`     | `x -= 3`  | `x = x - 3`       |
| `*=`     | `x *= 3`  | `x = x * 3`       |
| `/=`     | `x /= 3`  | `x = x / 3`       |
| `%=`     | `x %= 3`  | `x = x % 3`       |
| `&=`     | `x &= 3`  | `x = x & 3`       |
| `|=`     | `x |= 3`  | `x = x | 3`       |
| `^=`     | `x ^= 3`  | `x = x ^ 3`       |
| `<<=`    | `x <<= 3` | `x = x << 3`      |
| `>>=`    | `x >>= 3` | `x = x >> 3`      |

Compound assignments perform the indicated operation followed by an assignment. The left operand must be writable (a `var` binding or `ref` borrow); right-hand operands follow normal move/borrow rules.

### 2.10 Comparison Operators

| Operator | Name                     | Example  |
|----------|--------------------------|----------|
| `==`     | Equal to                 | `x == y` |
| `!=`     | Not equal                | `x != y` |
| `>`      | Greater than             | `x > y`  |
| `<`      | Less than                | `x < y`  |
| `>=`     | Greater than or equal to | `x >= y` |
| `<=`     | Less than or equal to    | `x <= y` |

Comparison operators yield `bool`. Equality/inequality are defined for primitives and any type providing `@derive(Equatable)` or user overloads; ordering comparisons apply to numeric types and others that implement the relevant traits.

### 2.11 Logical Operators

| Operator | Name        | Description                                        | Example                  |
|----------|-------------|----------------------------------------------------|--------------------------|
| `&&`     | Logical and | Returns `true` when both operands evaluate to `true` | `x < 5 && x < 10`        |
| `||`     | Logical or  | Returns `true` when either operand is `true`       | `x < 5 || x < 4`         |
| `!`      | Logical not | Inverts a boolean value                            | `!(x < 5 && x < 10)`     |

`&&` and `||` short-circuit: if the left operand determines the result, the right-hand expression is not evaluated. All logical operators operate on `bool` (or future types that implement the appropriate traits).

### 2.12 Bitwise & Shift Operators

| Operator | Name                   | Description                                        | Example        |
|----------|------------------------|----------------------------------------------------|----------------|
| `&`      | Bitwise AND            | Sets each bit to 1 if both bits are 1              | `flags & mask` |
| `|`      | Bitwise OR             | Sets each bit to 1 if either bit is 1              | `flags | mask` |
| `^`      | Bitwise XOR            | Sets each bit to 1 if bits differ                  | `flags ^ mask` |
| `~`      | Bitwise NOT            | Inverts every bit                                   | `~flags`       |
| `<<`     | Left shift             | Shifts bits left, filling with zero                | `value << 2`   |
| `>>`     | Right shift            | Shifts bits right, sign-extending for signed types | `value >> 1`   |

Shift operators operate on integer types. Right shifts follow the underlying signedness (`int`/`long` are arithmetic shifts; unsigned types use logical shifts). Bitwise operators integrate with assignment forms (e.g., `&=`) described earlier.

### 2.13 Null-Coalescing Operators

| Operator | Name                     | Description                                                | Example                     |
|----------|--------------------------|------------------------------------------------------------|-----------------------------|
| `??`     | Null-coalescing          | Returns left operand if non-null, otherwise right operand  | `value ?? fallback`         |
| `??=`    | Null-coalescing assignment | Assigns right operand only when left is `null`            | `name ??= \"<anonymous>\"`   |

These operators operate on `T?` (sugar for `Option<T>`) and references that permit `null`. `??` short-circuits: the right-hand expression is evaluated only when the left operand is `null`. The assignment form is equivalent to `if (x == null) { x = rhs; }`.

`??` binds more weakly than `||` and associates right-to-left so chains such as `a ?? b ?? c` evaluate as `a ?? (b ?? c)`. The left operand is evaluated exactly once, cached in a temporary, and then branched on the nullable sentinel (`HasValue`). Drops on the left operand follow the selected branch so destructors for disposable resources still run before the join point.

`??=` performs the same test before writing: Chic emits a branch that only copies the fallback into the nullable payload and flips `HasValue` when the original value is null. Property setters honour this shape too—the setter is invoked solely on the null branch—so accessor side effects are never duplicated.

### 2.13.1 Null-Conditional Assignment

Null-conditional member/indexing (`?.`, `?[ ]`) can appear on the left-hand side of assignment and compound-assignment operators (everything except `++`/`--`). The receiver is evaluated exactly once, cached in a temporary, and a null-check gates the remainder of the operation. If the receiver is `null`, the statement is a no-op and the right-hand side (or compound operand) is not evaluated. Examples:

```chic
customer?.Order = GetCurrentOrder();
customer?.Total += delta;
items?[idx] = value;
items?[idx] *= factor;
customer?.Orders?[0]?.Total += delta;
```

Nested chains short-circuit at the first `null` receiver. Index arguments participate in the short-circuit and are evaluated only once when the receiver is non-null. Receivers must be nullable (`T?`/nullable references); applying `?.`/`?[]` to non-nullable values emits a diagnostic. Null-conditional assignment is a statement-only form—its value is discarded.

`++` and `--` are rejected when applied through a null-conditional access; use an explicit null-check and assignment instead.

### 2.14 `nameof` Operator

`nameof(expr)` evaluates at compile time to the simple identifier name of the symbol referenced by `expr`. Examples:

```chic
let field = nameof(Point.X);      // "X"
let method = nameof(Math.Hypot);  // "Hypot"
let parameter = nameof(value);    // "value"
```

The argument is analysed but not executed; `nameof` always produces a `string` literal and is valid in constant expressions and attributes.

The operand must resolve to a symbol (local bindings, types, members, enum variants, or functions). Complex expressions such as `nameof(value + 1)` or ambiguous overload groups emit diagnostics at compile time. Generic arguments are analysed but ignored when reporting the simple identifier; `nameof(Vector<int>.Length)` still evaluates to `"Length"`.

### 2.15 `sizeof` and `alignof`

`sizeof` evaluates entirely at compile time and returns a `usize` describing the byte width of its operand. Parenthesised forms accept explicit type operands (`sizeof(Point)`), while bare operands (`sizeof value`) resolve the static type of a local, parameter, or pattern binding without executing the expression.

`alignof` mirrors the syntax and produces the required byte alignment for the operand’s type. Both operators report diagnostics when the layout cannot be determined (for example, unconstrained generic parameters or unresolved identifiers). Sequence types such as `Array<T>` and `Vec<T>` continue to report their pointer-sized metadata so the managed representation stays predictable. No MIR instructions are emitted—the operators lower to constants.

```chic
var primitive = sizeof(int);         // 4 bytes
var record = sizeof(Point);          // uses recorded layout metadata
var pointer = sizeof(Array<int>);    // pointer width (8 bytes on the bootstrap toolchain)
var alias   = sizeof buffer;         // `buffer` is a parameter; resolved at compile time

var primAlign = alignof(int);        // 4 bytes
var recordAlign = alignof(Point);    // obeys struct layout attributes
var spanAlign = alignof Span<byte>;  // pointer-sized on the bootstrap targets
var aliasAlign = alignof buffer;     // matches the static type of `buffer`
```

### 2.16 `import` Directives

Files may begin with `import` directives to bring namespaces into scope, bind aliases, or expose
static helpers:

```chic
import Std.Text;
import Collections = Std.Collections.Generic;
import static Std.Math;
```

`import` directives are processed before namespace declarations. The bootstrap parser recognises
alias and `import static` forms, and leading XML documentation comments (`///`) are preserved on the
directive so metadata and emitted artifacts retain author intent.

Any directive may be prefixed with `global` to make it visible to every namespace in the compilation
unit:

```chic
global import Std.Diagnostics.Tracing;
global import Ser = Std.Serialization;
global import static Std.Math;
```

Global directives must appear at the very top of the file before any namespace or type declarations
and cannot be nested inside namespaces or types. Misplaced directives are rejected with targeted
diagnostics. All `global import` directives across the compilation are collected once and applied to
every source file, so a project-level `global_imports.cl` can centralise imports such as `Std` and
`Std.Numeric`.

Resolution order becomes: all global directives, then the root namespace, then each enclosing
namespace from outermost to innermost. Alias bindings follow this precedence, but conflicting alias
targets between global directives—or between a global alias and a local alias—produce an error. When
multiple namespaces or aliases make a type name ambiguous, the compiler reports the competing
candidates and requires a fully qualified name. The `global` keyword is rejected unless it
immediately precedes `import`.

`using` directives are not supported; use `import`. Resource-management `using` statements described in §2.18 are unchanged.

### 2.17 Variable Declarations

Local variables inside blocks are declared with `let` (immutable) or `var` (mutable). Typed locals of the form `Type name = expr;` without a leading keyword are not valid Chic syntax and produce a compiler error.

```chic
let x = 5;
let y = 6;
let z = 50;
let message = "hi";
var count = 3; // when mutation is required
```

Each declarator is lowered individually; all bindings share the declared type, and definite-assignment rules apply to each variable. The rule applies uniformly to `for`/`foreach` headers, pattern bindings (`case let/var`, `if (expr is let …)`), `using`/`fixed` declarations, and any other block-local declaration site—typed forms trigger `LCL0001` with a help message suggesting `let`/`var`.

`let` bindings are immutable. Any attempted mutation—simple/compound assignment, `++`/`--`, or passing as `ref`/`out`—is rejected with `LCL0002` and a concrete suggestion to change the declaration to `var`.

Identifiers follow Unicode 17.0.0 UAX #31 (ID_Start/ID_Continue plus `_`, excluding Pattern_Syntax/Pattern_White_Space) and are normalised to NFC during lexing. Extended pictographic code points (emoji) are accepted as identifier starts/continues when not paired with forbidden controls. Symbol resolution and interning always use the NFC spelling, while diagnostics retain source spans; non-NFC spellings or disallowed code points (Bidi controls, join controls, variation selectors, or other invisible defaults) trigger errors with a suggested normalised replacement. This keeps native-script identifiers (`let число = 1;`, `var 数据 = 2;`, `public int 合計(int 左, int 右)`) legal without admitting invisible control characters into the symbol table.

**Naming constants and variables.** Unicode identifiers are first-class; the lexer folds to NFC but preserves spans for diagnostics. Examples:

```chic
let π = 3.14159;
let 你好 = "你好世界";
let 🐶🐮 = "dogcow";
```

Emoji identifiers are permitted when the code point is Extended_Pictographic and not combined with forbidden controls (Bidi controls, ZWJ/ZWNJ, variation selectors). Non-NFC spellings or invisible code points emit a targeted diagnostic that shows the normalised replacement.

#### Pinned storage

Locals that must never move can be marked with `@pin` ahead of the declaration:

```chic
@pin var buffer = Rings.Allocate();
```

The attribute is accepted in stand-alone declarations as well as inside `for`, `using`, and `fixed` headers. Once pinned, a binding cannot be moved (`move pinned` or assigning it into another slot by move triggers a compile-time error), and the borrow checker allows unique borrows of the pinned storage to span `await` suspension points. Explicit annotations of the form `Pin<T>` are treated equivalently—any variable or parameter whose declared type is `Pin<...>` is automatically marked as pinned during lowering. Async lowering records the set of pinned locals alongside suspend-point metadata so the runtime can keep those slots stationary when the state machine is resumed.

#### Thread-safety overrides

Type declarations accept lightweight attributes to override the compiler’s auto-derived concurrency traits:

- `@thread_safe` / `@not_thread_safe`
- `@shareable` / `@not_shareable`

Annotating a struct, class, enum, or union with these attributes forces (or denies) the corresponding trait regardless of the member types. Overrides participate transitively—if `Buffer` is marked `@not_shareable`, then `@thread_safe` on a wrapping `Packet` struct is rejected unless the developer also supplies an explicit `@shareable` override. These annotations feed directly into the MIR type-layout table so both the type checker and borrow checker can reason about trait availability at async/thread boundaries.

### 2.18 Statements

Chic adopts the C# statement catalogue [[reference](https://learn.microsoft.com/en-us/dotnet/csharp/programming-guide/statements-expressions-operators/statements)] and maps each form to the language’s ownership-aware semantics. The parser recognises:

- **Declaration statements:** `let`, `var`, and `const` with optional explicit type annotations. Block-scoped declarations **must** start with `let` or `var`; typed locals of the form `Type name = expr;` are rejected with `LCL0001`. These lower to `VariableDeclaration` nodes attached to the surrounding block.
- **Expression statements:** any expression followed by `;`, including assignment and invocation forms.
- **Selection statements:** `if`/`else` with nested statements and `switch` with `case`/`default` sections plus optional `when` guards.
- **Iteration statements:** `while`, `do-while`, `for` (declaration or expression initialisers), and `foreach` with Unicode bindings.
- **Jump statements:** `break`, `continue`, `return`, `throw`, `goto label`, `goto case expr`, `goto default`, `yield return`, and `yield break`.
- **Exception handling statements:** `try` with any combination of `catch` (including typed and filtered `when` clauses) and `finally`.
- **Resource management statements:** `using (resource) statement` and `using var name = expr;` map to `UsingStatement` nodes tracking either an expression or declaration resource.
-   Lowering rewrites `using` forms into MIR sequences that emit `StorageLive`, record a deferred
    drop in reverse declaration order, and insert `StorageDead` on the fallthrough path. The later
    drop pass expands those records into explicit `Deinit`/`Drop` statements so resources clean up
    on every exit.
- **Concurrency and arithmetic control:** `lock (expr) statement`, `checked { ... }`, and `unchecked { ... }`.
- **Unsafe operations:** `fixed (let|var name = expr) statement` and `unsafe statement` capture pointer pinning and unsafe blocks for later borrow checking.
-   Unsafe blocks emit MIR markers (`EnterUnsafe`/`ExitUnsafe`) so the borrow checker can distinguish code that runs with relaxed rules for raw pointers. These markers bound the lifetime of unsafe regions even when control flow exits early.
- **`unsafe` functions:** Adding the `unsafe` modifier executes the body as though it were wrapped in an `unsafe { … }` block. Callers must still provide an unsafe context; invoking an unsafe function from safe code produces a diagnostic at the call site.
- **Raw pointer casts:** Converting between raw pointers and integer types requires an unsafe region. MIR lowering rejects pointer-to-integer and integer-to-pointer conversions outside `unsafe` blocks or functions, matching the rules for address-of and dereference operations.
- **Pointer arithmetic:** In `unsafe` contexts, raw pointers permit `ptr + isize`, `ptr - isize`, and `ptr - ptr`. The first two yield adjusted pointers using byte offsets (no implicit element scaling); the last yields an `isize` byte distance. Backends lower to GEPs or integer offset math without pointer-to-integer round-trips when possible, preserving `@expose_address` provenance.
- **Labeled statements:** `identifier: statement` integrate with `goto` for control transfers inside the same function.

#### Inline assembly (`asm!`)

- Requires an `unsafe` block or `unsafe` function; safe contexts emit a targeted diagnostic.
- Syntax mirrors Rust: `asm!(template, operands..., options(...)? , clobber(...)?);` where templates are raw strings with `{name}`/`{0}` placeholders (`{{`/`}}` escape braces).
- Operand forms: `in(reg) expr`, `out(reg) place`/`lateout`, `inout(reg) expr [=> place]?`/`inlateout`, `const expr` (must be compile-time constant), and `sym ident`.
- Register classes: `reg`, `reg8/16/32/64`, `xmm/ymm/zmm`, `vreg`, `kreg`, plus explicit registers (`"{rax}"`, `"x0"`, `"xmm0"`). LLVM supports x86_64/aarch64; WASM and other targets reject inline assembly with a dedicated diagnostic.
- Options: `volatile`, `alignstack`, `intel`/`att_syntax`, `nomem`, `nostack`, `preserves_flags`, `pure`, `readonly`, `noreturn`. Clobbers are provided via `clobber(regs...)`; default clobbers include memory and flags unless `nomem`/`readonly`/`pure`/`preserves_flags` opt out.
- Lowering records a dedicated MIR `InlineAsm` node (template pieces, operands, clobbers, options) that maps directly to LLVM inline asm or a backend rejection; see `docs/codegen/inline_asm.md` for the full contract.

All statement forms preserve span information so diagnostics can anchor messages precisely. The AST exposes these variants via `StatementKind`, with nested statements expressed as `Box<Statement>` or `Block` nodes to mirror C#’s embedded statement rules.

`lock (expr) statement` evaluates `expr` exactly once into a temporary, invokes `Enter()` on that value (or a compatible lockable target such as `Std.Sync.Lock`), and binds the resulting guard to an implicit local. MIR lowering surrounds the body with a `try`/`finally` edge so the guard’s `dispose` runs on every exit path—normal fall-through, `break`/`continue`, `return`, and `throw` alike. Awaiting inside a locked region is rejected so critical sections cannot be suspended mid-execution.

`fixed (let|var name = expr) statement` pins the target expression for the lifetime of the block. Because locals must use `let`/`var`, pointer-typed bindings rely on inference or explicit casts in the initializer (`fixed (let ptr = (byte*)buffer)`), never a leading type token. Each declarator introduces a unique borrow recorded in a hidden guard local (marked as pinned) and initialises the pointer binding with an address-of rvalue. When the statement exits—via fall-through, `break`, `continue`, or `return`—the guard is dropped and the place becomes movable again.

Any function containing `yield return` or `yield break` is treated as a generator. Each `yield return` produces a MIR `Yield` terminator and a corresponding resume/drop pair of blocks recorded in `MirBody.generator`, enabling the runtime and backends to emit iterator state machines. `yield break` lowers to a plain `Return` yet still marks the function as a generator so the iterator machinery knows when to complete the sequence.

#### Const declarations

`const` bindings are resolved entirely at compile time and disappear from the generated MIR:

- Declaration syntax mirrors C#: `const T Name = initializer;` with optional comma-separated declarators that share the annotated type. `const` is valid at namespace scope, inside types, and as a block-level statement.
- Initialisers must reduce to compile-time values. The evaluator accepts primitive literals, string literals, the results of `nameof` and `sizeof` (type operands only), other constants (subject to visibility), and calls to pure Chic functions when every argument is itself constant.
- Compile-time function execution is available to any synchronous, non-generic Chic function (including extension methods) whose body is composed only of block scopes, `const`/`let`/`var` declarations with initialisers, expression statements, assignments, and `if`/`else` conditionals that ultimately return a value. Borrowed parameters (`ref`/`out`), extern bodies, loops, generators, `await`, `throw`, `goto`, `unsafe`, and resource statements are rejected with targeted diagnostics pointing at the unsupported construct.
- When the evaluator encounters a side-effectful statement, observes runtime-only state, or detects recursion, it aborts the compile-time evaluation with a diagnostic. In contexts where a value is required to be constant the compilation fails; in best-effort folders (e.g., optimisation passes) the evaluator simply leaves the expression for runtime execution so both WASM and LLVM backends remain behaviourally identical.
- Block-scoped `const` statements act as aliases; evaluation occurs as the block is lowered and subsequent uses of the name are rewritten to the computed literal. No local storage is allocated in MIR.
- String constants are interned with static lifetime so that backends can reuse the storage across translation units. Other literal kinds flow directly into the MIR via `Operand::Const`.

#### Foreach iteration

`foreach` lowers into a deterministic state machine that evaluates the sequence expression once, acquires an enumerator, and drives it with `MoveNext`/`Current` calls:

- The binding syntax accepts Unicode identifiers and the modifiers `in`, `ref`, or `ref readonly` ahead of `let`/`var`. Foreach bindings must use `let`/`var`; typed forms without a keyword are rejected. `let` bindings remain immutable inside the loop body; `ref` produces a unique borrow so the element can be mutated in place, `ref readonly` and `in` produce read-only borrows, and the plain form copies the element.
- Each iteration begins by creating a fresh scope for the iteration variable (`StorageLive`), binding it to the enumerator’s current element, running the body, and then ensuring `StorageDead` executes before any `continue` or subsequent `MoveNext`.
- A dedicated cleanup block sits behind `continue` statements so borrows are released before the next call to `MoveNext`. A separate break block runs the same cleanup before transferring control to the loop exit.
- Lowering synthesises hidden locals (`__foreach_seq_n`, `__foreach_enum_n`). The sequence expression is evaluated exactly once into the sequence slot, `GetEnumerator()` is invoked on it, and the enumerator local is guarded by a deferred drop so `dispose` runs on every exit path.

When the sequence resolves to one of the intrinsic containers (`Vec`, `Array`, `Span`, `string`, `str`), the compiler bypasses `GetEnumerator()` entirely. Instead it snapshots the container metadata into stack locals (`ptr`, `len`, `idx`), emits the `idx < len` guard inline, and assigns each element by indexing directly into the buffer. The iteration variable still receives a `StorageLive`/`StorageDead` pair per trip through the loop and a deferred drop is scheduled for it, so user-authored `dispose` bodies and auto-generated drop glue execute before every `StorageDead`, matching the semantics of the enumerator path.
- `MoveNext()` is emitted in the condition block and feeds a `SwitchInt` terminator, while `Current` is read in the prepare block: value bindings copy the result, and `in`/`ref readonly`/`ref` bindings materialise shared or unique borrows in MIR so the borrow checker can enforce aliasing guarantees.

A schematic expansion is:

```chic
let enumerator = values.GetEnumerator();
defer enumerator.Dispose();

while (enumerator.MoveNext()) {
    let current = enumerator.Current;
    body(current);
}
```

The MIR builder emits the same structure with explicit blocks for the condition, body, per-iteration cleanup, break cleanup, and the final exit so borrow checking and code generation can reason about the transitions precisely.

#### Pattern matching

Chic mirrors the full C# pattern-matching catalogue [[reference](https://learn.microsoft.com/en-us/dotnet/csharp/fundamentals/functional/pattern-matching)] across `switch` statements, `if (expr is pattern)` guards, and future switch-expression sugar. Supported pattern families include:

- **Declaration & type patterns:** `case Foo bar:` binds concrete types; `is Foo bar` works in conditionals. Nullable value patterns (`is int? x`) desugar into `Option<T>` checks.
- **Relational and logical patterns:** `case > 0`, `case <= 10`, `case > 0 and <= 10`, `case not null`, `case >= 0 and not 42`. Relational operators follow the target type’s comparison semantics.
- **Property patterns:** `case Order { Status: OrderStatus.Pending, Customer: { Tier: Tier.Gold } }` drills into nested members using Chic’s dot-access rules.
- **Positional patterns:** tuple-like deconstruction `(var x, var y)` leverages user-defined `Deconstruct` methods or struct field order for records.
- **List patterns:** `case [first, .. middle, last]` matches arrays, spans, and readonly spans once collection interfaces expose the required `Length`/indexing members. Slice bindings (`..tail`) materialise `ReadOnlySpan<T>` views with pointers advanced to the slice start so `tail[0]` and `tail.Length` reflect the remaining segment.
- **Record/anonymous patterns:** `case Shape.Circle { Radius: 10 }` or `{ X: 1, Y: let y }` match typed records and anonymous field sets in `switch`, `is`, `if`, and `while` contexts while preserving field spans for tooling.
- **Var/discard patterns:** `case var value` or `_` provide catch-all behaviour while capturing values when needed.

List patterns now recognise both prefix and suffix elements alongside the `..` slice form. The compiler synthesises guard expressions for every structural requirement (length checks, literal comparisons, suffix offsets) before any user-authored `when` expressions run. Each `when` clause is evaluated strictly left-to-right: the generated list guards fire first, followed by the guards that appeared in source order. Guard failures immediately transfer control to the next arm so side effects inside later guards never execute once an earlier predicate fails. The same evaluation order applies to `is pattern when ...` expressions, ensuring borrow scopes for pattern bindings remain contained while still allowing nested guard logic and backtracking across multiple guards. Record and list patterns reject duplicate fields or multiple slice bindings during type checking so every backend observes the same borrow scopes and guard ordering.

Pattern evaluation order matches C#: positional/property patterns run their sub-patterns left to right, and `when` guards execute after the pattern binds but before the arm body. Pattern-bound locals default to `let`/value semantics; developers opt into moves with the `move` modifier. Positional patterns flow into the underlying struct/enum layout (the compiler maps tuple elements onto field order), and the initial list-pattern implementation supports prefix literals/wildcards (`[1, 2]`) while richer slice suffix forms are tracked via issues.

`is` pattern expressions lower to the same `Match` machinery used by `switch`, returning a Boolean result without introducing additional scope bindings yet. This keeps flow-control parity with C#/Rust style conditionals while we finish binding lifetimes inside expressions.

MIR lowering emits either the lightweight `SwitchInt` terminator (for pure relational/literal sets) or a rich `Match` terminator carrying structured `Pattern` trees and guard metadata. Guard expressions lower to dedicated blocks so side effects and diagnostics remain well scoped.

#### Try / Catch / Finally

`try` statements produce an exception region in MIR so the compiler can reason about both normal and exceptional control flow:

- The builder synthesises two locals per region: `__exceptionN` (the active exception object) and, when a `finally` clause exists, `__pending_exceptionN` (a boolean that tracks whether the region is unwinding). Both locals receive explicit `StorageLive`/`StorageDead`.
- Each `catch` clause expands into an entry block (binding the identifier and evaluating the optional `when` expression), a body block, and a cleanup block that clears the pending flag before jumping to the `finally` clause or to the post-`try` continuation.
- The catch dispatch block currently issues a simple `Goto` into the first catch entry; later passes enrich it with type tests once the runtime representation of exceptions is finalised.
- Unhandled exceptions flow into a dedicated block whose terminator is `Throw { exception }`. The operand captures the active exception value so backends can wire the runtime unwinder consistently.
- `finally` clauses lower into paired entry/exit blocks. The exit block emits a `SwitchInt` on the pending flag so a `throw` before or inside the catch body re-enters the catch dispatcher after the cleanup runs; when no handler matches, control transfers to the `Throw` terminator described above.

As with `foreach`, no `Pending` statements remain in the lowered blocks. The explicit `Throw` terminator makes the exceptional edge visible to borrow checking, code generation, and the runtime layer even while the high-level unwinder continues to evolve.

#### Throw expressions

Chic now treats `throw` as both a statement and an expression. `throw expr` evaluates `expr`, marks the current block as terminating, and yields the never type (`!`) so callers cannot rely on a value flowing from that branch. This enables idioms such as `return throw BuildError();` and future expression-level sugar (conditional expressions, null-coalescing patterns, etc.).

- `throw;` without an operand is only valid inside a catch/finally stack that already set the pending exception binding; attempting to rethrow outside of that context produces a lowering diagnostic.
- When a `throw` escapes every handler, the lowering inserts a `Throw { exception }` terminator. Both LLVM and WASM backends now translate this into a call to `chic_rt_throw`, passing the exception payload (zero when rethrowing an ambient error) and a stable 64-bit type identity derived from the error’s canonical name. The bootstrap runtime records the invocation and terminates deterministically today; once the unwinder lands the same hook will forward the typed payload into the unwinding surface.
- `throw` expressions participate in the normal flow analysis: any statements that follow execute in a new block, and borrow checking treat the terminator like `return`.

Nullable exception payloads (`Exception?`, `TransientError?`, and so on) cannot flow directly into a `throw` operand or `catch` binding. Lowering reports a diagnostic when it detects a possible `null` propagation so code must convert the value into a non-null exception before throwing (for example, via an explicit guard or factory) and bind catch variables to non-nullable exception types.

#### Exception Type Hierarchy

Chic provides first-class support for specialised exception types via the `error` declaration form. Error declarations use the same member syntax as classes but automatically inherit from the language-defined `Exception` base type unless an explicit exception base is provided.

- `error TransientError : Exception { ... }` defines a concrete exception that inherits from `Exception`. Omitting the base clause is equivalent to `: Exception`.
- Error declarations may only inherit from other exception types. Attempting to derive from an arbitrary class or interface produces a lowering diagnostic.
- `throw` expressions must evaluate to an exception instance. The lowering step reports an error when a non-exception value is thrown or when the operand type cannot be resolved inside the exception hierarchy.
- Catch clauses may specify a concrete exception type (`catch (TransientError err)`); the annotation must resolve to `Exception` or a derived type. Omitting the annotation behaves as `catch (Exception err)`.
- MIR metadata records the concrete exception payload on `Throw` terminators and catch regions so later phases (borrow checking, code generation, and, eventually, unwinding) can reason about the precise type.

#### Mandatory Exception Handling

Chic treats fallible APIs as effects that must be acknowledged explicitly. The compiler enforces two layers of diagnostics so ignored exceptions/results never slip through a build:

1. **Drop warnings.** Expressions whose static type is `Exception`, `Exception?`, `Result<T>`, or `Result<T, E>` trigger a warning when their value is discarded (for example by calling a method and ignoring the return, or evaluating a `Result` in statement position). The warning is suppressed when the value is matched, passed into a `try`/`catch` block, stored for later inspection, or propagated via postfix `?`.
2. **Unhandled-path errors.** Control-flow analysis escalates the warning to a hard error whenever a function can reach a boundary (implicit return, end of scope, or `finally` exit) while a fallible value remains unhandled. This covers patterns such as swallowing a `Result` in a branch or returning from a `try` without consuming the active exception.

To silence the diagnostics, code must do one of the following:

- Consume the value via `?`, `await`, pattern matching, or explicit `try`/`catch`.
- Forward the fallible payload to another helper that records/propagates it (`return result;`, `throw exception;`).
- Explicitly acknowledge the intentional drop using `let _ = expr;` or a future `Result::ignore()` helper; this suppresses the drop warning but still participates in unhandled-path analysis.

These rules ensure Chic aligns with its “no silent failure” goal: fallible APIs either surface a diagnostic or are handled/propagated in a way the compiler can reason about.

#### Labels and goto

Labels introduce control-flow anchors without opening new lexical scopes. When lowering `goto` the compiler emits explicit `StorageDead` statements for every local whose scope is exited by the jump so that deterministic destruction and borrow scopes remain sound. Forward references are handled by recording the outgoing block and patching it once the label is encountered, which guarantees that the final MIR no longer contains pending placeholders. Jumping into a nested scope or reusing a label name produces diagnostics during lowering, and references to missing labels surface as errors while the MIR stays well-formed for subsequent passes.

### 2.19 Async & Await

- Functions, methods, and extensions may be marked `async`. Async bodies must return `Task` or
  `Task<T>` (from `Std.Async`) and lower to state machines that record suspension points, resume and
  drop blocks, captured locals, and pinned storage in an `AsyncStateMachine` descriptor.
- `await expr` is valid only inside async functions/testcases. It yields to the runtime, revalidates
  borrows at the suspension point, and requires pinned storage (`@pin`/`Pin<T>`) for values that
  must stay in place. MIR lowering introduces a hidden `__async_ctx` local that carries a pointer to
  `Std.Async.RuntimeContext` for runtime calls.
- Trait methods may also be `async`; trait/impl pairs must agree on asyncness (mismatches are
  lowered as diagnostics) and return `Task`/`Task<T>`. Async lowering synthesises poll/drop shims
  for trait methods and generators alike, recording them in `MirModule::async_plans`. Backends emit
  per-function async vtables (`FutureHeader.VTablePointer` in LLVM, `async_vtable_offsets` in WASM)
  so the executor can drive trait-object futures and async iterators without additional shims.
- Runtime/stdlib surface: `FutureHeader`/`FutureVTable`/`RuntimeContext` define the ABI. The runtime
  exports `async_register_future`, `async_spawn`, `async_block_on`, `await`, `yield`,
  `async_cancel`, `async_task_result`, and `async_token_{new,state,cancel}`; `AwaitStatus` is
  `Pending` = 0 and `Ready` = 1. `Std.Async.Runtime` wraps these as `Spawn`, `BlockOn`, and
  `Cancel`, exposes `Task`/`Task<T>` plus `CancellationToken{Source}`, and provides
  `RuntimeExports.TaskHeader` / typed result helpers for codegen and startup.
- Native codegen branches on `AwaitStatus` from `chic_rt_await`/`_yield`, wiring drop/ready
  blocks and copying results with `chic_rt_async_task_result` using MIR layouts. When no
  executor context is available (e.g., stubbed poll/drop bodies), the runtime blocks until the
  awaited future completes to guarantee progress.
- The WASM backend mirrors the ABI through `chic_rt.await` / `chic_rt.yield`; the
  executor bridge maintains a ready queue and waiter graph, propagates `Cancelled|Faulted` flags,
  and uses linear-memory token/result helpers. Pending awaits return from generated poll functions
  so the scheduler can resume tasks once the awaited header becomes ready.
- Cancellation: `Std.Async.Runtime.Cancel` calls the runtime cancel hook, which sets
  `Cancelled|Completed|Ready` and wakes waiters. `CancellationTokenSource`/`CancellationToken`
  expose token state built on `async_token_*` helpers; cancellation and fault flags surface as
  runtime errors when awaited futures fail.
- Async startup/test flow: `Std.Runtime.Startup.NativeStartup` routes async `Main`/`testcase`
  through the executor via `Std.Async.Runtime.BlockOn`. When `CHIC_SKIP_STDLIB=1` with
  `CHIC_ASYNC_STDLIB_OVERRIDE`/`CHIC_STARTUP_STDLIB_OVERRIDE`, `chic run/test` injects stub
  async/startup modules. Async LLVM tests emit `[SKIP] ... requires the runtime executor`, while the
  WASM harness records pass/fail when the runtime is available and otherwise skips (see
  `tests/backend_validation.rs` and [docs/runtime/async_runtime.md](docs/runtime/async_runtime.md)).
- Using `await` outside an async function/testcase or inside `lock`/`unsafe` blocks is a compile-time
  error; the lowering pass emits diagnostics explaining the restriction.
- The standard executor runs in the native Std runtime; freestanding builds may swap executors via
  configuration while preserving the ABI.
- Examples:

```chic
public async Task<string> FetchAsync(Uri endpoint)
{
    let response = await Http.Get(endpoint);
    return await response.ReadAsStringAsync();
}

public class Client
{
    public async Task ConnectAsync()
    {
        await Socket.Connect();
    }
}
```

- Async functions may appear in interfaces; implementers must match the signature and async
  modifier. Extension methods respect the same rules.

### 2.20 Inline Testing

- Chic supports first-class tests via the `testcase` keyword. Tests may live alongside production code (similar to Rust) or in dedicated files.
- Syntax mirrors lightweight xUnit: `testcase Name()` introduces a zero-parameter test. Prefix with `async` to run inside the async executor; async tests implicitly return `Task`.
- Tests support optional parameters for data-driven scenarios. Parameter types follow the normal type system rules.
- The standard `std.testing` module offers fluent assertions (`Assert.That(actual).IsEqualTo(expected)`, `Assert.That(value).IsCloseTo(target, tolerance)`, etc.).
- Example:

```chic
import Chic.Testing;

testcase ComputesHypot()
{
    Assert.That(Math.Hypot(3, 4)).IsEqualTo(5);
}

async testcase FetchesData()
{
    let payload = await Http.Get("/status");
    Assert.That(payload.Code).IsEqualTo(200);
}
```

- Tests integrate with the `manifest.yaml` project file (see `tests` section) and are discoverable via `chic test`, which will execute each `testcase` in the module using the configured executor/runtime. `chic test` accepts either the project file or a directory containing it, matching the `chic build/run` behaviour.
- The fluent assertion APIs live in the `std.testing` package; documentation and tests should reference `packages/std.testing` rather than ad-hoc stubs in `docs/`.
- Testcases accept optional metadata used by the runner:
  - Stable ids are derived from the fully-qualified name unless overridden via `@id`/`@test_id`/`@testid`.
  - Categories/tags may be attached with `@category`, `@categories`, `@tag`, `@group`, `@test_group`, or `@testgroup`; empty/invalid values surface lowering diagnostics.
  - Unsupported attributes on `testcase` emit diagnostics.
- Runner selection semantics:
  - `--test <pattern>` matches ids, fully-qualified names, or short names (wildcards allowed); `--test-group <pattern>` matches namespace prefixes or tags; `--all`/`--test-all` clears filters.
  - Environment equivalents: `CHIC_TEST`, `CHIC_TEST_GROUP`, `CHIC_TEST_ALL`, `CHIC_TEST_PARALLELISM`, `CHIC_TEST_FAIL_FAST`, and `CHIC_TEST_WATCHDOG_ENABLE_RELEASE`.
  - Discovered/filtered-out counts are reported consistently for LLVM and WASM backends, and scheduling honours `--test-parallel` plus fail-fast.
- Watchdog/loop detection: the interpreter enforces a per-test step counter (default 1_000_000 in debug builds) and optional wall-clock timeout. Configure via `--watchdog <steps>`, `--watchdog-timeout <ms>`, `CHIC_TEST_WATCHDOG_LIMIT[_FORCE]`, or `CHIC_TEST_WATCHDOG_TIMEOUT_MS`; `CHIC_TEST_WATCHDOG_ENABLE_RELEASE` enables the watchdog in release builds. Watchdog failures surface as test failures.

### 2.21 Operator Overloading

Chic adopts C#-style operator declarations inside classes and extensions. Operator bodies are ordinary static methods that participate in overloading and code generation just like regular members, but they follow additional syntactic and semantic rules.

#### Syntax

```
OperatorDecl        ::= attributes? visibility? 'static' OperatorSignature FunctionBody
OperatorSignature   ::= ReturnType 'operator' OperatorSymbol '(' ParameterList ')'
ConversionDecl      ::= attributes? visibility? 'static' ('implicit' | 'explicit')
                        'operator' TargetType '(' Parameter ')'
OperatorSymbol      ::= UnaryOperatorSymbol | BinaryOperatorSymbol
UnaryOperatorSymbol ::= '+' | '-' | '!' | '~' | '++' | '--'
BinaryOperatorSymbol ::= '+' | '-' | '*' | '/' | '%' | '&' | '|' | '^'
                      | '<<' | '>>' | '==' | '!=' | '<' | '<=' | '>' | '>='
```

- Overloadable **unary** operators: `+`, `-`, `!`, `~`, `++`, `--`.
- Overloadable **binary** operators: `+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`, `<<`, `>>`, `==`, `!=`, `<`, `<=`, `>`, `>=`.
- Non-overloadable operators: short-circuiting logical operators (`&&`, `||`), null-coalescing (`??`, `??=`), assignment and compound-assignment forms, member access/indexing (`.`/`?.`/`?.[]`/`[]`), the conditional `?:`, pointer/member arrows, and pattern/relational operators. Truthiness (`true`/`false`) and short-circuit overloading are reserved for a future revision; use `&`/`|` instead of `&&`/`||` when custom semantics are required.
- Explicit user-defined conversions are surfaced through both the C# form `(TargetType)expression` and the Rust-inspired `expression as TargetType`. Implicit conversions are applied automatically in assignment, variable initialisation, and return statements when the compiler can find a matching overload.
- Operator members must include the `static` and `public` modifiers. Additional modifiers (`virtual`, `override`, `async`, `extern`, etc.) are rejected by the parser with targeted diagnostics.
- Unary operators declare exactly one parameter; binary operators declare exactly two. Conversion operators (`implicit`/`explicit`) declare one parameter whose type is the source of the conversion. Generic parameter lists are not permitted.
- Operator declarations are legal only inside classes and extensions. Structs and interfaces rely on value semantics and traits instead of operator bodies.
- At least one operator parameter must be the containing type (or its nullable form). Equality/comparison operators must return `bool`, and `==` must be paired with `!=` (`<` with `>`, `<=` with `>=`). `++`/`--` share the same `op_Increment`/`op_Decrement` implementations for prefix and postfix forms.
- Compound assignments lower through the corresponding binary operator overload; dedicated `operator +=`/`-=` declarations are not yet supported.
- The compiler canonicalises operator names following the .NET metadata convention (`op_Addition`, `op_UnaryNegation`, `op_Implicit_Int32`, …) so MIR lowering and backends can resolve them without mangling collisions. Built-in numeric types surface their operator sets via `operator` members, aligning with the generic numeric traits.
- Cast expressions are parsed as unary operators. `(T)expr` binds tighter than binary operators, while `expr as T` shares precedence with the other binary infix operators in §2.18. Both forms lower to the same conversion pipeline, so parser tests cover ambiguous cases (`(value)` remains grouping when no type name follows the opening parenthesis).
- `as` casts follow Rust's semantics: integer downcasts truncate towards two's-complement, unsigned upcasts zero-extend, and pointer casts bridge via `ptrtoint`/`inttoptr` in backends. Because these operations are unchecked, the compiler emits diagnostics whenever a cast can wrap or drop precision and points developers towards the safer `From`/`Into` or `TryFrom`/`TryInto` traits. These warnings are part of MIR lowering, ensuring LLVM and WASM backends agree on the safety model.
- C-style casts `(<type>) <expr>` are treated as unary operators. Lowering first probes explicit conversion operators between the source and target types; if none match, the compiler falls back to the same numeric, floating-point, and pointer cast pipeline used by `as`. The parser accepts tuple, generic, nullable, and pointer targets (`((int, string))tuple`, `((List<int>?))value`, `(*mut byte)ptr`), while ambiguous parentheses continue to be parsed as grouping so `(value)` remains an identifier wrapped for precedence.
- Numeric conversions are allowed between the built-in integral and floating-point types. Widening conversions are permitted silently; narrowing conversions are explicit-only and continue to surface diagnostics when the cast may truncate or wrap (suppressed inside `unchecked` contexts). Nullable wrappers propagate transparently—`(int?)value` copies the payload and sets `HasValue`, while `(int)nullable` is rejected unless an explicit conversion exists.
- Enums with no payload fields (including `@flags` enums) participate in the numeric pipeline using their declared underlying type (defaulting to `int`/`Int32` for enums without an explicit base). Payload-carrying enums cannot be cast to or from numeric types; the compiler reports that the enum stores data and suggests pattern matching instead. Signedness follows the underlying type rather than assuming flags are always unsigned.
- Reference conversions are currently limited to statically provable upcasts along the class inheritance chain. Casting from a derived class to one of its bases succeeds; the reverse direction is rejected because downcasts require runtime type tests that are not yet implemented. Interface/object casts share the same restriction and are marked **Planned**. Nullable/reference combinations retain their nullability bit but obey the same directionality rules.
- Diagnostics reference the surface syntax. Invalid casts report “no C-style cast …”, pointer conversions warn about “C-style pointer cast …”, and the pointer/integer bridge still demands an `unsafe` context. Tooling can therefore lint C-style casts separately from `as` usage.
- Prefer `as` for purely numeric conversions and `TryFrom`/`TryInto` (or other helper APIs) when failure must be handled. Reserve C-style casts for interop-heavy code or for matching existing C# APIs; linting defaults should flag unchecked C-style pointer/integer casts unless they are wrapped in an `unsafe` region with a clear justification.

#### Examples

```chic
public class Vec2
{
    public float X;
    public float Y;

    public static Vec2 operator +(Vec2 lhs, Vec2 rhs)
        => new Vec2 { X = lhs.X + rhs.X, Y = lhs.Y + rhs.Y };

    public static Vec2 operator -(Vec2 value)
        => new Vec2 { X = -value.X, Y = -value.Y };

    public static implicit operator Vec2(Span<float> data)
    {
        return new Vec2 { X = data[0], Y = data[1] };
    }
}

public extension Angle
{
    public static Angle operator +(Angle lhs, Angle rhs)
        => Angle.FromRadians(lhs.Radians + rhs.Radians);

    public static explicit operator double(Angle angle) => angle.Radians;
}
```

#### Resolution & Diagnostics

- Overload resolution inspects the operand types during MIR lowering. Operators must be unambiguous: the compiler errors when multiple overloads match the same operand combination or when no viable overload exists. Built-in numeric operators participate as `operator` members when the stdlib is available and fall back to intrinsic lowering for primitives when it is not.
- Conversions are applied automatically in assignment and call positions that request the target type; `explicit` conversions require an explicit cast expression (sugar for invoking the generated operator method).
- When only explicit conversions are available, implicit contexts (variable initialisers, returns, etc.) emit diagnostics pointing out that an explicit cast is required. Cast expressions surface the conversion call directly so codegen still benefits from overload dispatch.
- Ambiguous conversions (two or more overloads match the same source/target pair) produce lowering diagnostics that enumerate the candidate method names to aid disambiguation.

### 2.22 Function Overloads

Chic groups every declaration that shares a canonical name into an overload set. Overloads allow APIs to tailor surface signatures for different argument counts, receivers, or modifiers without sacrificing determinism.

#### Overload Sets

- **Free functions**: all declarations with the same name in a namespace form one set (`Math::Combine`).
- **Methods**: each type (& its extensions/impls) owns sets per member name (`Widget::Render`). The implicit receiver participates in applicability checks.
- **Constructors**: desugared into the synthetic `Type::init` set, regardless of whether call sites use `new Type(...)` or functional syntax (`Type(...)`).
- **Operators**: operator declarations are mapped into method-style sets so overload resolution aligns with normal call sites.

#### Applicability

Chic evaluates every candidate overload using the following rules (in order):

1. **Accessibility** – candidates whose `Visibility` prevents the current scope or `Self` type from observing the member are discarded (`protected` requires the caller’s enclosing type to inherit from the declaring type, `internal` requires matching namespaces, etc.).
2. **Generic instantiation** – explicit generic arguments are applied first. Remaining type variables are inferred from the provided arguments; candidates whose constraints cannot be satisfied are rejected.
3. **Parameter modifiers** – the call’s argument modifiers must match the parameter’s mode exactly (`value`, `in`, `ref`, `out`). Implicit receivers are treated as the leading argument.
4. **Arity and defaults** – the number of supplied arguments (after inserting the receiver) must be within the inclusive range `[required parameters, total parameters]`. Parameters with defaults satisfy missing arguments. Named arguments are reordered before comparison.
5. **Type matching** – each supplied argument type must exactly match the parameter type after applying nullability annotations. (Implicit conversions are intentionally deferred to a future spec update.)
6. **Static/instance form** – instance calls (`expr.Member(...)`) only match non-static methods. Qualified calls (`Type.Member(...)`) match static methods or free functions; constructors always bind instance slots.

Constructors reuse the same machinery with an implicit `out` parameter for the destination object. Accessibility is determined by the constructor’s declaring type and visibility.

#### Ranking & Tie-Breaking

- Candidates earn 2 points for every explicit argument that matches a parameter and 1 bonus point when an overload succeeds only because of default parameters. The winner is the overload with the highest score, meaning overloads that consume more explicit arguments out-rank ones that rely on defaults.
- If multiple candidates share the highest score, the call is ambiguous and the compiler requires the user to disambiguate (by qualifying the member, adding explicit arguments, or using named arguments).
- Calls that fail to match any overload emit `CALL_OVERLOAD_NO_MATCH` (type checker diagnostic `[TCK141]`); constructor-specific errors surface as `[TCK131]`. Ambiguities emit `CALL_OVERLOAD_AMBIGUOUS` (`[TCK142]` / `[TCK132]` for constructors).

#### Diagnostics

- *No match:* “no overload of `Foo::Bar` matches the provided arguments; available: Foo::Bar(int), Foo::Bar(int, int)”.
- *Ambiguous:* “call to `Foo::Bar` is ambiguous; matching overloads: Foo::Bar(int), Foo::Bar(int, string)”.
- *Unresolved target:* “cannot resolve call target for `Foo::Bar`”.

#### Additional Notes

- Constructors follow the same accessibility, modifier, and default-argument rules. `new Type(args...)` and `Type(args...)` are aliases.
- Operator bodies cannot yield `void`. Conversion operators must return the declared target type exactly.
- Future revisions will extend applicability with implicit numeric and reference conversions; until then, overloads are only selected when arguments already match the declared types.

#### Best Practices

- Choose distinct arities when possible so resolution never falls back to tie-breaking. When defaults are required, prefer placing optional parameters at the end of the signature.
- Keep operator bodies thin; forward to named helper methods when logic spans multiple branches. This keeps diagnostics and profiling easier to follow.
- Avoid implicit conversions that lose information. Reserve `implicit` for value-preserving projections and provide `explicit` when a narrowing conversion is required.
- Pair equality operators with `@derive(Equatable)` or explicit overrides of `Equals`/`GetHashCode` so collections continue to behave correctly.

### 2.23 Optional Parameters & Default Arguments

- **Declaration rules.** Optional parameters must trail required ones and cannot use `ref`, `out`, or the implicit receiver binding. Violations emit **TCK044**/**TCK045**. Multiple declarations for the same member must reuse the exact textual default; otherwise **TCK046** points at both declarations.
- **`default` value expressions.** Chic supports `default` (contextual) and `default(T)` (explicit) expressions. `default` requires an expected type from the surrounding context (annotation, assignment target, or return type) and lowers to an explicit zero-initialisation of that type. `default(T)` provides the type directly. Value types zero-fill their payloads; nullable references become `null`; non-nullable reference types reject `default` with a diagnostic. MIR lowers `default` via explicit `ZeroInit` statements—no hidden runtime helpers or constructors run.
- **Expression forms.** Default expressions accept any Chic expression that would be legal in a method body: literals, `new` expressions, namespace/static calls, lambdas, tuple/array literals, etc. The parser records both the raw text and AST so later stages can evaluate or re-lower the expression as needed.
- **Evaluation semantics.** After positional and named arguments are bound, missing slots are filled left-to-right. Each default is evaluated exactly once per call—either by constant folding or by invoking a synthesized thunk. Thunks capture the surrounding type metadata so generic defaults (`Fallback<T>()`, `new T()`) observe the same instantiation as the body. Defaults run synchronously; `await` is not permitted inside the expression.
- **Named arguments & overloads.** Named arguments may skip past optional parameters (`Combine(start, scale: 3)`) and overload resolution (§2.22) awards one bonus point to the candidate that consumes the fewest defaults, keeping selection deterministic.
- **Backends & metadata.** MIR records `DefaultArgumentRecord` entries for every synthesized argument. Codegen surfaces them via `append_default_argument_metadata` so tools and debuggers can explain where synthesized values originated.
- **Reference fixtures.** `tests/spec/optional_parameters.cl` exercises positional, named, constructor, and thunk defaults.

#### Readonly Structs & Intrinsic Layout

- `readonly struct` declarations guarantee that the implicit `this` receiver is immutable outside of constructors. Assignments to fields (including auto-property backing storage) are only permitted inside the struct’s own constructors; the MIR builder rejects mutations that occur in methods, property setters, or extension helpers with the diagnostic “readonly struct `<T>` fields can only be assigned within its constructors.”
- Individual fields can also be marked `readonly`. The compiler permits assignments to a readonly field only in its declaration or within the declaring type’s constructors when targeting the current instance (`this`). Object initialisers and mutations performed through other instances are rejected with “readonly field `<Type>::<Field>` can only be assigned within its constructors.”
- Readonly semantics compose with `ref`/`in` methods: instance methods continue to receive `self` by immutable borrow and the borrow checker enforces that the receiver is never written to.
- The `@Intrinsic` attribute marks structs whose layout or behaviour is provided by the runtime. The lowering pipeline threads this flag through `TypeInfo`, MIR type layouts, and codegen metadata so drop glue, reflection, and backend-specific emitters can special-case runtime-provided types (for example, `Std.Numeric.Int32`, `Std.Numeric.Decimal.DecimalIntrinsicResult`, and interop helper structs).
- `@StructLayout(LayoutKind.Sequential, Pack=?, Align=?)` overrides the default struct layout derived from field order and target ABI. `LayoutKind.Sequential` maps to `repr(c)` on the bootstrapper; `Pack` and `Align` accept positive integers and clamp to the active pointer alignment. The parser accepts the `LayoutKind.Sequential` token directly without requiring a namespace qualifier.
- `@StructLayout` composes with `@repr(c)`/`@align(n)`—the compiler merges the hints and records the resulting metadata alongside the MIR type layout so both LLVM and WASM backends honour the requested packing/alignment.

### 2.9 Decimal Arithmetic Intrinsics

Chic exposes first-class decimal intrinsics through the numeric library (`Std.Numeric.Decimal.*`). Each helper mirrors the surface arithmetic (`Add`, `Sub`, `Mul`, `Div`, `Rem`, and `Fma`) and returns a `DecimalIntrinsicResult` containing the operation status, the resulting `decimal`, and the execution variant (currently always `Scalar`; the `Simd` slot is reserved for a future Chic-native SIMD implementation). The APIs accept optional arguments for `DecimalRoundingMode` (defaulting to ties-to-even) and `DecimalVectorizeHint` (`None` or `Decimal`), enabling explicit control over rounding semantics and future SIMD hints. Constant-folded expressions run through the same MIR nodes, so compile-time evaluation and runtime execution share identical behaviour. The library documents every entry point so IDEs and generated reference docs surface rounding, vectorisation, and diagnostic guidance directly within the standard library.

The runtime ABI between Chic code and the Rust runtime is backed by explicit structs: `DecimalRoundingEncoding` wraps the rounding-mode discriminant in a `@StructLayout(LayoutKind.Sequential)` payload so every backend (LLVM/WASM) passes a uniform 32-bit value, and the pointer-based kernels (`Sum`, `Dot`, `MatMul`, and the clone shim) consume `DecimalConstPtr`/`DecimalMutPtr` wrappers instead of raw `isize` handles. MIR/LLVM lowering automatically coerces `DecimalRoundingMode` operands into the encoding struct, ensuring the generated calls match the runtime signature on every target triple.

Runtime status codes are shared across Chic and Rust: `Success`, `Overflow`, `DivisionByZero`, `InvalidRounding`, `InvalidFlags`, `InvalidPointer`, and `InvalidOperand`. The vectorize flag is reserved; all calls currently pass `0` and lower to the scalar runtime entry points (`chic_rt_decimal_{add,sub,mul,div,rem,fma}`), and `DecimalIntrinsicVariant` remains `Scalar`.

Cloning `decimal` values now routes through `Std.Numeric.Decimal.Intrinsics.CloneExact`, which in turn calls the runtime’s `chic_rt_decimal_clone` via typed pointers. Even when `__clone_glue_of<decimal>()` returns `null` (because the type is structurally Copy), `Std.Clone.Runtime.CloneField` detects the decimal type and invokes the runtime shim, guaranteeing that aggregate clone glue never falls back to `Std.Clone::Impl::Identity` for decimal fields.

Functions that are annotated with `@vectorize(decimal)` are marked as SIMD candidates; the builder lowers matching intrinsic calls into `Rvalue::DecimalIntrinsic` nodes tagged for vector dispatch. Two diagnostics enforce correct usage:

- **DM0001** is emitted when a function carries `@vectorize(decimal)` but the lowered body contains no decimal intrinsics. Remove the attribute or introduce `Std.Numeric.Decimal.Fast` helpers to justify the hint.
- **DM0002** is emitted when decimal intrinsics appear in a function without `@vectorize(decimal)`. Annotate the function or migrate to `Std.Numeric.Decimal.Fast` to unlock SIMD-aware codegen.
- Both diagnostics ship with fix-it suggestions. `chic --diagnostics-format=rich` (or `chic lint`) presents `apply fix` actions that either add/remove the attribute or rewrite hot loops to call `Std.Numeric.Decimal.Fast` so teams can standardise on the SIMD-aware wrappers quickly.

`constexpr` code enjoys the same surface: decimal literals, arithmetic (`+`, `-`, `*`, `/`, `%`), and the `Std.Numeric.Decimal.Intrinsics` entry points all fold during constant evaluation. The const-eval engine reuses the runtime implementation (via `Decimal128`) so literals, constexpr functions, and static field initialisers observe the exact same rounding and overflow semantics as their runtime counterparts.

Tooling consumes vectorisation metadata via reflection. Functions annotated with `@vectorize(decimal)` emit a `vectorize=decimal` flag in `reflection.json`, allowing IDEs, linters, and documentation generators to surface SIMD intent without re-parsing source attributes.

The `Std.Numeric.Decimal.Fast` namespace layers higher-level primitives (`Sum`, `Dot`, and `MatMul`) over the intrinsics. They consume spans and currently route to the scalar kernels; vectorize hints are accepted but do not change runtime selection until Chic-native SIMD lands. Empty spans fold to zero, mismatched input shapes map to `DecimalStatus::InvalidOperand`, and matrix multiplication validates both source and destination layout before invoking the runtime. Callers may still inspect the reported `Variant`, but it will always be `Scalar` until SIMD lowering is reintroduced on the Chic side.

Performance is validated via targeted benchmarks (scalar and SIMD variants) and should remain stable as additional optimisations land.

**Choosing Scalar vs SIMD** Until Chic-native SIMD decimal lowering is available, `DecimalIntrinsicResult.Variant` is always `Scalar` and vectorize hints are advisory only. Feature overrides that previously selected SIMD are ignored; deterministic behaviour is therefore identical across builds.

#### Decimal Intrinsic Examples

```chic
import Std.Numeric.Decimal;
import Std.Span;

namespace Finance;

@vectorize(decimal)
public decimal Dot(decimal[] lhs, decimal[] rhs)
{
    var spanLhs = lhs.AsReadOnlySpan();
    var spanRhs = rhs.AsReadOnlySpan();
    var result = Std.Numeric.Decimal.Fast.Dot(spanLhs, spanRhs);
    if (result.Status != DecimalStatus.Success)
    {
        return 0m;
    }
    // `result.Variant` reports Scalar vs Simd so telemetry can attribute speed-ups.
    return result.Value;
}
```

The compiler lowers the call to `Std.Numeric.Decimal.Fast.Dot` into MIR decimal intrinsics, applies vectorization metadata, and selects the appropriate runtime kernel (scalar or SIMD) for the host CPU.

## 3. Ownership & Borrowing

- Move semantics by default; moved-from values become invalid until reinitialized.
- Borrow qualifiers on parameters:
  - `in` → shared, read-only borrow (multiple allowed).
  - `ref` → unique, mutable borrow (only one at a time).
  - `out` → unique borrow that must be assigned before scope exit.
- Deterministic destruction via `dispose(ref this)`.
- No null in safe code. `T?` is sugar for `Option<T>` with pattern shorthand:

```chic
if (let line? = file.ReadLine()) { print(line); }
```

- Example moves and borrows:

```chic
var buf = new Buffer(1024);
var other = move buf; // buf invalid thereafter

public void Push(ref this Vec<int> v, int value) { /* mutate via unique borrow */ }
public int? Get(in this Vec<int> v, usize i) { /* borrow read-only */ }
```

### 3.1 Stack vs Heap Allocation

### 3.3 Lending Returns and View Fields

Chic borrows remain second-class and cannot escape call scopes by default. Two mechanisms support zero-copy patterns without explicit lifetime syntax:

- **Lending returns** expose an explicit `lends(...)` clause on function signatures. A lending return is only valid when the return type is a `view` and every named source is an `in`/`ref` parameter whose type is also declared `view`:

  ```chic
  public view Span<byte> Slice(in view string src, int start, int len) lends(src)
  {
      return src.AsUtf8Span().Slice(start, len);
  }
  ```

The compiler rejects unknown `lends` targets, non-borrowed parameters, non-view parameters, or non-view return types ([TCK180–TCK183]). During MIR lowering, returning a value that does not originate from a listed lender yields a lowering diagnostic. Borrow-escape constraints are still emitted for other parameters so the rules remain consistent.

- **View fields** store non-owning projections of other fields in the same aggregate:

  ```chic
  public struct LineView
  {
      public string Buffer;
      public view str Slice of Buffer;
  }
  ```

  The `of Buffer` clause records the dependency so drop order and move validation can respect the owner/view relationship. Owners must be initialised before dependent views and cannot be moved while dependent views remain live; drop lowering destroys views before their owners.

- **Tooling and runtime visibility.** MIR signatures propagate `lends_to_return` into metadata payloads as `lends_return:Type::Func=param,...`, and aggregate layouts surface `view` dependencies as `view:Type::Field=of:Owner`. IDEs and lints can use these entries to highlight escapes or stale views. The runtime exposes allocation telemetry via `chic_rt_alloc_stats/reset`, counting allocation/reallocation/free events and bytes, so CI/perf gates can validate zero-copy lending paths against cloning fallbacks.

#### Lending Return Examples

```chic
public view str Substr(in string source, int start, int len) lends(source)
{
    return source.AsUtf8Span().Slice(start, len).ToStr();
}

public view Span<byte> Zip(view Span<byte> a lends(a), view Span<byte> b lends(b)) lends(a, b)
{
    // ... return a view that cannot outlive either input
}
```

The compiler statically enforces that the returned views do not outlive their donors and surfaces diagnostics when misuse is detected.


Chic follows a predictable allocation model so developers can reason about data layout and lifetimes without a garbage collector. The compiler prefers stack allocation for value semantics and resorts to the heap only when identity, dynamic size, or sharing is required.

- **Deterministic placement:** Value types (`struct`, `enum`, tuples) materialise on the stack. Heap promotion occurs only through explicit constructors (`new`, `Box`, `Vec`, `String`, etc.) or when a developer applies `@promote_large` to locals whose static footprint exceeds 8 KiB.
- **Pinned locals:** `@pin` and `Pin<T>` keep storage stationary across `await` suspension points and FFI calls. The async state machine records pinned locals so the executor preserves their addresses while the task is suspended.
- **Intrinsic containers:** `Vec`, `Array`, `Span`, `string`, and `str` expose stack-resident metadata (pointer/len/cap) and heap-owned buffers. When the caller is in the defining crate (or the type opts into `@inline(cross)`), MIR lowers `Len`, indexing, and `foreach` directly to pointer arithmetic plus bounds checks. Cross-crate callers without the opt-in continue to route through the shared library entry points, so ABI stability remains intact while fast paths stay available to the owners.
- **Stack iterators:** `foreach` loops over intrinsic containers never materialise heap enumerators. The compiler synthesises locals for the pointer, length, and index, emits the `index < len` guard inline, and drives the loop with explicit pointer math. The iteration binding still follows the same drop rules as enumerator-based loops—`StorageLive`/`StorageDead` pairs are emitted per iteration and Chic defers a drop on the binding so user-authored `dispose` implementations and auto-drop glue run before the synthesized locals go dead.
#### Cross-Crate Inline Opt-In

Aggregate declarations may carry `@inline(local)` (default) or `@inline(cross)`. The latter signals that pointer-metadata layouts are considered ABI-stable so other crates may inline the intrinsic fast paths. Types without the opt-in continue to expose only the exported helpers at crate boundaries, guaranteeing deterministic incremental builds and simplifying hot patching.

- Tooling records the opt-in in `.clrlib` metadata (`inline:<Namespace.Type>=cross`) so builds can audit every escape hatch.
- **Deterministic destruction:** The compiler emits explicit `Deinit`/`Drop` MIR operations in reverse lexical order before final `StorageDead` statements, guaranteeing predictable tear-down. Drops are scheduled for every exit edge—fallthrough, `return`, `throw`, `break`/`continue`, `goto`, and async/generator teardown—so each owned value is destroyed exactly once regardless of control-flow shape.

`foreach` loops over intrinsic containers lower to stack-based iteration without allocating intermediate iterator objects.

#### Stack-Resident Values

- **Local bindings of value types** (`struct`, `enum`, tuple literals, fixed-size arrays) are emitted inline on the stack by default. Moving such a value transfers the entire payload to the destination stack slot or aggregate field without implicit heap traffic.
- **Borrowed parameters** (`in`, `ref`, `out`) never allocate: they carry a pointer and metadata referring to an existing allocation (stack, heap, or static). Their lifetime is bounded by the borrow checker and cannot outlive the enclosing call.
- **`Span<T>` / `span<T>`** and similar view types are lightweight descriptors (pointer + length [+ stride]) that live on the stack even when they refer to heap-backed buffers.
- Span constructors (`Span<T>.FromValuePointer`, `Span<T>.StackAlloc`, `ReadOnlySpan<T>.FromValuePointer`, the string/array bridges) forward `Std.Runtime.Collections.ValueMutPtr` / `ValueConstPtr` handles—typed structs that carry the pointer plus its `usize` stride and alignment—so the runtime never has to guess at element metadata or consume untyped `byte*` arguments.
- **Closures and async/generator state machines** lower to plain structs. When a closure/state machine stays within a stack scope (e.g., immediately invoked or awaited), it remains stack-resident; moving it into a heap-owning container transfers the struct by value.
- **`Option<T>` / `T?`** reuse the storage of `T` (a niche optimisation); the discriminant is stored on the stack alongside the payload.

#### Heap-Backed Values

- **`class` instances** are always heap-allocated and referenced via implicit pointers. Constructors (`new`) reserve heap storage and invoke field initialisers; `dispose(ref this)` runs deterministically when the owning reference drops.
- **Reference-counted or owning containers** (`Vec<T>`, `Array<T>`, `String`, `Rc<T>`, `Arc<T>`) manage heap buffers internally. The container object itself (metadata such as pointer/len/cap) is a stack value; its elements reside on the heap.
- The bootstrap standard library re-exports the raw collection intrinsics via `Std.Collections`.  It ships mirrors of the runtime layout (`VecPtr`, `VecViewPtr`, `VecIterPtr`, `ArrayPtr`) alongside the `VecError` enum and provides two helper surfaces:
- `VecIntrinsics`, which forwards the `chic_rt_vec_*` functions verbatim for advanced callers that need exact control over drop callbacks and raw pointers. Matching helpers exist for the runtime array hooks (`chic_rt_array_*`).
- `Vec`, a light façade that computes element size/alignment for callers. `Vec.New<T>()` / `Vec.WithCapacity<T>(capacity)` return initialised `VecPtr` handles with the appropriate drop glue registered for `T`, while overloads that accept explicit size/alignment remain for advanced scenarios. `Vec.IsEmpty`, `Vec.Len`, `Vec.Capacity`, `Vec.View`, `Vec.Data`, `Vec.Clone`, and `Vec.Iter` expose the common read-only queries without re-specifying runtime symbols. Mutation commands (`Reserve`, `Push`, `Remove`, …) continue to rely on the raw `VecIntrinsics` surface while a borrow-safe layer is designed.
- `Array` and `VecView` helpers mirror the vector queries for fixed-size buffers so callers can check `Len`, `IsEmpty`, or obtain a view without threading linker symbol names through application code.
- Conversion helpers (`Vec.ToArray`, `Vec.IntoArray`, `Array.ToVec`, `Array.IntoVec`) wrap the runtime entry points defined in [docs/runtime/vec_array_conversions.md](docs/runtime/vec_array_conversions.md). `To*` methods clone buffers and leave the source intact, while `Into*` methods transfer ownership, reusing storage when the buffer is already tightly packed and allocating a trimmed copy otherwise. All helpers return `VecError` so callers can surface allocation failures.
##### Std.Span Surface

- `Std.Span` complements the collections façade with ergonomic `Span<T>` / `ReadOnlySpan<T>` wrappers.  The structs are thin views over the runtime `ChicSpan` mirrors (pointer, length, element stride) and expose helpers for constructing spans from typed-pointer handles (`FromValuePointer`), vectors, arrays, or UTF-8 strings.  Slicing uses the runtime’s bounds-checked helpers (`Slice(start, length)` plus the single-argument `Slice(start)`), `Span<T>.Empty` manufactures a zero-length view, and both `Span<T>.StackAlloc(len)` and `Span<T>.StackAlloc(ReadOnlySpan<T>)` lower to the dedicated MIR intrinsic so borrow checking can reason about stack scratch buffers.  `Span<T>.CopyTo` / `Span<T>.CopyFrom` and `ReadOnlySpan<T>.CopyTo` provide typed, allocation-free copies, while `ReadOnlySpan.FromString` and the `string.AsUtf8Span()` / `string.TryCopyUtf8(span, out written)` extensions provide allocation-free UTF-8 bridges.  `Utf8String.FromSpan(ReadOnlySpan<byte>/Span<byte>)` converts stack data back into owned `string` values without touching `string::from`.  These helpers deliberately avoid unsafe indexing APIs; code that needs element access can rehydrate the view into a container or use the underlying collection intrinsics. All of the entry helpers marshal through the `Value{Const,Mut}Ptr` typed-pointer structs so element size/alignment travel with the pointer.
  The runtime span mirrors reserve a 16-byte padding slot after the `Value{Const,Mut}Ptr` header to keep the metadata 16-byte aligned and preserve ABI space for future span state without breaking layout; the visible fields remain `data`, `len`, `elem_size`, and `elem_align`.
  `Span<T>.StackAlloc(ReadOnlySpan<T>)` (or any span argument) coerces the source length to `usize`, emits the stack allocation intrinsic, and immediately calls `chic_rt_span_copy_to` so the scratch buffer is prefilled without helper calls in Chic code.  Derived slices/read-only views retain stackalloc provenance, so async lowering rejects `await` points while any stack-backed span view is still live.
  `Utf8String.FromSpan` inspects the typed handle before calling into the runtime so only byte-sized, byte-aligned spans are accepted, and `Std.Memory.StackAlloc` exposes a shared façade for requesting stack-backed spans or raw typed handles when non-span helpers (numeric formatting, UTF-8 staging, runtime adapters) need allocation-free buffers.
- The runtime `SpanPtr`, `ReadOnlySpanPtr`, and `StrPtr` structs are all `@repr(c)` mirrors with typed
  pointer fields (`*mut @expose_address byte` / `*const @expose_address byte`).  Constructors funnel
  through the exported span intrinsics (`chic_rt_span_from_raw_{mut,const}`) so Chic code
  marshals `Value{Const,Mut}Ptr` handles for Vec/Array/string/stackalloc bridges instead of calling
  bespoke container shims.  `Span<T>.Raw` / `ReadOnlySpan<T>.Raw` surface those handles for
  interop-heavy code, while UTF-8 helpers (`string.AsUtf8Span`, `Utf8String.FromSpan`) simply forward
  slices to `StrPtr` and the runtime string bridge.

##### Range Expressions
- Syntax follows the C# range/indexing model: `start..end` (exclusive upper bound), `start..` (open upper bound), `..end` (from zero), `..` (full range), and `..=` variants for inclusive upper bounds. Prefix `^expr` builds an index from the end of the sequence and can appear on either side of the operator. Chained operators such as `a..b..c` are rejected with a dedicated diagnostic.
- Precedence sits below other binary operators and above the conditional/assignment tier, so `a + 1..b` parses as `(a + 1)..b` while `(a..b) ? c : d` treats the range as a single operand. Operands evaluate left-to-right before the range is materialised; `^expr` captures its operand before any bounds checks.
- Semantics: `^` constructs `Std.Range.Index { Value, FromEnd }`; ranges produce value types (`Std.Range.Range`, `RangeFrom`, `RangeTo`, `RangeInclusive`, `RangeFull`) without allocations. Exclusive ends permit `end == len`; inclusive ends require `end < len`. Invalid bounds (`start > end`, `end` past the sequence, `^0`) trap with **RangeError.OutOfBounds**.
- Slicing and indexing: intrinsic containers (`Span<T>`, `ReadOnlySpan<T>`, `Vec<T>`, `Array<T>`) plus string/str views accept range indices. The compiler lowers `span[1..^1]` into a bounds-checked slice that computes offsets from the span’s length, returns a new view without copying, and panics using the runtime `SpanError` codes on failure. Single-element indexing with `^expr` subtracts from the container length before performing the usual bounds check. Strings expose `ReadOnlySpan<byte>` slices to mirror `AsUtf8Span`.
- Iteration: `foreach` over `Range`/`RangeInclusive` enumerates `start..end` (exclusive unless `..=` is used) without allocating enumerators; the inline fast path reuses the `idx` locals emitted for direct container iteration. Open-ended ranges (`..end`, `start..`, `..`) and index-from-end bounds are rejected with **RangeError.Invalid** because there is no ambient length to resolve them against.

Stack-allocated spans behave like short-lived borrows of stack memory.  The borrow checker rejects
`await` / generator suspension points when a stackalloc span is still live, and async lowerings emit
the same diagnostics (`TCK4012`) as ordinary stack borrows.  Dropping or copying the span into a heap
buffer before the suspension point releases the synthetic loan, allowing async tasks and generators
to proceed without pinning the surrounding locals.
  - The collections façade now forwards `Array<T>.AsReadOnlySpan()` and its extension-form `array.AsReadOnlySpan()` through the span runtime intrinsics, so user code does not interact with raw pointers when requesting readonly views.
- **Boxed trait objects and interface dispatch** allocate a heap shell when a value type must adopt reference semantics (e.g., storing a `struct` behind an interface pointer).
- **Long-lived async tasks**: calling an `async` function produces a `Task`/`Task<T>` wrapper. The lower-level state-machine struct is moved into the executor, which pins it on the heap while the task is scheduled. Short-lived tasks that are awaited immediately can remain on the stack until the await point transfers ownership to the runtime.
- **Task/future layout**: `Std.Async` exposes a stable ABI for `FutureHeader`, `Future<T>`, and `Task<T>` so runtime shims and generated code can exchange pointers without reflection. Headers store the state pointer, vtable, executor context, and flags; typed tasks append an `InnerFuture` whose `Result` field contains the awaited value. MIR now records concrete offsets for generic `Task<T>`/`Future<T>` instantiations and both LLVM/WASM consume those layouts for flag/result projection. Layout details live in [docs/runtime/async_runtime.md](docs/runtime/async_runtime.md).
- All async runtime structs (`FutureHeader`, `FutureVTable`, `Future`, `Future<T>`, `Task`, `Task<T>`) are `@repr(c)` so their field order matches the native runtime definitions in `src/runtime/async_runtime.rs`.
- **Strings, closures, or arrays that escape their defining scope** are promoted to the heap via the containers above. Promotion is explicit—there is no automatic escape analysis that spills values behind the developer’s back.
- **FFI handles** (`@extern` resources, GPU buffers, OS descriptors) live wherever the callee API dictates. Chic models them as owning structs whose destructor releases the foreign resource; the struct itself obeys the usual stack/heap rules depending on where it is stored.

##### Drop Glue

- The compiler synthesises *drop glue* for every non-trivially-droppable type. Drop glue is emitted as an `extern "C"` thunk named `__cl_drop__…` (sanitised from the canonical type name) that receives a raw pointer and performs the same deterministic destruction the borrow checker already expects: it runs `dispose(ref this)` when present, then recursively drops fields/variants in declaration order while honouring nullable payloads, tuple destructors, and union views.
- Callers obtain the glue through the intrinsic `__drop_glue_of<T>() -> (fn @extern("C")(void*) -> void)?`. It returns `null` for trivially droppable types so containers can skip any callback; otherwise it yields the address of the generated `__cl_drop__T` thunk so runtimes can safely destroy elements (e.g., `Vec<T>` reallocation, slice clears, async unwinds).
- Glue is generated during monomorphisation so each concrete instantiation (`Vec<int>`, `struct Foo<T> where T=Bar`, etc.) gets an ABI-stable Chic entry point. Auto traits (`ThreadSafe`, `Shareable`) flow through the glue metadata so runtime containers can enforce send/share rules when invoking it.
- Structural drops follow RAII ordering guarantees: fields are destroyed from last to first, enum payloads drop before the discriminant leaves scope, and `finally`/async unwind paths invoke glue exactly once. Tail recursion is elided where possible so glue stays leaf-safe for WASM targets.
- The compiler always prefers user-authored `dispose(ref this)` implementations. When present, glue simply casts the pointer and calls the user body. When absent, glue expands to element-wise drops, leveraging the same MIR lowering as inline destructors to keep behaviour consistent.
- Containers and runtime helpers obtain the pointer via the intrinsic `__drop_glue_of<T>() -> fn(*mut u8)?`. It returns `null` for Plain-Old-Data types so callers can omit callbacks entirely; otherwise it exposes the monomorphised glue function. The standard library substitutes the shared runtime no-op (`Std.Runtime.DropRuntime.DropNoopPtr()`, which resolves to `__drop_noop`) when the intrinsic yields `null`, keeping APIs like `Vec<T>` ergonomic without baking drop decisions into the runtime.
- Glue is part of the compilation unit’s symbol table. Both LLVM and WASM backends emit it with Chic ABI so native and WebAssembly executors can invoke drops uniformly, including bulk drops (`drop_in_place`, slice/array clears, task unwinds). Generated symbols are internal unless referenced through `__drop_glue_of<T>()`.

#### Copy, Clone, and Moves

- **Clone trait.**

  ```chic
  public trait Clone
  {
      Self Clone(in this);
  }
  ```

  Clone is explicit—developers either implement the trait or attach `@derive(Clone)` (currently
  limited to non-generic structs/classes just like `@derive(Equatable)`). Derives expand to concrete
  field-wise clones so MIR remains inspectable. Calling `Clone()` borrows the receiver (`in this`),
  runs user code, and returns an owned `Self`. The trait is infallible; allocation failures bubble out
  via panic/abort today while a future `TryClone<TError>` covers fallible paths.

- **Copy auto-trait.** Copy joins `ThreadSafe`/`Shareable` in the auto-trait set. A type is `Copy`
  when it has no `dispose(ref this)`, contains only `Copy` fields/views (including enum variants), and
  lacks an explicit opt-out. `@copy` forces the trait (subject to the structural checks) while
  `@not_copy` forbids it. Copy types automatically satisfy `Clone`; lowering simply emits a bitwise
  move. This keeps POD clones zero-cost while still allowing non-Copy types (those with destructors)
  to opt in via manual trait implementations.

- **Moves.** Moves remain MIR primitives. Assignments evaluate the RHS into a temporary, drop the
  previous value, then move the temporary into place. Moving a non-Copy value invalidates the source
  binding/field until it is reinitialised; the borrow checker reports “use of moved value” errors for
  later reads. Copy types skip the invalidation.

- **Clone glue intrinsic.** Runtime-owned containers occasionally need to duplicate elements via
  erased pointers. The intrinsic `__clone_glue_of<T>() -> (fn @extern("C")(*const byte, *mut byte)
  -> void)?` mirrors `__drop_glue_of`: it returns `null` for `Copy` types and yields a monomorphised
  thunk for everything else. The thunk receives `src`/`dest` pointers, reborrows `src` as `in this`,
  calls `Clone()`, and writes the result into `dest`.

- **Nullable results.** `T?` remains sugar for `Option<T>`; nullable helper syntax (`if (value?)`,
  `??`, pattern matching) simply manipulates the Option representation. APIs such as
  `Weak<T>.Upgrade() -> Arc<T>?`, `Arc<T>.GetMut() -> Std.Sync::ArcMutableRef<T>`, and
  `Arc<T>.TryUnwrap() -> Std.Result<T, Arc<T>>` therefore compose directly with existing nullable
  diagnostics and pattern sugar.

These rules keep Chic aligned with Rust’s ownership model while maintaining the spec’s
“everything is visible in MIR” principle: clones are explicit method calls, Copy inference happens in
the same pass that already computes `ThreadSafe`/`Shareable`, and runtime hooks (`__drop_glue_of`,
`__clone_glue_of`) remain the only reflection needed for containers.

#### Deterministic Destruction

- Every heap allocation is paired with a deterministic destructor. When the last owning value goes out of scope (stack unwinds, container drops, executor completes a task), `dispose` runs immediately, even in the presence of panics (`panic = abort` on freestanding targets).
- Stack values drop in reverse lexical order. If a stack value owns heap storage (e.g., `Vec<T>`), the buffer is reclaimed during its drop.
- Moving a value transfers responsibility for its destructor. Borrowing does not affect ownership; the original owner remains responsible for cleanup.
- MIR lowering records cleanup explicitly: before a `StorageDead` consumes a slot, the compiler materialises `Deinit`/`Drop` statements so destructors and nested fields run on every exit path (fallthrough, `return`, `throw`, async cancellation).

#### Guidelines

- Prefer value types (`struct`, `enum`) when identity or shared mutation is unnecessary; they benefit from stack allocation and eliminate allocator traffic.
- Reserve `class` for types that require reference identity, inheritance, or polymorphic lifetimes beyond a single scope.
- Use containers with explicit ownership semantics (`Vec`, `Box`, `Rc`, `Arc`) when data must outlive the creating frame or be shared. The compiler’s borrow checker enforces correct usage regardless of storage location.
- Avoid placing extremely large buffers on the stack; favour explicit heap allocation (e.g., `Vec::with_capacity`) for multi-page payloads to keep stack frames predictable.

#### Shared Ownership with `Rc<T>` and `Arc<T>`

- `Rc<T>` provides single-threaded reference counting. Cloning an `Rc<T>` performs a borrow-checked increment in the runtime, while dropping the value decrements the counter and frees the buffer when it reaches zero. The type is **Shareable** but **not ThreadSafe**, so the borrow checker will reject unique borrows across `await` boundaries or thread hand-offs.
- `Arc<T>` mirrors the API but uses atomic counters so it can cross task and thread boundaries. `Arc<T>` inherits the auto-trait status of its payload: it is `ThreadSafe` when `T` is `ThreadSafe`, and `Shareable` when `T` is `Shareable`.
- The bootstrap implementation bridges these semantics through runtime intrinsics (`chic_rt_rc_new`, `chic_rt_rc_clone`, `chic_rt_rc_drop`, and their `Arc` counterparts) using typed `Value{Const,Mut}Ptr` payload handles. MIR lowering emits the appropriate calls automatically for assignments, clones, drops, and raw conversions, so user code interacts with `Rc<T>`/`Arc<T>` as regular value types without raw pointer casts.
- Both pointers expose borrow helpers that return raw views into the payload (`ArcMutableRef<T>` for
  `Arc<T>::GetMut`, guard types for `Mutex`/`RwLock`). These participate in the runtime borrow
  checker so destructors cannot run while a borrow is live.

### 3.1 Conditional Compilation Directives

Chic source now supports compiler directives inspired by the C#/C family:

```
#if DEBUG && TARGET_OS == "macos"
    public struct DiagnosticSink {}
#elif TARGET_ENV == "musl"
    public struct DiagnosticSink { public static void Emit() {} }
#else
    public struct DiagnosticSink { }
#endif
```

- Directives (`#if`, `#elif`, `#else`, `#endif`) are evaluated by the frontend *before* lexing. Inactive regions are replaced with whitespace so byte offsets and diagnostics remain stable.
- Conditions can combine boolean literals/defines with `&&`, `||`, `!`, parentheses, equality (`==` or a single `=`) and inequality (`!=`) against string literals. Undefined identifiers default to `false`.
- The compiler injects a standard set of defines for every invocation (all keys are available in lower/upper/mixed case):
  - `DEBUG` / `RELEASE` (mutually exclusive) mirror the build profile; `PROFILE` carries the string `debug` or `release`.
  - `TRACE` defaults to `true` in both debug and release builds and can be disabled with `--define TRACE=false`.
  - `TARGET` / `TARGET_TRIPLE` hold the full target triple; `TARGET_ARCH`, `TARGET_OS`, and `TARGET_ENV` expose the split components (e.g., `x86_64`, `macos`, `glibc`).
  - `BACKEND` is `llvm` or `wasm`; `KIND` is `executable`, `static-library`, or `dynamic-library` depending on the requested artifact.
  - `feature_<name>` booleans are synthesised from `--define feature=a,b,c` (non-alphanumeric characters in feature names are normalised to `_`).
- Additional defines can be supplied on the CLI via `chic build/run/test/check ... --define KEY` (sets `KEY` to `true`) or `--define KEY=value` (string or boolean literal). CLI-supplied values override the defaults; when only `DEBUG` or `RELEASE` is overridden, the complementary flag flips automatically. The same map drives both textual `#if` stripping and structural `@cfg(...)` filtering later in the pipeline.

#### 3.1.1 Conditional attributes on methods

- A builtin attribute `@conditional("SYMBOL")` can be attached to void-returning functions and methods. The argument must be a single, non-empty string literal naming a conditional define.
- When a call targets a `@conditional("X")` function and `X` is not defined (or explicitly `false`), the compiler removes the call during MIR lowering. Argument expressions are not evaluated, and the surrounding sequencing is preserved.
- Applying `@conditional` to a non-void function yields a deterministic lowering diagnostic (**MIRL0330**), and the call remains intact.
- MIR debug dumps include a note whenever a conditional call is elided so tooling can trace the optimisation.

### 3.2 Borrow Lifetimes

Chic borrows are second-class values used exclusively to express argument passing. The qualifiers may decorate parameters (including implicit receivers) but never appear on fields, properties, locals, or return types.

- `in` → shared, read-only borrow. Multiple `in` borrows may exist simultaneously as long as no `ref` or `out` borrow targets the same storage.
- `ref` → unique, mutable borrow. At most one live `ref` may exist for any storage location.
- `out` → write-only borrow. Callees must assign a value before returning; the borrow then releases automatically.

Qualifiers appearing on locals, fields, patterns, or return types are rejected during parsing. Code that stores or re-shares a borrow triggers **CL0031 — borrow escapes scope**, including:

- returning a borrowed parameter or receiver,
- storing a borrowed parameter into a field, local, or capture environment, and
- capturing a borrowed parameter inside a closure, lambda, or async state machine.

MIR lowering introduces `BorrowRead`/`BorrowWrite` statements only around the call frame that consumes the borrow and tears them down automatically once the invocation ends. No `Assign` statements hold borrow operands, ensuring borrows remain call-scoped temporaries.

Return values always transfer or copy ownership; the compiler never manufactures hidden borrows on behalf of a callee.

Every borrow lifetime equals the invocation frame. Attempts to return, store, capture (in a closure), or otherwise extend a borrow beyond the call produce diagnostic **CL0031 — borrow escapes scope**, citing the offending expression. MIR emits `BorrowRead`/`BorrowWrite` for the lifetime of the call and automatically tears them down when the frame ends.

### 3.4 Region Profiles

`region name { ... }` introduces a lexical arena. Entering the block calls `Std.Memory.Region.Enter("name")`, binds the resulting `RegionHandle` to `name`, and schedules a teardown drop at the end of the scope (including early returns or unwinding). All allocations obtained via `Region.Alloc{,Zeroed}` or region-backed containers free en masse when the block exits; teardown is idempotent.

- Region helpers: `Region.Alloc<T>(handle, count)`, `Region.AllocZeroed<T>(...)`, `Region.Span<T>(handle, len)`.
- Containers: `Vec.NewIn<T>(handle)` and `Vec.WithCapacityIn<T>(..., handle)` route all allocations through the region allocator; operations after teardown return `AllocationFailed`.
- Telemetry: `Region.Telemetry(handle)` exposes allocation/zeroed/free counts; `Region.ResetTelemetry(handle)` clears counters inside hot loops for profiling.
- Profile hashes use 64-bit FNV-1a over the UTF-8 bytes of the region name, matching the native runtime’s `RegionHandle.profile` field.

Arenas are labelled by profile hashes (FNV-1a of the identifier or supplied profile string) so tooling can aggregate and compare per-region metrics.

### 3.5 Concurrency Memory Model

Chic adopts a C11-style, data-race–free memory model backed by RAII synchronisation primitives and borrow-checked auto traits. All atomic operations are expressed through the `Std.Sync` namespace and share a consistent set of ordering rules across LLVM and WASM backends.

#### 3.5.1 Memory Order Enumeration

`Std.Sync::MemoryOrder` encodes the five fundamental orderings:

| Variant | Semantics | Typical Use |
| ------- | --------- | ----------- |
| `Relaxed` | Performs the operation without introducing a happens-before edge. | Counters that only require atomicity. |
| `Acquire` | Prevents subsequent loads/stores from being reordered before the atomic load. | Reading flags written by another thread. |
| `Release` | Prevents prior loads/stores from being reordered after the atomic store. | Publishing data guarded by a flag. |
| `AcqRel` | Combines acquire on load with release on store. | Read-modify-write sequences that publish data. |
| `SeqCst` | Provides a single total order for all participating operations. | Global coordination, default ordering. |

Unless an ordering is supplied explicitly, Chic methods and helper sugar default to `SeqCst`.
Compare-exchange operations accept a single ordering hint; the runtime chooses an appropriate failure
ordering that is no stronger than the requested success ordering.

#### 3.5.2 `Std.Sync` Atomic APIs

Chic exposes `Std.Sync.Atomic*` cells for common integral primitives; these map to runtime
intrinsics on native backends and fall back to simple compare/store when an operation is missing:

| Type | Element | Supported Operations |
| ---- | ------- | -------------------- |
| `AtomicBool` | `bool` | `Load`, `Store`, `CompareExchange` |
| `AtomicI32`, `AtomicI64` | signed integers | Boolean ops plus `FetchAdd/Sub` |
| `AtomicU32`, `AtomicU64` | unsigned integers | Same as signed variants |
| `AtomicUsize` | `usize` | `Load`, `Store`, `FetchAdd/Sub` |

Each struct is `@repr(c)` and contains a single primitive field, ensuring predictable layout for MIR
and backend lowering. On native targets the standard library forwards these calls to the runtime exports
(`chic_rt_atomic_*`); on platforms without native atomics the library uses a best-effort
compare/replace sequence. For cross-thread synchronisation prefer `Mutex`/`RwLock`/`CondVar` when
running on targets that do not guarantee hardware atomics.

```chic
import Std.Sync;

public struct Flag
{
    private AtomicBool _ready;

    public bool WasSignalled() => _ready.Load(MemoryOrder.Acquire);

    public void Signal()
    {
        _ready.Store(true, MemoryOrder.Release);
    }
}
```

- Loads and stores accept overloads that default to `SeqCst`, while read-modify-write operations require the caller to spell out the desired ordering.
- `CompareExchange(order)` uses a single ordering hint; the runtime maps unsupported failure orderings (e.g. `Release`) down to the nearest valid choice (`Relaxed` or `Acquire`) while preserving the requested success semantics.
- All atomic APIs are available to the constant evaluator but will raise **MM0001** when executed in a pure `constexpr` context, reflecting the fact that the runtime semantics require threads.

Fences are exposed through `Std.Sync.Fences.Fence(order)`, which the compiler lowers directly to fence instructions with the supplied scope.

#### 3.5.3 Atomic Blocks

`atomic` blocks provide expression-oriented sugar for common critical sections:

```
atomic { counter += 1; }

atomic(Std.Sync.MemoryOrder.AcqRel)
{
    let old = counter.FetchAdd(1, Std.Sync.MemoryOrder.AcqRel);
    log.WriteLine($"previous value {old}");
}
```

- `atomic { ... }` desugars to a `SeqCst` block. `atomic(ordering) { ... }` reuses the supplied `MemoryOrder` and applies it to implicit fences once lowerings are implemented (1.59.3+).
- Blocks introduce no implicit scoping beyond the contained statements; they exist to make intent explicit and to allow the compiler to synthesise the correct fence/atomic instructions during MIR lowering.
- Nested blocks are permitted. The innermost annotation wins when chaining `atomic` and `unchecked`/`checked` constructs.

#### 3.5.4 Synchronisation Primitives

`Std.Sync` layers higher-level coordination helpers on top of atomics:

| Primitive | Purpose | Notes |
| --------- | ------- | ----- |
| `Lock` | Non-reentrant mutual exclusion without carrying data. | `Enter`/`TryEnter` return `LockGuard`, which releases the lock on `dispose`; raw `EnterRaw`/`ExitRaw` helpers allow pairing with condition variables. Guards are thread-confined and cannot be awaited across. |
| `Mutex<T>` | Mutual exclusion gate that yields `MutexGuard<T>` for mutable access. | Guards unlock automatically on drop; `Lock`/`TryLock` defer to the runtime so waiting threads park instead of spinning. Guards and the mutex both satisfy `ThreadSafe`/`Shareable`. |
| `RwLock<T>` | Many-reader/single-writer coordination. | Read guards provide snapshot access, write guards behave like `MutexGuard<T>`. Non-blocking probes are exposed as `TryRead`/`TryWrite`; the runtime serialises writers until all readers drain. |
| `Condvar` | Cooperative wait/notify paired with a `Mutex`. | `Wait` hands the guard to the runtime: it releases the mutex, parks the caller, and reacquires the mutex before returning so the caller can resume with a fresh guard. |
| `Once` | One-time initialisation primitive. | Combine `TryBegin`/`Complete` manually or call `Once.Call` with a `OnceCallback`. The runtime serialises the first caller and blocks subsequent callers on `Wait` until initialisation completes. |

The primitives are backed by Chic runtime handles on native targets (and stubbed on WASM, returning `ThreadStatus::NotSupported`). `Lock`/`Mutex`/`RwLock`/`Condvar`/`Once` call into `chic_rt_{lock,mutex,rwlock,condvar,once}_*`; handle size/alignment are validated before dispatch, and misuse (null handles, invalid guards) raises `InvalidOperationException`. Guards drop deterministically and release the underlying runtime handles; busy-wait spin loops are avoided in favour of OS/executor-aware parking.

#### 3.5.5 Litmus Verification

Executable litmus tests ship under `tests/concurrency/litmus` to keep the spec, compiler, and
runtime aligned:

- `StoreBufferingRejectsZeroZero` and `LoadBufferingRejectsOneOne` spawn two actors apiece and panic
  if the forbidden `(0, 0)` or `(1, 1)` pairs ever manifest under Acquire/Release ordering.
- `IriwRejectsInconsistentReads` launches two writers and two readers, ensuring no backend can
  surface mismatched `(1,0)` vs `(0,1)` observations.
- `MessagePassingTransfersPublishedValue` validates the release-flag / acquire-load idiom for data
  transfer.

Each testcase uses real `Std.Thread` actors, synchronises them with a shared start gate, and records
outcomes through `Std.Sync::Atomic*` cells. The Rust harness at `tests/concurrency/litmus/mod.rs`
compiles the Chic sources via `CompilerDriver::run_tests` and executes them on both LLVM and WASM
backends; CI treats any failing testcase as a spec violation.

### 3.6 Async Frames & Promotion Controls

Async lowering records frame layouts and enforces promotion policies during MIR lowering:

- `@stack_only` enforces stack residency for the async frame. The compiler errors with **AS0001** when non-argument locals are captured across suspension points or when the computed frame size exceeds the stack budget (default 8 KiB, or the explicit `@frame_limit` budget when present).
- `@frame_limit(bytes)` bounds the async frame size and reports **AS0002** if the computed layout exceeds the budget; in the absence of an attribute, the compiler emits a warning when frames exceed 64 KiB.
- `@no_capture` rejects captured locals across suspension points (**AS0003**); the `@no_capture(move)` form allows by-value captures but rejects `ref`/`out` bindings that would keep borrowed state alive.
- Misapplied or malformed async attributes trigger **AS0004** (e.g., attributes placed on non-async functions/testcases, duplicate attributes, or an invalid `@frame_limit` payload).
- Runtime `Task::scope`/`spawn_local` implementations poll inline with a small budget before enqueuing futures, keeping short-lived frames on the caller’s stack and avoiding heap promotion when possible.
- Diagnostics surface the source span of the attribute or captured local; `CHIC_DEBUG_ASYNC_PROMOTION=1` or `CHIC_WARN_ASYNC_PROMOTION=1` enable verbose logging of the analysed frame size, capture set, and promotion decisions.
- Runtime/stdlib fast paths: `Task::SpawnLocal`/`Task<T>::SpawnLocal` poll a future once on the current thread before scheduling it, and `Task::Scope`/`Task<T>::Scope` inline-complete ready futures while deferring to the executor when work remains.

Pinning semantics are unchanged: pinned locals are never moved between suspension points, and the borrow checker still enforces pin requirements for references that cross an `await`.

## 4. Generics & Constraints

### 4.1 Const Generics

Const generics are available on every nominal item (structs, classes, enums, unions, interfaces, and functions). They make compile-time values part of a type’s identity so array extents, protocol sizes, or SIMD lanes can be enforced without runtime bookkeeping.

#### Declaring Const Parameters

```
public struct Matrix<const ROWS:int, const COLS:int, T>
    where ROWS : const(ROWS > 0)
    where COLS : const(COLS > 0)
{
    private T[ROWS * COLS] _data;
}
```

- `const` parameters appear in the same list as type parameters and must specify a scalar type (`int`, `uint`, `bool`, `char`, or other primitive numerics). Additional scalar kinds (enums, pointers) will be surfaced as the const-eval engine expands.
- Parameters follow normal visibility/ordering rules. Chic lints favour ALL_CAPS names for consts to keep them visually distinct from type parameters, but this is stylistic rather than enforced.
- Const parameters participate in overload resolution and type identity. `Matrix<4, 4, float>` and `Matrix<4, 8, float>` are distinct types with separate metadata and drop glue.

#### Constraints & Evaluation

- `where` clauses can attach `const(...)` predicates to const parameters. The predicate body is parsed as a Chic expression, evaluated with the const arguments in scope, and must yield `bool`. Failing predicates raise **GENERIC_CONSTRAINT_VIOLATION**; malformed expressions surface **CONST_EVAL_FAILURE** with the const-eval diagnostic text.
- Argument expressions are parsed just like regular expressions but are evaluated at type-check time. All arithmetic uses checked semantics: overflow, division by zero, or unknown values produce **CONST_EVAL_FAILURE** tied to the argument span.
- Const expressions appearing in patterns (`case const(N + 1):`) share the same evaluation pipeline and therefore reuse the diagnostics and caching infrastructure.

#### Usage Sites

- Anywhere a generic argument list is accepted, const slots can interleave with type slots: `Buffer<int, 64>`, `Window<const WIDTH:int, const HEIGHT:int, Pixel>`, etc. Syntax mirrors type arguments—no extra keywords are required at the call site.
- Patterns can capture const arguments for matching and code generation (`switch (len) { case const(16): ... }`).
- Const arguments can reappear inside other type expressions (e.g., `Span<T, N>` where `N` flows into an array suffix `T[N]`), ensuring the canonical name of the instantiated type embeds the evaluated const values.

#### Backend & Metadata Guarantees

- MIR `NamedTy` records const arguments as either `Type` or `Const` variants. This information flows into symbol mangling, drop-glue synthesis, and layout hashing so two instantiations that only differ by const value get unique symbols and metadata entries.
- Type layouts store const-augmented canonical names, enabling the monomorphiser and `drop_glue` generator to discover every concrete instantiation that requires codegen.
- LLVM/WASM backends receive the const-enriched canonical names via the signature map; sanitised symbols (e.g., `Smoke__Buffer__N64`) remain deterministic because the const values are normalised during type checking.

See [docs/compiler/const_generics.md](docs/compiler/const_generics.md) for authoring guidance, diagnostics, and lint expectations.

### 4.2 Traits & Generic Associated Types

Traits generalise interfaces while preserving zero-cost abstraction. Chic adopts Rust-inspired syntax with deterministic resolution rules documented in [docs/compiler/traits.md](docs/compiler/traits.md).

```chic
public trait IntoIter<T>
{
    type Iter<TSelf>;
    TSelf::Iter Into(ref this);
}

public impl<T, const N:int> IntoIter<T> for VecN<T, N>
{
    type Iter<VecN<T, N>> = VecNIterator<T, N>;
    VecNIterator<T, N> Into(ref this) { /* ... */ }
}
```

Traits also enable blanket implementations:

```chic
public trait Hashable { fn int GetHash(in this); }
public trait Equatable { fn bool Equals(in this, in this); }

public impl<T: Hashable> Equatable for Vec<T>
{
    bool Equals(in this Vec<T> lhs, in Vec<T> rhs)
    {
        if (lhs.Len() != rhs.Len()) return false;
        for (int i = 0; i < lhs.Len(); i++)
        {
            if (!lhs[i].Equals(rhs[i])) return false;
        }
        return true;
    }
}
```

- **Surface syntax** – `trait` items may declare methods, associated types (`type Item<'scope>;`), and (future) associated constants. `impl` items mirror the trait syntax; blanket impls require that either the trait or the self-type is defined in the current crate so coherence remains checkable at definition time.
- **Deterministic Solver** – Every trait obligation is canonicalised (type/const variables replaced with de Bruijn indices) before resolution. The solver first checks the ambient environment (e.g., `where T: Trait` bounds), then consults impl tables. Overlaps, cycles, or orphan violations yield diagnostics **TCK090–TCK095**. There is no specialization or negative reasoning.
- **Cycle/blanket enforcement** – Super-trait graphs must remain acyclic; declaring `trait A : B` and `trait B : A` yields **TCK090 TRAIT_CYCLE_DETECTED**. Blanket impls such as `impl<T> Trait for Vec<T>` (which would require negative reasoning to stay coherent) are rejected with **TCK095 TRAIT_IMPL_SPECIALIZATION_FORBIDDEN** until the deterministic solver grows specialization support.
- **Blanket Implementations** – `impl<T: Hashable> Equatable for Vec<T>` is permitted if the trait or the self-type is defined in the current crate. Overlaps are rejected at registration time, ensuring downstream crates cannot introduce conflicting blanket impls.
- **Associated Types & GATs** – Traits may declare associated types with their own generic arguments. Projections (`MyIter::Item<'ctx>`) are resolved by the solver, which expands the selected impl’s associated-type body. Projection cycles emit **TCK096 TRAIT_ASSOC_CYCLE**.
- **Trait Objects (`dyn Trait`)** – Traits that satisfy object-safety rules (no `Self` as a return type, all associated types have defaults, no generic methods) can be used behind `dyn`. Type checking records any violations as **TCK097 TRAIT_OBJECT_UNSAFE**. Lowering materialises a concrete layout `{ ptr data_ptr, ptr vtable_ptr }` and records one vtable per `(trait, impl)` pairing (`__vtable_{Trait}__{Impl}` sanitised symbol). LLVM emits the vtable as an `[N x ptr]` constant and calls indirect through the loaded slot; WASM writes the table into linear memory (4‑byte function indices) and issues `call_indirect` with the matching signature index. Both backends reuse the same MIR metadata (`TraitVTable`, `TraitObjectDispatch`).
- **Runtime/Stdlib glue** – The MIR interpreter/test executor now reuses the same vtable metadata so `chic test` behaves identically to backend builds. `Std.Traits.Debug` exposes the initial standard-library namespace for trait objects, keeping diagnostics/code samples stable while we grow richer helpers.
- **Diagnostics** – Trait-related errors share a new set of codes (**TCK090–TCK098**) covering cycles, orphan violations, missing impls, ambiguous resolution, specialization attempts, member mismatches, and object-safety issues. Tooling can surface these with actionable guidance.
- **Backend Contracts** – Static dispatch monomorphises selected impls into direct calls. Trait objects emit vtable layouts during lowering and indirect calls in LLVM/WASM. `.clrlib` metadata records `(Trait, Impl)` pairs for downstream tools.

#### Object-Safety Rules

To participate in `dyn Trait`, every method and associated item must satisfy:

1. Methods may not return `Self`, contain unconstrained type parameters, or use `where Self: Sized` clauses (those belong on inherent impls). Violations surface as **TCK097**.
2. Associated types must provide defaults (optionally referencing trait parameters) so trait objects have a concrete projection. Missing defaults emit **TCK098** when the type checker builds the trait metadata.
3. `ref this`/`in this` receivers are allowed; `out this` is rejected to keep object-safe borrows consistent across backends. Async trait methods/impls are supported and must return `Std.Async.Task` or `Std.Async.Task<T>` so vtable entries can be driven by the executor.

These rules mirror the runtime layout guarantees described above: we always know the receiver layout (`{ data_ptr, vtable_ptr }`), the slot ordering inside each vtable, and the function signature recorded for every slot. See `docs/guides/traits.md` for a short trait overview.

See [docs/compiler/traits.md](docs/compiler/traits.md) for solver internals, vtable layout, and benchmarking requirements.

### 4.3 Effect Typing

Chic exposes side effects explicitly so tooling and autonomous agents can reason about program behaviour without hidden runtime state. Functions may declare:

- A `throws` clause that enumerates the exception *types* that can escape.
- An `effects(..)` clause that enumerates named *effect capabilities* checked by the type system.

Both clauses participate in type checking, generic instantiation, and overload resolution. Callers must satisfy (`effects`) or handle (`throws`) every capability surfaced by the callee, and higher-order functions propagate effect requirements through their function parameters.

```chic
public effects(random, cancel)
async fn RunEpisode(env: ref Environment, rng: ref RNG, budget: Duration)
    throws TimeoutError
{
    cancel_scope(budget) {
        let policy = ??Policy;              // obligation: supply a policy
        let (state, child_rng) = env.Reset(rng);
        return policy.Sample(state, child_rng); // propagates `random`
    } // cancel_scope discharges the `cancel` effect
}
```

Effects are tracked per function item and per MIR body. The canonical set is stored in MIR metadata, forwarded to backends, and emitted in `mir.json` for machine consumption (see §16.7).

#### Declaring and Satisfying Effects

- `effects(...)` accepts one or more identifiers. The built-in effect names are `random`, `measure`, `network`, and `cancel`. Future library-defined effects may appear once the trait-based extension mechanism lands.
- Effect declarations appear between the parameter list and the return type (before `throws`, if present). They can also decorate lambda expressions and trait method signatures.
- When calling a function, the caller’s effect set must be a superset of the callee’s set. Async lowering threads effect requirements through await points so suspended frames record the ambient capabilities.
- `using` imports do not change effect semantics; effects compose lexically just like generic bounds.

#### Exception Effects (`throws`)

Functions declare the set of exceptions they may propagate with a `throws` clause. Each listed type must be an `error` class (or derive from one), and the clause becomes part of the function’s callable surface area:

```chic
public error IoError { }
public error FormatError { }

public Config Load(str path) throws IoError, FormatError
{
    let data = ReadFile(path)?;      // propagates `IoError`
    return ParseConfig(data)?;       // propagates `FormatError`
}
```

The type checker enforces exception safety:

- Every `throw` or effectful `?` must be covered by a declared effect. Missing entries emit **TCK100**.
- Re-throwing from a `catch` is permitted, but only inside handlers that were already guarding the effect.
- Functions without a `throws` clause may not allow exceptions to escape.

`throw` expressions require a non-null `error` instance. Structured exception handling (`try`/`catch`/`finally`) interacts with the effect system as before: a handled exception is removed from the surrounding effect set, while unhandled branches must re-`throw` or be declared. The `?` operator desugars into `throw` for the `Err` arm, so the exception payload must derive from `Exception`.

#### Fallible Types & Diagnostics

Lowering tags every type that represents a “fallible result” with `TypeFlags::FALLIBLE` so MIR, optimisation passes, and runtime metadata can uniformly recognise values that must be handled:

| Type | Notes |
| --- | --- |
| `Exception` and any derived `error class` | Includes user-defined classes whose names end with `Exception`. |
| `Exception?` | Nullable wrappers created by `T?` when `T : Exception`. |
| `Std::Result<T, E>` | All instantiations of the standard `Result` container. |
| `@fallible` aggregates | User-defined structs/classes/enums/unions annotated with `@fallible`. |

`SynthesisedTypeMetadata` and the `@__chic_type_metadata` / `chic.type.metadata` tables now include the flag bit, so tools can surface “must handle” hints without re-running the type checker. MIR lowering inserts explicit `MarkFallibleHandled` sentinels ahead of sinks (`?`, `throw`, explicit `let _ =`, exhaustive pattern matches, try/catch exception locals), and the dedicated `mir::passes::fallible_drop` analysis performs forward dataflow to emit two diagnostics:

- **EH0001** — warning when a fallible temporary drops immediately (e.g., ignoring a `Result` return). The warning points at the drop span and notes where the value was produced.
- **EH0002** — error when any control-flow exit (`return`, implicit fallthrough, async resume/drop paths, unwinds) can leave a scope while a fallible value remains live.

Both diagnostics flow through the normal CLI report, ensuring new APIs wire up explicit handling before landing.

#### Randomness (`effects(random)`)

Stochastic APIs thread an explicit `RNG` handle and declare `effects(random)` so every random draw is visible to the compiler:

```chic
public effects(random)
fn dropout(x: in Tensor<f32,[B,H],RowMajor,Gpu<0>>, p: f32, rng: inout ref RNG)
    -> Tensor<f32,[B,H],RowMajor,Gpu<0>>;
```

- `RNG` is a splittable counter-based generator (`split` returns deterministically partitioned streams). Splitting and advancing the generator do not allocate.
- MIR introduces `Rand` and `SplitRng` ops (§16.4). Each site annotates its source span so replay tooling can deterministically reproduce sequences.
- The effect checker ensures RNG handles never flow into non-`random` contexts without first discharging or splitting the stream. Async tasks capture the RNG by value to guarantee deterministic replay.

#### Measure Effects (`effects(measure)`)

Probabilistic programming libraries rely on `sample` and `observe` primitives. Chic treats `measure` as a sibling to `random`:

- `Dist<T>::sample` requires both `effects(random)` and `effects(measure)` if it records log-probabilities.
- `observe(dist, value)` lowers to a `Observe` MIR op that contributes to the active trace (§16.7). The effect ensures callers account for the accumulated log-likelihood (e.g., to hand it to an inference engine).
- Functions that only *read* accumulated log-probability do not require the `measure` effect; only producers need to declare it.

#### Network Effects (`effects(network)`)

Distributed collectives and actor messaging require explicit capability declarations:

- Collectives (`allreduce`, `broadcast`, `pshard`) lower to MIR `Collective` ops (§16.8) and demand `effects(network)`.
- Actor interfaces mark every entry point with `effects(network)`. The runtime performs deterministic serialization; no ambient sockets or hidden globals exist.
- Effects prevent synchronous APIs from being called in isolated or deterministic-only environments without acknowledging the network requirement.

#### Cancellation Effects (`effects(cancel)`)

Structured concurrency primitives surface cancellation budgets through the `cancel` effect:

- `cancel_scope { ... }` introduces a region where spawned tasks inherit the same cancel token. The enclosing function must declare `effects(cancel)` unless the scope is immediately discharged (`cancel_scope(async move { ... }).await`).
- `await_any`, `on_timeout`, and `cancel_rest` lower to MIR primitives (§16.10) that require the effect so tooling can insert required handlers (e.g., to tidy resources on cancellation).
- Pure functions remain effect-free; only scopes that *initiate* cancellation need to declare it. Callees that accept a `CancelToken` parameter but never trigger it do not need the effect.

#### Metadata and Tooling

- MIR records effect sets per function, closure, and graph (§16.7). The data feeds `mir.json`, diagnostics, and the schedule profiler so agents can reason about required capabilities.
- Backends do not erase effect information. Deterministic tracing (§16.14) cross-links tracepoints to their effect requirements.
- `chic explain --why` emits effect provenance graphs, showing where capabilities enter a call chain and how they are discharged.

### 4.4 Unsafe Contract

Unsafe Chic code follows a precise aliasing contract so the optimiser, borrow checker, and
runtime agree on what may alias and when memory becomes live:

- Borrowed parameters (`in`, `ref`, `out`) are emitted as `noalias + nocapture` pointers.
  `out` parameters start life uninitialised and the compiler enforces “write before read”
  semantics.
- Raw pointers accept inline qualifiers. The outermost pointer in `*mut @restrict @aligned(32)
  @readonly Foo` is treated as:
  - `@restrict` / `@noalias`: unique for the duration of the call.
  - `@readonly`: loads only (stores are diagnosed).
  - `@aligned(N)`: minimum alignment that codegen may rely upon.
  - `@expose_address`: this pointer may be cast to integers or have its provenance erased.
- Pointer/int casts require both `unsafe` *and* `@expose_address`. Casts without the qualifier
  now emit `DM0150` so provenance-sensitive code cannot accidentally leak addresses.
- `Std.Numeric.UIntPtr` provides the sanctioned pointer↔integer helpers. The `FromPointer<T>` /
  `FromConstPointer<T>` + `.AsPointer<T>` / `.AsConstPointer<T>` families keep handles opaque while
  `AddressOf<T>` / `AddressOfConst<T>` expose raw `nuint` addresses. Their signatures enforce
  `@expose_address`, so provenance-erasing casts are rejected during type checking. Regression coverage
  lives in `tests/numeric_structs.rs` and `mir::builder::tests::unsafe_pointers.rs`.
- The standard memory intrinsics opt into the contract. For example
  `Std.Memory.GlobalAllocator.Copy` takes `*mut @restrict byte dest` /
  `*const @restrict byte src`, and `Vec`’s push/pop shims re-use the same guarantees when
  writing elements into inline storage.
- Unsafe pointer parameters without qualifiers now raise a diagnostic (`DM0211`) so the aliasing
  contract is documented at the type level. Backends use that information to attach LLVM alias
  scopes and to publish the `chx.alias.contracts` Wasm custom section.

#### `MaybeUninit<T>`

`Std.Memory.MaybeUninit<T>` is the canonical way to work with partially initialised storage:

```chic
var slot = MaybeUninit<int>.Uninit();
slot.Write(42);         // initialises in place
var value = slot.AssumeInit(); // moves the value out
```

- Instances track whether the payload is initialised; double writes or double reads surface as
  `Std.InvalidOperationException`.
- `dispose(ref this)` only drops the payload when it is live, allowing `MaybeUninit<T>` to live
  on the stack without caller-supplied bookkeeping.
- Helpers such as `AssumeInitRead`, `AssumeInitRef`, `ForgetInit`, and
  `MarkInitialized` underpin placement patterns.

`Vec` now exposes placement-aware helpers that compose with `MaybeUninit<T>`:

- `Vec.PushInitialized(ref MaybeUninit<T>)` moves a user-initialised slot into the vector
  without an intermediate copy.
- `Vec.PopInto(ref MaybeUninit<T>)` writes directly into an uninitialised slot.
- `Vec.InsertInitialized`, `Vec.RemoveInto`, and `Vec.SwapRemoveInto` provide the same guarantees
  at arbitrary positions, allowing builder-style code to avoid redundant copies.
- Ergonomic wrappers (`Push<T>`, `Pop<T>`) continue to exist for value-oriented code, but
  they are implemented on top of the placement primitives.
- `MaybeUninit<T>.AsValueConstPtr()` / `AsValueMutPtr()` expose the shared
  `Std.Runtime.Collections.Value{Const,Mut}Ptr` structs (pointer + `usize` size + alignment), so
  placement helpers and stackalloc lowering pass a single typed handle to the runtime rather than
  juggling raw `byte*` arguments and duplicated metadata.

#### Zero-Initialisation Intrinsics

- Chic now mirrors C#’s [`Unsafe.InitBlock`](https://learn.microsoft.com/en-us/dotnet/api/system.runtime.compilerservices.unsafe.initblock) and Rust’s [`ptr::write_bytes`](https://doc.rust-lang.org/core/ptr/fn.write_bytes.html): zeroing memory is a first-class intrinsic the compiler understands, not an ad-hoc pointer cast.
- `Std.Memory.Intrinsics.ZeroInit<T>(out T target)` is an intrinsic method that requires `unsafe`. The MIR builder emits `StatementKind::ZeroInit { place: Place }`, recording the destination, its layout size, and alignment via `TypeMetadata`. Borrow checking treats the statement as a write, so `out` parameters become initialised immediately without ever taking their address.
- For raw pointer scenarios (inline vector storage, external buffers) the intrinsic `Std.Memory.Intrinsics.ZeroInitRaw(*mut byte destination, usize length)` emits `StatementKind::ZeroInitRaw { pointer: Operand, length: Operand }`. Callers must guarantee the region is writable; the intrinsic simply fills `length` bytes at `destination` with `0x00`.
- LLVM lowers both statements to `llvm.memset` (24/32/64-bit) whenever the size/alignment are constants; otherwise it calls the runtime helper `chic_rt_zero_init(ptr dest, usize len)`. The Wasm backend emits bulk-memory instructions or falls back to the same runtime hook when the target lacks bulk-memory support.
- Runtime support mirrors the allocator shims: `chic_rt_zero_init` lives alongside `chic_rt_memset` so bootstrap builds and host integrations use a single ABI.
- Standard library code (`Std.Memory.InitializeDefault`, `Std.Memory.MaybeUninit<T>`, `Std.Collections.Vec`, `Std.Span`) should use these intrinsics; pointer/int casts used solely for zeroing are rejected.
- The `Std.Memory` module ships stub definitions for `Intrinsics.ZeroInit`/`ZeroInitRaw`. The compiler always rewrites the calls to MIR statements; the stubs exist only so bootstrap builds type-check and will throw if the lowering phase is bypassed.
- `Std.Memory.GlobalAllocator.InitializeDefault`, `Std.Memory.MaybeUninit<T>.Uninit`, and the placement helpers exposed by `Std.Collections.Vec`/`Vec.PushInitialized`, `Vec.PopInto`, `Span<T>.StackAllocCopy`, etc. route through the new intrinsics. Manual overloads that accept element metadata now take `usize` sizes/alignments so they can be forwarded directly to the runtime without lossy casts.

The runtime emits `noalias`/`nocapture` metadata in the LLVM backend, forwards alignment hints,
A debug-only assertion guards `chic_rt_memcpy` against overlapping reserves, and
the runtime refuses integer casts that would violate provenance. The
`docs/guides/unsafe_contract.md`
guide includes end-to-end examples and diagnostics for unsafe code. ABI details for the new runtime hook live in
`docs/runtime/abi.md#memory-intrinsics`.

### 4.5 Variance & Auto-Trait Qualifications

Variance and auto traits jointly determine when Chic values may cross
`await`, thread, and generic boundaries:

- `in` borrows (and the `ref readonly T` type constructor) are covariant: the
  borrow checker and the async state machine treat them as read-only and only
  require the `@shareable` auto trait when a value survives across a suspension
  point. Mutable borrows (`ref`/`out`) remain invariant and therefore always
  require `@thread_safe`. The MIR builder emits the correct trait requirement
  by inspecting both the parameter binding (`in`/`ref`/`out`) and the inferred
  `ref` type (see `mir::builder::body_builder::async_support::tests::*`).
- Interfaces and delegates continue to declare variance using `in` / `out`.
  The type checker enforces that members respect the declared variance and the
  parser emits `TCK022` when variance modifiers appear outside those contexts
  (`frontend::parser::tests::grammar::generics::*`).
- Generic parameters can require auto traits directly in their constraint list:
  `where T : @thread_safe, @shareable`. The parser records these modifiers and
  reflection descriptors emit the `@thread_safe`/`@shareable` strings alongside
  the existing `struct`/`class`/`new()` constraints.
- Traits may also be annotated with `@thread_safe` / `@shareable`. Every
  `impl Trait for Type` inherits the declaration’s requirements; the trait
  solver inserts a generic auto-trait constraint and rejects implementations
  whose target type cannot prove the corresponding trait. This behaviour is
  covered by `typeck::arena::tests::diagnostics::trait_impl_requires_thread_safe_when_trait_marked_thread_safe`.
- Instantiating a generic type now checks auto-trait requirements eagerly. A
  concrete argument that lacks the requested trait triggers `[TCK035]
  AUTO_TRAIT_REQUIRED`, while arguments that the compiler cannot prove to be
  safe trigger `[TCK037] AUTO_TRAIT_UNPROVEN`. When the argument is itself a
  type parameter, the enclosing function or type must declare the matching
  constraint (e.g., `struct Uses<T> where T : @thread_safe`). `typeck::arena::
  tests::diagnostics::auto_trait_constraint_rejects_non_thread_safe_argument`
  and `...::auto_trait_constraint_respected_for_generic_arguments` capture the
  new behaviour end-to-end.
- Async state machines inherit the same logic: `ref readonly` locals only
  require `@shareable`, while mutable refs and owned values require
  `@thread_safe`. The new unit tests in `mir::builder::body_builder::
  async_support::tests` cover both modes.
- When a captured local or async state slot refers to a generic parameter,
  the compiler trusts the enclosing context before falling back to the layout
  table. Functions or types that declare `where T : @thread_safe` (or
  `@shareable`) satisfy the requirement even though the MIR builder cannot
  resolve a concrete layout, keeping async APIs generic without sprinkling
  manual trait assertions. See
  `typeck::arena::tests::diagnostics::auto_trait_constraint_respected_for_async_generic_context`.

These rules replace the previous “Planned” placeholder.

### 4.6 `impl` Trait and Opaque Returns

Opaque types hide concrete implementations while keeping call sites monomorphised. Chic accepts
`impl Trait` in return positions and parameter lists; bounds use the same `+`-separated syntax as
trait objects:

```chic
public impl Iterator<int> MakeCounter()
{
    return new Counter();
}
```

- A function with an opaque return must evaluate to a single concrete type. The compiler inspects
  every `return` (including async/generator lowering) and substitutes the inferred concrete type
  through any wrappers, e.g., `Task<impl T>` becomes `Task<Counter>` before building the async
  state machine. Divergent return types trigger a lowering diagnostic and keep the function
  unresolved.
- Each bound on `impl Trait` produces a solver obligation for the inferred type; missing
  implementations are reported as `[TCK310] IMPL_TRAIT_BOUND_UNSATISFIED`. Object-safety gates
  continue to apply to `dyn Trait` but are skipped for `impl Trait` because codegen is
  monomorphised.
- Async/generator lowering captures the resolved payload, so layout, borrow, and auto-trait checks
  run against the concrete type rather than an opaque placeholder.
- API guidance: prefer `impl Trait` to hide iterator/generator internals while keeping call sites
  optimised, and ensure every return path produces the same concrete type to avoid the opaque-return
  diagnostic.

```chic
public interface IOrder<T> { int Compare(in this T a, in T b); }

public void Sort<T>(ref Span<T> xs) where T : IOrder<T>
{
    // Specialization per T (monomorphization)
}

public void Log(in IShape shape) { print(shape.Area()); } // vtable call
```

- Generic parameters appear on types, methods, extensions, and testcases. Chic follows
  C#-style constraint syntax using trailing `where` clauses. Each clause names a type parameter
  and lists one or more comma-separated requirements. Multiple clauses may be chained:

  ```chic
  public struct Pooled<T, TAllocator>
      where T : struct, notnull
      where TAllocator : IAllocator, new()
  { /* ... */ }
  ```

- Classes and structs share identical parsing rules for generic parameter lists and `where`
  clauses. Tests ensure attributes, access modifiers, and nested namespace declarations behave
  the same for both surfaces so future changes keep them in lockstep.

- Supported constraint kinds:
  - **Type/interface**: `where T : IFoo, Bar` enforces that `T` implements every listed
    interface and inherits the specified base class (exactly one class may appear).
  - **`struct`**: the argument must be a value type. Option and nullable differences are still
    enforced by the definite-assignment pass.
  - **`class`**: the argument must be a reference type.
  - **`notnull`**: forbids nullable instantiations (e.g. `T = string?`).
  - **`new()`**: requires an accessible parameterless constructor. This is checked after base
    and interface constraints resolve. Because Chic monomorphises generics, the constructor
    call is statically dispatched.
- The language deliberately omits C#'s `unmanaged`, `default`, and `enum` constraints; the
  runtime has no managed/unmanaged dichotomy, and future revisions will revisit the remaining
  forms if the type system demands them.
- Constraints are validated at instantiation time. Violations produce type-checker diagnostics
  that cite the offending `where` clause. Satisfied constraints feed MIR lowering so backends can
  rely on value semantics (e.g. emitting stack allocations for `struct`-constrained generics).
- Generics are monomorphized; no runtime reification.
- Dynamic polymorphism remains available through interfaces.

## 5. `no_std` / OS Development

- **Crate attributes:** `#![no_std]` and `#![std]` are crate-level and mutually exclusive; they must appear before any items. `#![no_std]` disables the implicit `std` dependency and relaxes the `Main` requirement for executables, while `#![std]` makes the dependency explicit. Namespace-scoped `@no_std`/`@nostd` attributes are not supported. `#![no_main]` disables the implicit `Main` requirement and suppresses startup descriptor emission so custom runtimes/bootloaders can supply their own entry symbols (`start`, exported host callbacks, etc.); see `docs/guides/no_main.md` for usage guidance.
- **Library tiers & loading:** `core` always loads (primitives such as `Option`, `Result`, `Span<T>`, `Copy`/`Drop`). Standard builds also load `alloc`/`foundation`/`std` when `load_stdlib` is enabled. `#![no_std]` crates link `core` only; `alloc` and the `foundation` crate become available when `CHIC_ENABLE_ALLOC=1` is set at build time, and `std` is never loaded in this mode. Emitted metadata carries `profile=no_std` so backends/linkers can track the selected runtime surface. The `Std` namespace prelude is always in scope; when `std` is skipped the `Std.Platform` surface simply remains unavailable.
- **Core crate behaviour:** `src/core` ships the shared primitives (`Std.Option`, `Std.Result`, `Std.Span<T>`/`Std.Span.ReadOnlySpan<T>`, range helpers, `Std.Copy`/`Std.Drop`) and now backs both `std` and `#![no_std]` builds without pulling platform-dependent wrappers. `Option.None()`/`Result.FromErr` zero inactive payloads via the zero-init intrinsics so `out` parameters stay initialised. `Option.Expect` now throws `Std.InvalidOperationException` when unwrapping a missing value instead of aborting.
- **Foundation crate behaviour:** `src/foundation` exposes no_std-friendly utilities (Vec/Array views, span-based slice helpers, reflection metadata, and trait utilities) under the `Foundation` namespace. It loads whenever `alloc` is enabled (guarded by `CHIC_ENABLE_ALLOC=1`) and is intended to host portable collections while platform-facing APIs live elsewhere.
- **Platform layer:** Platform-integrated wrappers (files, sockets, threads, clocks, async runtime) live under the `Std.Platform` subtree and are skipped for `#![no_std]` crates. The `Std` surface continues to host platform-only namespaces while `Foundation` stays host-agnostic.
- **Allocator hooks:** `Std.Alloc.Hooks.Install` registers an allocator vtable that feeds all runtime allocations (GlobalAllocator, Vec, String, etc.). `CHIC_ENABLE_ALLOC=1` must be present to link `alloc`/`foundation` into `#![no_std]` builds, and an allocator must be installed for the heap to become usable on freestanding targets. Vtables are ABI-stable across backends (see `runtime/include/chic_rt.h`), and telemetry is exposed through `AllocationTelemetry` counters.
- **POSIX/Apple IO wrappers:** `Std.Platform.IO.File` provides safe wrappers over `fopen`/`fread`/`fwrite`/`fflush`/`fclose` for native targets. Paths are UTF-8 encoded with null terminators, `IoError` codes surface partial reads/EOF, and WASM builds import `env.fopen`/`env.fread`/`env.fwrite`/`env.fflush`/`env.fclose`/`env.clock_gettime`/`env.nanosleep`/`env.socket`/`env.connect`/`env.send`/`env.recv`/`env.shutdown`/`env.close`/`env.htons`/`env.inet_pton` via `WasmExecutionOptions.io_hooks` so host runners can provide filesystem/clock/socket shims. These wrappers are only linked when the `std` tier is active.
- **Span invariants:** span/readonly span handles must carry a valid pointer/length/alignment triple. Slicing and copy helpers throw `Std.IndexOutOfRangeException`/`Std.ArgumentException` when out-of-bounds or stride violations occur; zero-length spans always use null data with zeroed metadata for FFI safety.
- **Exception surface:** `throw` is a non-returning control-flow edge and participates in both statement and expression contexts. Standard-library error paths now raise `Std` exceptions (`FormatException`, `OverflowException`, `ArgumentException`/`ArgumentNullException`, `InvalidOperationException`, `IndexOutOfRangeException`) instead of using panic-style startup hooks; catastrophic runtime aborts remain confined to the runtime shims.
- **Targets:** macOS and Linux on x86_64/aarch64 with `chic build --target` selecting the desired triple.
- **Outputs:** `--crate-type exe|lib|dylib` (executables, static libs with `.clrlib` sidecars, shared libs).
- **Incremental builds:** cache MIR summaries, backend options, and toolchain hashes to skip redundant codegen.
- **Cross compilation:** e.g. `chic build kernel.cl --target aarch64-unknown-none` for freestanding builds; use `#![no_std]` plus optional `CHIC_ENABLE_ALLOC=1` when a heap is required.
- **WASM note:** the bootstrap toolchain and in-tree executor target `wasm32-unknown-unknown` (memory32). `nint`/`nuint` map to 32-bit linear-memory indices and all runtime string/handle helpers accept `i32` offsets into linear memory. Memory64 remains deferred. The executor implements the minimal `i64` opcode set (const/eqz/add/sub/mul/div/rem/and/or/xor/shifts/load/store) needed for string interpolation and numeric formatting; wider-than-64-bit interpolation is rejected with a backend diagnostic until 128-bit lowering lands.

### 5.1 SIMD & GPU Profiles (Planned)

Chic already supports multiversioned CPU codegen. Upcoming work unifies SIMD and GPU generics via `simd<T, const LANES:int>` constructs with per-target code selection, enabling a single generic surface across CPU and accelerator backends. Formal syntax, target selection attributes, and lowering rules will be documented here once the implementation lands.

## 6. Macros / Compiler Extensions

- Attribute-driven macros, both declarative and procedural, e.g.:

```chic
@derive(Equatable, Hashable)
public struct Id { public ulong Value; }

@memoize
public static int Fib(int n) { /* … */ }

@extern("C") @link("c")
public static extern isize write(int fd, byte* buf, usize count);
```

- Compile-time function evaluation (CTFE) for const expressions.
- Attribute macro expansion happens before lowering: collection tags macro attributes with an `expandable` flag and raw token stream, expansion walks items in lexical order with pass-scoped hygiene IDs (stamping generated items with the originating attribute span), and staged builtin evaluation replays on the expanded tree. Diagnostics for unknown/unsupported macros are attached to the attribute span, and runaway expansion is capped at 32 passes. See `docs/compiler/attribute_macros.md` for the full pipeline.
- Built-in derives enable common patterns without boilerplate:
  - `@derive(Equatable)` synthesises `op_Equality`/`op_Inequality` pairs for structs and classes by comparing their public fields and properties.
- `@derive(Hashable)` emits a simple `GetHashCode` helper that XORs the hash codes of those members so values can participate in hashed collections.
- `@memoize` is recognised by the expander today and emits a targeted diagnostic placeholder until the runtime cache scaffolding lands.

#### Compile-Time Reflection & Quasiquotes

- Reflection metadata is compile-time generated, immutable, and ABI-aligned. `public` declarations (types and members) always emit descriptors; `internal` items are excluded until a future `reflect_visibility=internal` manifest toggle lands (tracked via issues) to keep today’s output minimal and deterministic. No runtime discovery or dynamic type loading is permitted.
- Surface entry points:
  - `typeof(T)` yields a `Std.Meta.TypeHandle` (full name + stable type id) that can be compared or serialized without allocations.
  - `Std.Meta.Reflection.reflect<T>()` is a CTFE intrinsic that rejects value arguments and returns the canonical `Std.Meta.TypeDescriptor` pointer for `T`. Diagnostics are deterministic for missing metadata or invalid arity, and behaviour is identical on LLVM/WASM. The intrinsic is O(1) and allocation-free.
- Descriptor shapes (C#-like, Chic-safe):
  - `TypeDescriptor` carries `Namespace`/`Name`/`FullName`, `TypeId`, `Kind` (Struct/Class/Record/Enum/Interface/Trait/Union/Delegate/Function/Const/Static/Extension/Impl), `Visibility`, `IsGeneric`, ordered `GenericArguments: DescriptorList<TypeHandle>`, `Bases: DescriptorList<TypeHandle>`, `Members: DescriptorList<MemberDescriptor>`, `Attributes`, optional `UnderlyingType` (enums), and `Layout: TypeLayoutDescriptor` sourced from the same ABI tables used by codegen.
  - `MemberDescriptor` stores `Name`, `Kind` (Field/Property/Method/Constructor/EnumVariant/AssociatedType/ExtensionMethod/etc.), `DeclaringType: TypeHandle`, `Visibility`, and `Attributes`. Specialised records extend it:
    - `FieldDescriptor`: `FieldType`, `IsStatic`, `IsReadonly`, optional `Offset`, optional accessor thunks.
    - `PropertyDescriptor`: `PropertyType`, `HasGetter`/`HasSetter`/`HasInit`, optional `Getter`/`Setter`/`Init` method descriptors, plus the base attributes/visibility envelope.
    - `MethodDescriptor`: `ReturnType`, ordered `Parameters`, `IsStatic`, `IsVirtual`/`IsOverride`/`IsAbstract`, `IsAsync`, and calling-convention flags for extern slots.
    - `ConstructorDescriptor`: ordered `Parameters` plus `IsDesignated`/`IsConvenience`.
    - `ParameterDescriptor`: `Name`, `ParameterType`, `Mode` (`value`/`in`/`ref`/`out`), `HasDefault`/`DefaultValue` (const-only), and `Attributes`.
    - `AttributeDescriptor`: `Name`, positional `Args`, and named `Args` as compile-time constants (no runtime attribute objects are materialised).
  - `TypeLayoutDescriptor` mirrors the computed ABI layout: `Size`, `Align`, and per-field `FieldLayoutDescriptor` entries (`Name`, `Offset`, `Type`, `IsReadonly`).
- Accessors respect the no-GC/borrowing model: generated thunks are optional and use typed or raw-pointer signatures (`fn(void* instance, void* out_value)` / `fn(void* instance, void* in_value)` or typed `ref` accessors). Mutating access requires a mutable/`ref` receiver; readonly/init-only members never receive setters. Metadata-only builds omit thunks rather than boxing values.
- Ordering and determinism: types are sorted by `FullName`, member lists preserve declaration order, and parameters/attributes remain in source order. Descriptor tables carry a schema `version` field; identical inputs produce identical reflection outputs across backends.
- Artefacts: the compiler emits a binary reflection table (`@__chic_reflection` on LLVM, `chic.reflect` custom section on WASM) plus a JSON sidecar `<artifact>.reflect.json` (`version = 2`) co-located with the primary output and embedded into `.clrlib` as `metadata/module.reflect.json`. JSON arrays mirror the descriptor lists with stable ordering and type/attribute/layout data.
  The JSON schema is `{ "version": 2, "types": [TypeDescriptor...] }` with fields mirroring the in-memory descriptors (`type_id`, `namespace`, `name`, `full_name`, `kind`, `visibility`, `is_generic`, `generic_arguments`, `bases`, `attributes`, `underlying_type`, `members`, `layout`).
- Reflection is read-only and never mutates layouts; monomorphised generics record closed `GenericArguments` so tooling and runtime helpers can reason about concrete instantiations without boxing.
- `quote(expr)` (part of `std.meta`) captures Chic syntax trees as hygienic compile-time values. Quoted trees respect lexical hygiene, retain source spans, and can be interpolated into macro expansions.
- Macro expansion remains hygienic and halts at a fixed point; infinite expansion attempts are diagnosed with a deterministic error.
- `quote(expr)` lowers to the Chic-native `Std.Meta.Quote` struct. The compiler records both the original source and the sanitized form (with `${quote(...)}` interpolations rewritten into placeholders), attaches absolute `QuoteSpan` byte offsets, and emits a `QuoteHygiene` seed derived from the declaration site so injected identifiers never collide with user code. The `Captures` list enumerates unique identifier references inside the sanitized tree (placeholders such as `__chic_quote_slot0` are excluded), `Interpolations` carry the evaluated `Std.Meta.Quote` values for each `${...}` segment plus their spans, and `Root: QuoteNode` describes the syntactic structure using a stable `QuoteNodeKind` enum (`Literal`, `Identifier`, `Binary`, `Call`, `Argument`, `Lambda`, `Quote`, `Unknown`, …).
- `${...}` interpolations are evaluated as ordinary const expressions. Each interpolation must produce another `Std.Meta.Quote`; the compiler emits a compile-time diagnostic when a different type is supplied so macros cannot accidentally splice arbitrary runtime values into the tree.
- Macro expansion now runs until no derive/attribute macros remain or until a fixed-point guard trips after 32 passes. Each pass re-walks newly generated items so attribute macros that emit additional derives (or other macros) are expanded deterministically. When the pass limit is exceeded the compiler emits a `macro expansion exceeded` error naming the runaway feature, preventing buggy macros from hanging the pipeline.

### 6.1 Const Functions & CTFE

Chic supports `const fn` for deterministic compile-time evaluation. Const functions run entirely in the CTFE engine, may be invoked from const contexts and ordinary runtime code, and are folded before MIR/backends so static initialisers, default arguments, and const items embed their results without recomputation.

- Syntax: `const fn Name(params) -> Return { ... }`. Functions must be non-async, non-extern, non-unsafe, non-throwing, and non-generic; parameters cannot use `ref`/`out` bindings ([TCK160]).
- Allowed statements: blocks, `const`/`var` with initialisers, `if`/`else`, expression statements, and `return`. Loops, `try`/`using`/`lock`, `yield`, `goto`, and other runtime-only statements are rejected with a deterministic diagnostic ([TCK161]).
- Allowed expressions: literals, identifiers, unary/binary ops, casts, calls to other `const fn` (and whitelisted CTFE intrinsics such as `reflect<T>()`/decimal intrinsics), assignments to local identifiers, member access on constant structs/enums, and `sizeof`/`alignof`/`nameof`/`quote(...)`. Dynamic constructs such as `await`, lambdas, object construction/indexing, pattern matches, and interpolated strings are rejected with [TCK161].
- The CTFE engine enforces fuel limits and memoises both expression folds and const-function invocations (`fn_cache_hits`/`fn_cache_misses` in `ConstEvalMetrics`). Results are cached across module lowering stages so statics, default arguments, and const items share a single evaluation.
- Backends consume the folded `ConstValue` records emitted by MIR; static initialisers, default-argument thunks, and const declarations never re-run const code at runtime.

Diagnostics:

- [TCK160] const fn signature or parameter is not CTFE-safe (async/extern/unsafe/throws/ref/out/generic).
- [TCK161] const fn body uses an unsupported statement/expression.

### 6.2 Async Traits & Generators (Planned)

Trait methods will be able to use `async` directly, lowering to poll-based vtables automatically. This removes the need for external macros and keeps async ergonomics on par with free functions. Generators will piggyback on the same lowering so iterator traits can expose async behaviour without boilerplate.

```chic
public trait AsyncReader
{
    async Task<int> ReadAsync(ref this, Span<byte> buffer);
}
```

Compiler support will synthesise poll shims and executor hooks; details will appear here as the implementation progresses.

### 6.3 Drop Glue Intrinsic

`__drop_glue_of<T>() -> (fn @extern("C")(void*) -> void)?` exposes the compiler-generated drop
glue for fully monomorphised types. The intrinsic returns:

- `null` when `T` is trivially droppable—callers may skip drop callbacks or substitute the shared
  runtime no-op via `Std.Runtime.DropRuntime.DropNoopPtr()`; and
- The address of the synthesised `__cl_drop__T` thunk when `T` requires deterministic destruction.

The pointer is a Chic symbol that codegen lowers to a concrete function address on both LLVM
and WASM backends. Containers and runtime helpers pass the pointer directly to their native hooks
(`chic_rt_vec_*`, slice drops, async unwinds) so heap-owned elements honour the same drop
ordering as stack values. Diagnostics enforce that the intrinsic is invoked with a type argument
and without runtime parameters.

```chic
public void PrepareVec<T>()
{
    var drop_fn = __drop_glue_of<T>();
    if (drop_fn == 0)
    {
        drop_fn = Std.Runtime.DropRuntime.DropNoopPtr();
    }
    var metadata = Std.Runtime.TypeMetadata.Resolve<T>();
    _ = Std.Collections.VecIntrinsics.chic_rt_vec_new(
        (isize)metadata.Size,
        (isize)metadata.Align,
        drop_fn
    );
}
```

At compile time the builder rewrites the intrinsic into a symbol or `null` constant, ensuring no
runtime reflection is required. The same pointer participates in metadata emission so reflection
descriptors and runtime drop registries observe a stable ABI surface.

## 7. Interop (Native & C ABI)
### 7.1 TOON (Token-Oriented Object Notation) Support (Planned)

To reduce token counts when communicating with language models and other agents, Chic will offer a TOON (Token-Oriented Object Notation) output mode alongside JSON. TOON flattens structured data into columnar segments, yielding 30–60% fewer tokens per payload.

Example:

````json
// JSON
{"users": [ { "id": 1, "name": "Alice", "role": "admin" }, { "id": 2, "name": "Bob", "role": "user" } ] }
````

````toon
users[2]{id,name,role}:
1,Alice,admin
2,Bob,user
````

Planned integration points include CLI commands (`chic test --format toon`), RPC/agent endpoints, and standard library serializers. Formal grammar, escaping rules, and streaming semantics will be documented here once the feature lands.

### 7.2 C ABI Interop (Implemented)

Chic provides first-class C ABI boundaries. C interop is explicit and opt-in: Chic's
internal calling convention stays unchanged, and C ABI rules apply only when `@extern("C")`
appears on a declaration or function pointer type.

#### 7.2.1 Imports, Exports, and Linkage

- `@extern("C")` on a function declaration selects the C calling convention for that symbol.
  - On an `extern` declaration it defines an *import*: Chic code may call a C-provided symbol.
  - On a Chic-defined function it defines a *C export*: C code may call the Chic function using
    the C ABI, and the compiler emits a public symbol.
- `@link("<name>")` declares a link dependency for native builds. It is ignored for `wasm32`.
- `@export("<symbol>")` overrides the exported symbol name. When absent, exported functions use
  their canonical Chic name with namespace separators replaced by `__`.

Example import:

```chic
@extern("C") @link("c")
public static extern isize write(int fd, byte* buf, usize count);
```

Example export:

```chic
@extern("C") @export("chic_add2")
public static int Add2(int x) { return x + 2; }
```

#### 7.2.2 C ABI Type Surface

Only ABI-stable, layout-defined types may cross an `@extern("C")` boundary. The compiler rejects
other types with a diagnostic in the `ffi` category.

Allowed categories:

- Scalars: `bool`, `byte`/`sbyte`, `short`/`ushort`, `int`/`uint`, `long`/`ulong`, `isize`/`usize`,
  `char` (UTF-16 code unit), and floating-point scalars (`float`, `double`).
- Pointers: typed pointers `T*`, `void*`, and `fn @extern("C")(...) -> ...` function pointers.
- Aggregates: `struct`/`record struct` with `@repr(c)` or `@repr(packed(N))` (and optional `@align`)
  whose fields are themselves C-ABI-safe. Fixed-size buffers/arrays inside such structs are ABI-safe
  only when their element type is ABI-safe and their layout is fully known at compile time.

Rejected across C ABI boundaries:

- Managed references (`ref T`, `Rc<T>`, `Arc<T>`), `string`/`Str`/slices/spans, trait objects,
  generics that are not fully monomorphised, and any type without a concrete ABI layout.

#### 7.2.3 ABI Classification (Deterministic Rules)

For every `@extern("C")` call, export, and `fn @extern("C")` pointer, the compiler classifies each
parameter and return value into one of:

- **Direct scalar**: passed/returned as an integer, float, or pointer.
- **Direct aggregate**: passed/returned in registers as one or more ABI scalars (target-defined
  register classes).
- **Indirect aggregate**: passed/returned via a hidden pointer (a.k.a. `sret` / “hidden return
  pointer” for returns, and “by-address” for parameters).

Classification is deterministic and target-specific, but must match the platform C ABI for the
supported targets (ELF/Mach-O/COFF on x86_64 and aarch64). Where the platform ABI is genuinely
unsupported (for example, C varargs on `wasm32`), the compiler emits an explicit diagnostic and
rejects the program.

Target-specific aggregate thresholds:

- **x86_64 SysV (ELF/Mach-O)**: aggregates of size ≤16 bytes are passed/returned directly, but are
  *coerced* to integer lanes to match the ABI: sizes ≤8 become `i<size*8>`, sizes 9–15 become
  `{ i64, i<(size-8)*8> }`, and size 16 becomes `{ i64, i64 }`. Larger aggregates use an indirect
  pointer (`sret` for returns, `byval align <N>` for parameters). No HVA/HFA recognition is
  required.
- **aarch64 ELF/Mach-O**: homogeneous floating-point aggregates (all `float` or all `double`,
  1–4 elements) are passed directly in FP registers regardless of size; other aggregates of size
  ≤8 are coerced to `i<size*8>` and sizes 9–16 are coerced to `[2 x i64]`. Larger aggregates are
  indirect. Return values follow the same rule (`sret` when indirect).
- **Windows (COFF) x86_64/aarch64**: non-scalar aggregates whose size is *not* 1, 2, 4, or 8 bytes
  are passed indirectly (`byval align <N>`) and returned via a hidden `sret` pointer; 1/2/4/8 byte
  aggregates are treated as scalars and coerced to the matching integer width for direct passing.

The classifier records the chosen mode so both direct and indirect calls (including function
pointers) lower to the same C ABI shape in LLVM and WASM backends, and the same shape appears in
forward declarations and generated headers.

The lowered ABI rewrites prototypes but not headers: indirect returns become a hidden first
parameter (`ptr`, annotated `sret(<T>), align <N>`), and aggregate parameters that require memory
become `ptr` (with `byval align <N>` on SysV/COFF as appropriate). Imports, exports, and indirect
calls all use the same rewritten prototype; C-facing headers remain in source form so callers use
the natural C signature.

#### 7.2.4 Aggregate Returns (sret)

When an `@extern("C")` function returns a non-scalar aggregate, the ABI classifier determines
whether the value is returned directly (in registers) or indirectly:

- **Direct returns** return a value in the C ABI register convention for the target.
- **Indirect returns** are lowered to a hidden first parameter `out T*` (conceptually), and the
  IR-level return type becomes `void`. The caller allocates a suitably-aligned return slot and
  passes its address.

This rule applies to both direct calls and indirect calls through `fn @extern("C")` pointers, and
it applies symmetrically to imports and exports.

#### 7.2.5 Raw Function Pointer Types and Indirect Calls

Function pointer types use `fn` syntax. Chic distinguishes:

- `fn(params) -> ret` — Chic calling convention (may be a closure-capable representation).
- `fn @extern("C")(params) -> ret` — C ABI *raw* function pointer (a thin pointer).
- `fn @extern("C")(params, ...) -> ret` — C ABI *raw* variadic function pointer (thin pointer).

Rules:

- Raw `@extern("C")` function pointers are thin code addresses (no environment/context cell) and
  always use the C ABI; Chic closures never implicitly coerce to these pointers.
- A value of type `fn @extern("C")(...) -> ...` may be called only from an `unsafe` context.
- The compiler lowers indirect calls by applying the ABI classifier to the pointed-to signature,
  including aggregate returns/parameters and the variadic flag. Hidden `sret` pointers are inserted
  for indirect calls exactly as they would be for direct calls.
- A Chic `@extern("C")` exported function may be taken as a `fn @extern("C")(...) -> ...` value,
  enabling callbacks from C into Chic and round-tripping function pointers across the boundary.
- CPU multiversion dispatch stubs are never generated for variadic functions; such combinations are
  rejected during code generation to avoid mismatched dispatch thunks.

Example (callback with an sret aggregate):

```c
// C side
struct Big { long a, b, c; };
typedef struct Big (*make_big_fn)(long base);
long c_call_chic_make(make_big_fn cb) {
  struct Big v = cb(30);
  return v.a + v.b + v.c;
}
long c_call_chic_sum(long (*sum_cb)(struct Big));
make_big_fn c_provide_big_cb(void);
long c_sum_big(struct Big value);
```

```chic
import Std.Runtime.InteropServices;

@StructLayout(LayoutKind.Sequential)
public struct Big { public long a; public long b; public long c; }

@extern("C") @link("ffi_fnptr") public static extern fn @extern("C")(long) -> Big c_provide_big_cb();
@extern("C") @link("ffi_fnptr") public static extern fn @extern("C")(Big) -> long c_provide_sum_cb();
@extern("C") @link("ffi_fnptr") public static extern long c_call_chic_make(fn @extern("C")(long) -> Big cb);
@extern("C") @link("ffi_fnptr") public static extern long c_call_chic_sum(fn @extern("C")(Big) -> long cb);

@extern("C") @export("chic_make_big")
public static Big ChicMakeBig(long base) { return new Big(base, base + 1, base + 2); }
@extern("C") @export("chic_sum_big")
public static long ChicSumBig(Big value) { return value.a + value.b + value.c; }

public static int Main()
{
    unsafe
    {
        let cb_from_c = c_provide_big_cb();
        let c_result = cb_from_c(10);      // Chic → C indirect call (sret inserted)
        if (c_result.c != 12) { return 1; }

        let sum = c_call_chic_make(ChicMakeBig);  // C → Chic callback (sret on import)
        if (sum != (30 + 31 + 32)) { return 2; }
        let back = c_call_chic_sum(ChicSumBig);
        if (back != (7 + 8 + 9)) { return 3; }
    }
    return 0;
}
```

#### 7.2.6 Pointer Coercions for FFI (Safety Rules)

Interop relies on a small, explicit pointer-coercion set:

- **FFI-only `T* → void*`:** In FFI contexts (an `@extern("C")` declaration or call, including
  indirect calls through `fn @extern("C")` pointers), `T*` may implicitly coerce to `void*`.
  Outside FFI, the same conversion is rejected to keep pointer intent explicit.
- **`void* → T*` requires an explicit cast and `unsafe`.** Implicit conversions are rejected;
  casts inside `unsafe` blocks are allowed and simply reinterpret the bits.
- **`*mut T → *const T` is implicit.** Dropping mutability is allowed anywhere, including
  non-FFI Chic code, provided the element type matches.
- **`null` is a polymorphic null pointer literal.** Its type is inferred from context and may
  initialise `T*`, `void*`, or `fn @extern("C")` pointers. In ambiguous cases, an explicit cast
  (e.g. `(int*)null`) is required.
- **Raw pointer casts** (`(U*)ptr`, integer↔pointer casts, and pointer reinterpretation) are
  `unsafe`.

Example (Chic ↔ C):

```chic
@StructLayout(LayoutKind.Sequential)
public struct Value { public long marker; public long other; }

@extern("C") @link("ffi_pointers") public static extern void touch_void(*mut void ptr);
@extern("C") @link("ffi_pointers") public static extern long read_const(*const Value ptr);
@extern("C") @link("ffi_pointers") public static extern void* get_void_pointer();
@extern("C") @link("ffi_pointers") public static extern int is_null(void* ptr);

public static int Main()
{
    unsafe
    {
        var local = new Value { marker = 1, other = 2 };
        let mut_ptr = &local;
        touch_void(mut_ptr);                 // `Value*` → `void*` (FFI-only)
        let sum = read_const(mut_ptr);       // `*mut Value` → `*const Value`

        let raw = get_void_pointer();
        var typed = (*mut Value)raw;         // explicit + unsafe for `void*` → `Value*`
        (*typed).marker = 123;
    }
    if (is_null(null) == 0) { return 1; }         // `null` inhabits any pointer type
    return 0;
}
```

#### 7.2.7 C Varargs

Variadic C functions are supported only for `@extern("C")` declarations and `fn @extern("C")`
pointer types:

```chic
@extern("C") @link("c")
public static extern int printf(byte* fmt, ...);
```

Rules:

- Declaring or calling a variadic C function requires `unsafe`.
- For varargs calls, the compiler applies the default argument promotions required by the C ABI:
  `float → double`, and integer types narrower than `int` promote to `int` (or the target’s
  promoted integer type).
- Varargs payloads accept only C-ABI-safe scalars and pointers; aggregates are rejected unless the
  target ABI explicitly supports them.
- The variadic flag is preserved on `fn @extern("C")(..., ...) -> ...` function pointer types so
  indirect calls obey the same classification. LLVM lowering emits variadic prototypes on
  declarations, definitions, and call sites so targets that require a register save area (e.g.
  aarch64) spill the fixed arguments correctly.
- `wasm32` rejects C varargs with a deterministic diagnostic (`ffi` category).
- Dynamic FFI stubs (those that rely on runtime `dlopen`/`GetProcAddress`) are not generated for
  variadic imports; such declarations are rejected so callers cannot observe partial resolution.

#### 7.2.8 Thread-Local Storage (TLS) for Statics

`@threadlocal` marks a `static` as thread-local:

```chic
@threadlocal
public static mutable int Counter = 0;
```

Rules:

- Each OS thread observes an independent instance of the variable.
- Initialisers must be compile-time constants. When omitted, the variable is zero-initialised.
- TLS variables are instantiated before the thread executes any Chic code; there is no lazy
  runtime initialisation hook and no cross-thread synchronisation.
- Combine `@threadlocal` with `@extern("C")` / `@link` when importing or exporting C TLS symbols so
  headers and object emission both mark the storage as `thread_local`.
- Native targets emit LLVM `thread_local` globals using the platform’s default TLS model; the
  address of a TLS static refers to the calling thread’s instance.
- Unsupported targets emit explicit diagnostics. (`wasm32` does not provide native TLS in the
  bootstrap toolchain.)

#### 7.2.9 Weak Linkage

Weak linkage is available for native targets:

- `@weak` on a definition emits a weak definition (overridable by a strong definition in another
  object/library).
- `@weak_import` on an `extern` declaration marks the symbol as an optional import. Its address may
  be `null` when the symbol is absent.
- Apply `@weak` to functions or statics. Weak statics preserve layout/initialisation rules from
  their non-weak counterparts.
- Weak linkage is native-only. `wasm32` rejects `@weak` and `@weak_import` with an `ffi` diagnostic.

Presence checks must be explicit and `unsafe`:

```chic
@extern("C") @weak_import
public static extern int optional_feature();

public static unsafe int CallIfPresent()
{
    let fp: fn @extern("C")() -> int = optional_feature;
    if (fp == null) { return 0; }
    return fp();
}
```

Windows/COFF builds reject `@weak_import` (use dynamic `@extern(library=..., optional=true)`).

#### 7.2.10 Tooling: Header Generation

The toolchain can emit C-compatible headers for public exports:

- `chic header <file> -o <out.h>` generates a header for the module’s public API.
- `chic build --crate-type lib --emit-header` emits a header alongside the built library.

Generated headers preserve `@extern("C")` calling convention, `@export` names, and the declared
C-compatible types. Unsupported constructs are rejected rather than silently erased.

> **Implementation status (layout):** The bootstrap compiler preserves `@repr(c)` and
> `@repr(packed(N))` when lowering into MIR type layouts and enforces minimum alignment requests
> from `@align(N)`. Packing clamps field offsets and aggregate alignment while the type checker
> cross-checks the resulting layout metadata so ABI mismatches surface deterministically.

#### 7.2.11 Extern Globals (Imports/Exports)

Chic supports C ABI globals via a dedicated surface form that composes with the existing
`@extern`/`@link` metadata. Canonical syntax (Option A):

```chic
@extern("C") @link("c")
public extern static mut int errno;

@extern("C") @weak_import
internal extern static const void* optional_anchor;

@extern("C") @threadlocal
public extern static mut int tls_counter;
```

Rules:

- **Form:** `extern static const T name = <const>;` defines/export a global; `extern static mut T name;`
  imports a mutable global; `extern static const T name;` imports a readonly global. The `extern`
  keyword is required; applying `@extern` without the keyword is rejected.
- **Attributes:** `@extern("C", alias = "...", library = "...", binding = ..., optional = ...)`
  selects the ABI, library, and symbol name (defaults to the identifier). `@link("lib")` declares a
  static link dependency. `@weak` marks a Chic definition as weak; `@weak_import` marks an import as
  optional. `@threadlocal` produces TLS storage where supported.
- **Type surface:** Only FFI-safe layouts are allowed: primitives, raw pointers (including
  `fn @extern("C")` pointers), and structs/records/unions with explicit `@repr(c)`/`@repr(packed(N))`
  whose fields are themselves FFI-safe. Unsized types, managed references, slices/strings, trait
  objects, and generics without a concrete layout are rejected.
- **Initialisation:** Imports may not provide initialisers. Exports must have a compile-time
  constant initializer; omit the initializer to import the symbol (use `= 0` for zeroed exports).
  TLS exports follow the same rule.
- **Safety:** Reading or writing a mutable extern global requires `unsafe` (per-thread for TLS); safe
  wrappers may expose narrower APIs. Const extern globals may be read safely.
- **Linkage/visibility:** `public` globals are emitted for linking and appear in generated headers;
  `internal` globals remain module-local. Weak imports resolve to a nullable address; weak exports
  emit weak definitions. Thread-local imports/exports use the platform TLS model; unsupported
  targets emit an `ffi` diagnostic (“extern globals unsupported on wasm backend”).
- **Diagnostics:** The compiler rejects extern globals with initialisers on imports, non-FFI-safe
  types, missing `extern` keyword, unsupported attributes (`binding` without a `library`, duplicate
  `@extern`/`@link`, `@extern(library=...)` dynamic bindings), or targets that cannot materialise
  the storage model.

## 8. Optional OOP Ergonomics

- Prefer structs + free functions for hot paths.
- Use extension methods for dot-syntax without compulsory classes.
- Introduce interfaces only when dynamic dispatch across packages is required.
- Support record-like value types:
  - Surface syntax mirrors C# record structs: `record struct Point(int X, int Y);` for positional, or `record struct Point(int X, int Y) { public int Z; }` when mixing positional fields with a body.
  - Value semantics identical to structs (stack storage, implicit copy), but every non-static field is implicitly `readonly` and primary-ctor parameters become required readonly fields.
  - The compiler synthesises a positional constructor when parameters are present; otherwise the implicit value-type default constructor is used. Object initialisers may assign readonly record fields during construction.
  - Pattern matching uses primary-ctor order for positional patterns (`case Point(var x, var y):`) and named-field patterns continue to work for both positional and body fields.
  - Auto-generated `op_Equality`/`op_Inequality` and `GetHashCode` extensions are emitted for non-generic records unless explicitly derived. Reflection reports `TypeKind::Record` and preserves layout/layout hints alongside the readonly marker.
  - Interop matches structs: `@repr`/`@align` apply, and the runtime treats records as immutable value types (no object headers, no vtables).

```chic
public record struct Vec2(float X, float Y);
public record struct LabeledVec2(float X, float Y) { public string Label; }
```

### 8.1 Placement Construction (Planned)

To minimise copies and support lock-free data structures, Chic will offer placement construction APIs:

```chic
public void Push<T>(ref Vec<T> vec, in T value)
{
    emplace(ref vec.EmplaceSlot(), value);
}
```

Placement constructors (`emplace`, `placement init`) guarantee that elements are initialised in-place without intermediate moves. The compiler enforces aliasing rules so that partially-initialised slots are tracked via `MaybeUninit<T>`. Detailed semantics and runtime support will be documented here once implemented.

## 9. Syntax Quick Sheet

### 9.1 Borrow-Aware Pattern Matching

Pattern bindings participate fully in Chic’s ownership model. Every binding can
explicitly describe how it projects the matched value by placing a modifier before
`let`/`var` *or* immediately after the identifier:

```chic
switch (packet)
{
    case Header(ref var header) when header.Validate():
        return ProcessHeader(ref header);

    case in var snapshot:
        return snapshot.Length;

    case Payload payload move:
        return Deliver(move payload);

    default:
        return -1;
}
```

Supported modifiers are:

| Modifier  | Placement                           | Effect |
|-----------|-------------------------------------|--------|
| *(none)*  | —                                   | Value binding (copy if possible, otherwise move). |
| `move`    | `move var value` / `var value move` | Forces a move into the binding, even when the source is normally copyable. |

Borrow-style modifiers (`in`, `ref`, `ref readonly`) remain limited to parameter and receiver positions.
For example, `case Packet.Header(ref var header)` now emits CL0031 rather than capturing a borrow;
use the `ref T`/`ref readonly T` types from Section&nbsp;2.2 when a first-class reference is required.
Moving patterns such as `case Packet.Header(var payload move)` remain available.

Guards (`when expr`) execute while any pending move is active. If the guard fails, the compiler
rolls the binding back before testing the next arm so later patterns see the original value.
Each switch section defines a single binding scope:

- A name must use the same mutability (`let` vs `var`) in every arm where it appears.
- `move` bindings cannot appear in `when` expressions that would observe a partially-initialised value.

At the MIR level, the compiler now emits explicit `Move` statements for pattern bindings.
These operations feed directly into the loan checker, so attempting to mutate a scrutinee while a
shared `in` binding is live, or trying to use a `move`d value later in the function, produces the
standard borrow-checker diagnostics.



- Bindings: `let` (immutable), `var` (mutable), `const` (compile-time).
- Receivers: `in this` (read-only), `ref this` (mutable unique).
- Parameters: `in T x`, `ref T x`, `out T x`.
- Destructors: `public void dispose(ref this) { ... }`.
- Pattern matching: `switch` supports destructuring for enums & structs.
- Generics: `where T : IFoo, IBar`.
- Errors: `Result<T, E>` with `?` propagation; `panic` is exceptional.

### 9.2 AI Systems Essentials

```chic
@diff(reverse)
graph FFN {
    node y1 = matmul(x, w1);
    node y2 = gelu(y1);
    node y3 = matmul(y2, w2);
    output y3;
}

schedule FFN {
    tile(matmul, M=128, N=128, K=64);
    fuse(gelu, into: matmul#1);
    place(*, stream=0);
    memory_plan(region: "ephem");
}

public effects(random)
async fn run_batch<const B: usize, const H: usize>(
    x: Tensor<f16, [Dim<B>, Dim<H>], RowMajor, Host>,
    rng: ref RNG,
    s: ref Stream<Gpu<0>>)
    -> Tensor<fp8_e4m3, [Dim<B>, Dim<H>], RowMajor, Host>
{
    region ephem {
        let x_d = to_device_async(x, s); await ready(x_d);
        let y = FFN(x_d) @use_schedule(FFN, profile = "a100-v1");
        let y_do = dropout(y, 0.1, rng); // effects(random)
        let y_h = to_host_async(qcast<fp8_e4m3>(y_do), s); await ready(y_h);
        return y_h;
    }
}
```

- Declare effects with `effects(...)` (before any `throws`). Agents can inspect the same metadata in `mir.json`.
- `Tensor<T, Shape, Layout, MemSpace>` carries full shape/layout/device information; views never allocate.
- `graph` + `schedule` pair to pre-compile compute graphs; `@use_schedule` binds a tuned profile at build time.
- `region name { ... }` scopes allocator lifetimes; leaving the block frees every allocation deterministically.
- Streams (`Stream<M>`) and RNG handles (`RNG`) are linear capabilities—borrow them explicitly to overlap compute and I/O.

## 10. End-to-End Styles

### Data-First

```chic
namespace Images;

public struct Image { public Span<byte> Pixels; public int W; public int H; }

public void Grayscale(ref Image img)
{
    for (var i = 0; i < img.Pixels.Len(); i += 4) {
        let r = img.Pixels[i];
        let g = img.Pixels[i+1];
        let b = img.Pixels[i+2];
        let y = (r*30 + g*59 + b*11) / 100;
        img.Pixels[i] = img.Pixels[i+1] = img.Pixels[i+2] = (byte)y;
    }
}

public extension Image
{
    public void SavePpm(in this, string path) { /* POSIX write() */ }
}
```

### Optional OOP

```chic
namespace Net;

public interface IRequestHandler { void Handle(ref this Request req); }

public class Router : IRequestHandler
{
    public void Handle(ref this Request req) { /* dispatch to routes */ }
}
```

Both paradigms converge on identical safety and performance guarantees.

## 11. Compiler Implications

- **Parser:** C-family grammar with namespace-scope functions and type members.
- **Type System:** Interface constraints, borrow qualifiers, move vs. borrow semantics.
- **Intermediate Representation (MIR):** Explicit `Move`, `BorrowRead`, `BorrowWrite`, `Drop`, `Call`. See [docs/mir_design.md](docs/mir_design.md) for the full MIR design.
- **Borrow Checker:** Non-lexical lifetimes, enforce "many `in` or one `ref`", partial moves, scheduled drops.
- **Backends:** LLVM (with LTO/PGO hooks) remains the production path, and the in-house WASM backend now covers fast iteration. The former Cranelift integration has been removed. The WASM pipeline currently supports structured control flow, integer arithmetic, and host execution via the bundled interpreter. Both LLVM and WASM emit binaries for `x86_64-*` and `aarch64-*` targets; supporting additional architectures requires extending the lowering rules and backend build configurations.
- **CLI build artifact:** `chic build` accepts `--backend llvm` (default) or `--backend wasm`. Executables default to `<source>.clbin`, static libraries to `<source>.a` (or `<source>.lib` on Windows) with a companion `<source>.clrlib`, and shared libraries follow the platform extension (`.so`/`.dylib`/`.dll`). `--output` overrides the artifact root. Regardless of backend, the driver records the textual MIR/metadata summary in its report while leaving the generated object (and, for LLVM, the `.ll`) on disk when `keep_object` is enabled.
- **CLI test runner:** `chic test` discovers all `testcase` declarations (sync + async) and routes
  async cases through the runtime executor. When `CHIC_SKIP_STDLIB=1` with async/startup overrides,
  async LLVM tests emit `[SKIP] ... requires the runtime executor`; WASM harnesses execute when the
  runtime hooks are present and otherwise skip, keeping discovery output authoritative for tooling.

## 12. Formal Grammar (EBNF Snapshot)

Notation: `[...]` marks optional elements, `{...}` marks repetition, and string literals represent exact tokens.

### 12.1 Declarations

```ebnf
CompilationUnit       ::= { ImportDirective } FileScopedNamespace? ItemList EOF
FileScopedNamespace   ::= 'namespace' QualifiedIdent ';'
NamespaceDecl         ::= 'namespace' QualifiedIdent NamespaceBody
NamespaceBody         ::= '{' ItemList '}'
QualifiedIdent    ::= Identifier { '.' Identifier }
ItemList          ::= { Item }
Item              ::= FunctionDecl
                    | StructDecl
                    | EnumDecl
                    | ClassDecl
                    | InterfaceDecl
                    | ExtensionDecl
                    | TestCaseDecl
                    | GraphDecl
                    | ScheduleDecl
                    | NamespaceDecl
                    | ImportDirective

Attributes        ::= Attribute { Attribute }
Attribute         ::= '@' Identifier AttributeArgs?
AttributeArgs     ::= '(' [ ArgumentList ] ')'
ArgumentList      ::= Expression { ',' Expression }

Visibility        ::= 'public' | 'internal' | 'private'
Modifiers         ::= { Identifier }  // e.g. static, async, virtual
TypeExpr          ::= Identifier TypeSuffix*
TypeSuffix        ::= GenericArgs | ArraySuffix | NullableSuffix | PointerSuffix | QualifierSuffix
GenericArgs       ::= '<' TypeExpr { ',' TypeExpr } '>'
ArraySuffix       ::= '[' ']'
PointerSuffix     ::= '*'
QualifierSuffix   ::= '.' Identifier
NullableSuffix    ::= '?'
ImportDirective   ::= ['global'] 'import' QualifiedIdent ';'
                    | ['global'] 'import' Identifier '=' QualifiedIdent ';'
                    | ['global'] 'import' 'static' QualifiedIdent ';'
TestCaseDecl      ::= 'testcase' Identifier TestSignature? Block
TestSignature     ::= '(' [ Parameter { ',' Parameter } ] ')'
```

#### 12.1.1 Import Directive Semantics

Import directives must appear as a contiguous block at the top of a compilation unit or at the start
of a namespace body. Once a non-`import` item (declaration, attribute list, etc.) is encountered, any
subsequent directive in the same scope is rejected with an ordering diagnostic. `@cimport`
attributes expand into synthetic `using` directives and therefore follow the same rule. A directive
may be prefixed with `global` to promote its scope to the entire compilation unit; global directives
obey the same ordering rule and may appear inside block-scoped namespaces even though their effect
is global. `global` is rejected unless it immediately precedes `import`.

Three directive forms are supported:

- **Namespace import** (`import Namespace.SubNamespace;`) makes all public types nested inside the
  imported namespace available for unqualified lookup. Resolution first checks aliases, then the
  current type (allowing `Self`-style references), the current namespace chain, imported
  namespaces, and finally fully-qualified names. Ambiguous resolutions produce an error that lists
  every candidate.
- **Alias import** (`import Alias = Namespace.Type;`) binds `Alias` to a namespace or type. Aliases
  participate in the normal lookup order, may refer to other namespaces imported in the same scope,
  and are prohibited from forming cycles.
- **Static import** (`import static Namespace.Type;`) exposes the target type’s `static` fields,
  properties, constants, and methods without requiring the qualifying type name. Only members marked
  `static` are considered; instance members must still be accessed through an object or qualified
  type reference. When multiple static imports expose the same member name, the compiler reports an
  ambiguity that must be resolved by either qualifying the access or narrowing the set of imports.

`using` directives are not supported; use `import`. Resource-management `using` statements (`using (resource)`, `using var ...`) are unchanged.

Alias bindings are resolved before any other imports, and when duplicate names exist the innermost
scope wins (global aliases provide a fallback when no local alias is defined). After alias
substitution the compiler checks the enclosing type (supporting nested `Self.Member` lookups), walks
the current namespace chain, evaluates global namespace/static imports, then processes the local
imports declared for that file or namespace before finally attempting the fully-qualified name.
Diagnostics reference the file/namespace scope that introduced the conflicting directives so authors
can quickly identify the offending `using`.

### 12.2 Functions

```ebnf
FunctionDecl      ::= Attributes? Modifiers? Visibility? TypeExpr Identifier ParameterList EffectsClause? ThrowsClause? FunctionTail
ParameterList     ::= '(' [ Parameter { ',' Parameter } ] ')'
Parameter         ::= BindingModifier? TypeExpr Identifier [ '=' Expression ]
BindingModifier   ::= 'in' | 'ref' | 'out'
EffectsClause     ::= 'effects' '(' EffectIdent { ',' EffectIdent } ')'
EffectIdent       ::= Identifier
ThrowsClause      ::= 'throws' TypeExpr { ',' TypeExpr }
FunctionTail      ::= Block
                    | '=>' Expression ';'
                    | ';'

Parameters marked with `in`, `ref`, or `out` expose reference semantics to the caller. Chic
requires call sites to repeat the binding modifier so intent is explicit; the compiler rejects
`Mirror(x)` when the signature is `Mirror(ref int value)` and guides the author to write
`Mirror(ref x)` instead. By-value arguments (`value`) are the default and do not accept
modifiers. Optional default arguments reuse the full expression grammar; the parser attaches the
expression to the parameter node so lowering/typeck can substitute the value when a call site omits
an argument. The MIR encodes the selected argument mode so borrow checking and codegen can enforce
read-only versus mutable access and ensure `out` parameters are treated as definitely assigned.

The optional `effects` clause enumerates capability identifiers (`random`, `measure`, `network`,
`cancel`, etc.). The clause contributes to the function type and must appear before any `throws`
clause. `throws` lists exception types that may escape. Both clauses participate in overload
resolution and monomorphisation; omitting them is equivalent to declaring an empty set. Call sites
must satisfy the callee’s declared effects and either handle or forward the listed exceptions.

### 12.3 Graph & Schedule Declarations

```ebnf
GraphDecl        ::= Attributes? 'graph' Identifier GraphBody
GraphBody        ::= '{' { GraphElement } '}'
GraphElement     ::= GraphNode
                   | GraphInput
                   | GraphOutput
GraphNode        ::= 'node' Identifier '=' Expression ';'
GraphInput       ::= 'input' Identifier ':' TypeExpr ';'
GraphOutput      ::= 'output' Expression ';'

ScheduleDecl     ::= Attributes? 'schedule' Identifier ScheduleBody
ScheduleBody     ::= '{' { ScheduleDirective ';' } '}'
ScheduleDirective::= Identifier '(' [ ScheduleArgumentList ] ')'
ScheduleArgumentList ::= ScheduleArgument { ',' ScheduleArgument }
ScheduleArgument ::= Identifier
                   | Identifier '=' Expression
                   | Identifier ':' Expression
                   | '*'
                   | Expression
```

`graph` declarations capture static dataflow DAGs. Nodes refer to named Chic functions,
intrinsics, or other graph nodes in the same body. `output` terminates the graph by selecting one
or more expressions. Schedules (`schedule`) attach transformation directives to graphs; each
directive uses a uniform call-like syntax so the grammar stays LL(1). Semantic validation ensures
arguments reference existing nodes, attributes, or device resources.

Block             ::= '{' StatementList '}'
StatementList     ::= { Statement }

### 12.6 Statements

```ebnf
Statement          ::= Block
                    | LabeledStatement
                    | DeclarationStatement ';'
                    | ExpressionStatement ';'
                    | IfStatement
                    | SwitchStatement
                    | WhileStatement
                    | DoStatement
                    | ForStatement
                    | ForeachStatement
                    | RegionStatement
                    | CancelScopeStatement
                    | BreakStatement
                    | ContinueStatement
                    | ReturnStatement
                    | ThrowStatement
                    | TryStatement
                    | UsingStatement
                    | LockStatement
                    | CheckedStatement
                    | UncheckedStatement
                    | FixedStatement
                    | UnsafeStatement
                    | YieldStatement
                    | GotoStatement

LabeledStatement   ::= Identifier ':' Statement
DeclarationStatement ::= VariableDeclaration
ExpressionStatement ::= Expression

IfStatement        ::= 'if' '(' Expression ')' EmbeddedStatement [ 'else' EmbeddedStatement ]
WhileStatement     ::= 'while' '(' Expression ')' EmbeddedStatement
DoStatement        ::= 'do' EmbeddedStatement 'while' '(' Expression ')' ';'
ForStatement       ::= 'for' '(' [ForInitializer] ';' [Expression] ';' [ExpressionList] ')' EmbeddedStatement
ForeachStatement   ::= 'foreach' '(' ForEachBinding 'in' Expression ')' EmbeddedStatement
SwitchStatement    ::= 'switch' '(' Expression ')' '{' { SwitchSection } '}'
TryStatement       ::= 'try' EmbeddedStatement { CatchClause } [ FinallyClause ]
UsingStatement     ::= 'using' '(' ResourceAcquisition ')' EmbeddedStatement
                    | 'using' VariableDeclaration ';'
LockStatement      ::= 'lock' '(' Expression ')' EmbeddedStatement
CheckedStatement   ::= 'checked' Block
UncheckedStatement ::= 'unchecked' Block
FixedStatement     ::= 'fixed' '(' VariableDeclaration ')' EmbeddedStatement
UnsafeStatement    ::= 'unsafe' EmbeddedStatement
YieldStatement     ::= 'yield' ('return' Expression | 'break') ';'
GotoStatement      ::= 'goto' ( Identifier | 'case' Expression | 'default' ) ';'
BreakStatement     ::= 'break' ';'
ContinueStatement  ::= 'continue' ';'
ReturnStatement    ::= 'return' [Expression] ';'
ThrowStatement     ::= 'throw' [Expression] ';'
RegionStatement    ::= 'region' Identifier Block
CancelScopeStatement ::= 'cancel_scope' [ '(' Expression ')' ] Block

EmbeddedStatement  ::= Statement
ForInitializer     ::= DeclarationStatement | ExpressionList
ExpressionList     ::= Expression { ',' Expression }
ForEachBinding     ::= ForeachBindingModifier? ForeachBindingHead
ForeachBindingModifier ::= 'in'
                        | 'ref' ['readonly']
ForeachBindingHead ::= ( 'let' | 'var' ) Identifier
                     | TypeExpr Identifier
SwitchSection      ::= { SwitchLabel } StatementList
SwitchLabel        ::= 'case' Expression ':' | 'default' ':'
CatchClause        ::= 'catch' [ '(' CatchType Identifier? ')' ] [ 'when' '(' Expression ')' ] EmbeddedStatement
CatchType          ::= TypeExpr
FinallyClause      ::= 'finally' EmbeddedStatement
ResourceAcquisition ::= DeclarationStatement | Expression
```
```

### 12.4 Structs and Enums

```ebnf
StructDecl        ::= Attributes? Modifiers? Visibility? 'struct' Identifier StructBody
StructBody        ::= '{' { FieldDecl } '}'
FieldDecl         ::= Attributes? Visibility? TypeExpr Identifier ';'

EnumDecl          ::= Attributes? Modifiers? Visibility? 'enum' Identifier EnumBase? EnumBody
EnumBase          ::= ':' TypeExpr
EnumBody          ::= '{' EnumVariant { ',' EnumVariant } ','? '}'
EnumVariant       ::= Identifier EnumVariantPayload?
EnumVariantPayload::= '{' { FieldDecl } '}'
```

#### 12.4.1 Enum Discriminants & Flag Semantics

- **Explicit discriminants** use the syntax `Variant = Expression`. The expression must be a constant and may compose integer literals, previously declared variants, unary `-`/`~`, and the binary operators `+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`, `<<`, and `>>`. Division and remainder by zero are rejected at compile time, as are results that overflow the signed 128-bit range used for Chic discriminants.
- **Implicit discriminants** default to `0` for the first payload-free variant and increment by `1` thereafter. Variants with payload fields always use their declaration index as the discriminant unless the enum is marked as a bitflag (see below).
- **Value reuse diagnostics**: reusing a previously assigned discriminant yields a lowering diagnostic that names both variants involved.

Flag-style enums are opt-in via the `@flags` attribute applied to the enum declaration:

- **Bit allocation:** the compiler reserves bit `0` for the first variant (usually `None`) and auto-assigns subsequent variants the next available power-of-two mask. Explicit discriminants must be non-negative; any newly introduced bit must be a single power of two.
- **Structural rules:** flag variants cannot declare payload fields. Attempting to mix payloads or to assign composite masks (e.g. `ReadWrite = 3`) before the constituent single-bit variants exist results in diagnostics.
- **Metadata:** lowered MIR layout records a `is_flags` marker and the concrete discriminant of every variant. Both LLVM and WebAssembly backends lower flag values to the enum’s underlying integer representation (defaulting to `int`/`Int32` when no base is supplied), enabling bitwise operations without additional casts.
- **Runtime helpers:** `chic::runtime::flags` exposes utilities for formatting, parsing, combination, and iteration of flag masks. These helpers are used by generated code and are available to user programs for consistent diagnostics.

#### 12.4.2 Enum Underlying Types (Implemented)

- **Syntax:** an optional base clause after the enum name selects the underlying integral type: `enum Status : Int16 { Pending = 0, Active = 1 }`. When omitted, the underlying type is `int`/`Int32`.
- **Allowed types:** only integral primitives may appear in the base clause: `sbyte`/`byte` (`Int8`/`UInt8`), `short`/`ushort` (`Int16`/`UInt16`), `int`/`uint` (`Int32`/`UInt32`), `long`/`ulong` (`Int64`/`UInt64`), `nint`/`nuint`, and `char`. Pascal-case `Std.*`/`System.*` aliases and shorthand names (`Int16`, `UInt32`, etc.) are accepted; non-integral types (floating point, decimal, bool, structs, reference types) are rejected with a diagnostic: “Only integral numeric types may be used as enum underlying types; found `<T>`.”
- **Range validation:** every explicit discriminant must fit in the chosen underlying type; compile-time evaluation rejects negative values for unsigned bases and any value outside the signed/unsigned range. Auto-incremented discriminants use the underlying type’s width starting from `0`; overflow (for example `enum Small : byte { Zero = 255, TooBig }`) is diagnosed. Payload variants still use their declaration index as the discriminant but must also fit in the underlying range.
- **Flags and bit allocation:** `@flags` enums use the underlying width for power-of-two assignment. Requesting a bit beyond the width (for example `UInt16` flags with more than 16 distinct bits) or supplying a negative/packed value is rejected.
- **Layout & ABI:** enum layout size and alignment match the underlying type; backends pass and store enums using that width. Signedness is derived from the underlying type, not from the `@flags` attribute.
- **Conversions:** enums remain nominal. Payload-carrying enums cannot be cast to numeric types. Payload-free enums allow explicit casts to and from integral types; casting to the underlying type is exact, and casting from other widths follows the usual numeric cast rules (including truncation warnings). Values outside the declared variant set are allowed but remain distinct nominal values of the enum type.
- **Reflection/metadata:** type metadata records the underlying type alongside the variant list so runtime inspection and tooling can surface it.

Examples:

```chic
enum Color {
    Red,
    Green,
    Blue,
}

enum Status : Int16 {
    Pending = 0,
    Active  = 1,
    Closed  = 2,
}

@flags
public enum Permissions : UInt16 {
    None  = 0,
    Read  = 1,
    Write = 2,
    Exec  = 4,
}
```

### 12.5 Pattern Matching

```ebnf
SwitchStmt        ::= 'switch' '(' Expression ')' '{' { SwitchArm } '}'
SwitchArm         ::= 'case' Pattern ArmGuard? ':' StatementList
                    | 'default' ':' StatementList
ArmGuard          ::= 'when' Expression

Pattern           ::= Identifier PatternPayload?
                    | QualifiedIdent PatternPayload?
                    | Literal
                    | '_'                         // wildcard

PatternPayload    ::= '{' PatternFieldList '}'
PatternFieldList  ::= PatternField { ',' PatternField } ','?
PatternField      ::= Identifier ':' Pattern
                    | Identifier
                    | Literal
```

## 13. Borrow Qualifier Lowering

- **`in` parameters** are lowered to an immutable borrow:
  - MIR emits `BorrowRead` from the caller-owned value into a temporary region tied to the call frame.
  - The callee receives a read-only reference; the source remains usable after the call.
  - Drops are scheduled only on the original owner.
- **`ref` parameters** become unique mutable borrows:
  - MIR emits `BorrowWrite` ensuring exclusivity for the borrow region.
  - The source variable is marked as inaccessible for the duration of the call frame.
  - Any drop in the callee is treated as acting on the borrowed place; ownership does not transfer.
- **`out` parameters** are unique borrows of possibly-uninitialized memory:
  - MIR creates an `UninitPlace` and a `BorrowWrite` to that place.
  - The callee must assign exactly once; the borrow checker enforces definite assignment before return.
  - On success the ownership of the newly initialized value returns to the caller; failure paths must either `panic` or explicitly `Drop` partial state.
- **Moves vs. borrows**:
  - Passing a value without a qualifier moves it into the callee; the caller must not use it until reassigned.
  - `move` expressions are lowered to explicit `Move` ops so partial moves can be tracked and their drops scheduled when the borrow checker determines the last use.

## 14. Attribute Expansion & Hygiene

- **Evaluation order:**
  1. Built-in attributes (`@extern`, `@link`, `@no_std`, `@export`, `@repr`, etc.) are applied first; they mutate the semantic model but do not rewrite syntax.
  2. Declarative macros marked with `@derive` expand next, each emitting zero or more new items before the original target.
  3. Procedural attribute macros run last, in source order from outermost to innermost attributes, and may replace or augment the annotated node.
  4. Expansion iterates until a fixed point is reached; cycles are detected and reported as errors.
- **Hygiene rules:**
  - Newly introduced identifiers are hygienic by default; they live in a macro-specific namespace and cannot capture user bindings unless explicitly exported.
  - Macros can mark identifiers with `@expose(Name)` to deliberately leak into the surrounding scope; doing so requires the macro crate to declare the symbol in its manifest to aid tooling.
  - Attribute macros execute in an isolated module scope; they may request access to surrounding names via `@import(Name)` directives that the compiler resolves before expansion.
  - Diagnostics produced inside macros track both the expansion site and the original invocation site to aid debugging.

## 15. Modules & Packaging

### 15.1 Conditional Compilation

`@cfg(<condition>)` filters items and statements using the same expression grammar as textual `#if` directives (§3.1). The compiler evaluates `@cfg` twice: once after preprocessing/parse and again after macro expansion. Inactive nodes are removed before name resolution or MIR construction, so dead branches generate no metadata, drop glue, or backend symbols.

- **Where it applies:** modules/namespaces, structs/classes/interfaces/traits/extensions (including members), free functions and methods, constructors, properties/accessors, testcases, local functions, and statements (`if`/`switch`/loops/try`/using`/region`/checked`/unsafe`, etc.). `@cfg` attached to an `if`/loop body collapses the branch; a lone active `else` is hoisted.
- **Expression grammar:** boolean/string literals, identifiers, `!`, `&&`, `||`, parentheses, equality (`==` or single `=`), and inequality (`!=`). Mixing string and boolean operands in a comparison is an error. Unknown identifiers evaluate to `false`.
- **Defines available to conditions:** the shared map from §3.1 (`DEBUG`/`RELEASE`/`PROFILE`, `TARGET`/`TARGET_TRIPLE`/`TARGET_ARCH`/`TARGET_OS`/`TARGET_ENV`, `BACKEND`, `KIND`, and `feature_<name>` from CLI `--define feature=...`). Keys are case-insensitive; CLI overrides win.
- **Diagnostics:** missing parentheses or malformed expressions raise errors at the attribute site and are treated as inactive. Conflicting attributes are stripped after evaluation; other attributes remain intact for later passes.
- **Portability:** stdlib and workspace sources both run through the pass, so backend-specific shims can be isolated cleanly with `@cfg(BACKEND == "wasm")` or `@cfg(target_os == "windows")`.

### 15.2 Optimisation Hints

Functions, methods, constructors, and testcases accept optional code-generation hints:

```chic
@hot @always_inline
public void FastPath(in Span<byte> data) { ... }

@cold @never_inline
public void RareFallback() { ... }
```

- `@hot` / `@cold` mark likely/rare paths. `@always_inline` / `@never_inline` request or forbid inlining. The compiler rejects duplicates and conflicts (`@hot`+`@cold`, `@always_inline`+`@never_inline`).
- Hints do not change language semantics, determinism, or borrow rules; they only guide backend lowering. MIR retains the flags on `MirFunction.optimization_hints`.
- Backend mapping:
  - **LLVM:** emits function attributes `hot`/`cold`/`alwaysinline`/`noinline`.
  - **WASM:** writes a custom metadata section `chic.hints` containing `symbol:hint|hint` entries for engines/tooling to consume.
- Hints are advisory; inline decisions remain subject to backend constraints (e.g., recursive functions stay non-inlinable).

### 15.3 Fuzz & Property Testing (Planned)

Future versions of the test runner will understand `proptestcase` and `fuzzcase` annotations, integrating deterministic property and fuzz testing directly into `chic test`. Syntax and runner behaviour will expand this section once stabilised.

### 15.4 Safe Reinterpretation (Planned)

A checked replacement for raw `transmute` is planned:

```chic
public reinterpret<byte[16], Uuid>(in bytes) requires sizeof(Uuid) == 16;
```

The compiler will verify layout, size, and alignment constraints and require justification attributes for edge cases. Semantics and diagnostics will be captured here when implemented.



- **Package manifest:** Every package roots at a YAML manifest named `manifest.yaml`, located at `packages/<name>/manifest.yaml`. Nested manifests are not supported. The manifest captures package metadata, default namespace prefix, build targets, dependency coordinates, and runtime selection for builds. Example:

```yaml
package:
  name: geometry
  version: 0.1.0
  authors:
    - "Team Chic"
  namespace: Geometry

targets:
  default: exe
  profiles:
    dev:
      optimize: false
    release:
      optimize: true

sources:
  - path: src
    namespace_prefix: Geometry

dependencies:
  math: "1.2.*"
  logging:
    path: ../logging
```

- **Project templates:** `chic init --template app [path] [--name <project>]` scaffolds a console application with `manifest.yaml` (`build.kind: exe`), `src/App.cl` (entrypoint with `Main(string[] args)`), `tests/AppTests.cl`, a docs stub, and optional CI workflow. Placeholders (`{{project_name}}`/`{{project_namespace}}`) default to the output directory name when `--name` is omitted.
- **Source layout:** Packages contain one or more source trees. By default `src/` is scanned recursively; additional roots can be declared in the `sources` array of `manifest.yaml`.
- **Namespace allowlist:** `package.friends` lists extra namespace prefixes this package may declare. File-scoped `@friend("...")` directives append to the same allowlist at parse time and are validated alongside manifest entries.
- **Dependency coordinates:** `dependencies` entries accept semver ranges (`>=1.2.0 <2.0.0`, `1.2.*`) and structured sources: `path`, `git` (with `rev`/`branch`/`tag`), or `registry`. Invalid ranges raise `PKG0201`; missing versions for registry entries raise `PKG0202`. Resolved graphs are recorded in `manifest.lock` alongside git commit pins.
- **Namespace to module mapping:**
  - File paths map to namespaces by combining the package prefix with directory segments. For example `src/geometry/point.cl` contributes to `namespace Geometry.Point;` by default.
  - Explicit `namespace` directives—either file-scoped (`namespace Geometry.Point;`) or block-scoped (`namespace Geometry.Point { ... }`)—override the inferred namespace but must stay within the package prefix unless `@friend` grants access. Nested block namespaces append their identifier segments to the active path, so `namespace Geometry { namespace Diagnostics { ... } }` contributes to `Geometry.Diagnostics`.
- **Visibility across packages:** `public` exposes symbols to dependent packages, `internal` keeps them within the current package, and `private` restricts to the current namespace block.
- **Module initialization:** Each compilation unit produces a module descriptor listing exported types, functions, and macros. During linking, module descriptors are merged according to namespace to form the final package surface.
- **Package imports:** `@cimport` binds to external headers, while `@package("name")` imports Chic packages declared under `dependencies`. Both participate in the same dependency graph resolved at build time and recorded in `manifest.lock`.
- **Resolver & restore:** `chic build` performs dependency restore unless explicitly disabled (`--no-restore` or `CHIC_NO_RESTORE`). Resolution honours semver ranges, git/path/registry sources, and offline flags (`CHIC_OFFLINE`/`CHIC_PACKAGE_OFFLINE`), caches checkouts/artifacts, and writes `manifest.lock` with pinned versions/commits. Conflicts, cycles, offline misses, and version mismatches raise `PKG2001`–`PKG2005`.
- **Package trimming:** After MIR lowering, the linker walks symbol references from the root package’s entrypoints, exports, and tests to retain only reachable dependency functions/exports. Root/stdlib modules are always preserved; unused dependency exports are pruned so artifacts ship only referenced package APIs.

## 16. AI-First Systems & Agent Features

*Status:* Draft. This section codifies the zero-hidden-cost AI capabilities Chic will offer while staying faithful to the LL(1) grammar and deterministic compilation guarantees.

### 16.1 First-Class Tensors & Shape System (P0)

- **Type surface:** `Tensor<T, const SH: Shape, L: Layout, M: MemSpace, const ALIGN: usize = 64>` is a first-class value type. `Shape` is an `N`-length array of `Dim` descriptors (`Dim<const LB: usize = 0, const UB: usize = usize::MAX>`). Layout and memory space are supplied through traits (`Layout`, `MemSpace`) so downstream libraries can extend them without compiler patches.
- **Symbolic dimensions:** Shapes may mix compile-time constants (`Dim<32>`) and symbolic dimensions (`Dim<1, 1024>`) that resolve at runtime. Symbolic bounds participate in shape checking; violations produce diagnostics with suggested repairs. Shape expressions remain LL(1) by restricting to identifiers and integer literals inside `Dim<...>`.
- **Ownership model:** Owning tensors allocate storage via `TensorAlloc`. Borrowed views (`TensorView`) reference slices, transposes, or reshaped regions without new allocations. Views carry their own stride metadata, and the borrow checker enforces that views cannot outlive the source tensor or introduce conflicting writes.
- **Zero-cost views:** Operations like `view`, `slice`, `permute`, and `reshape` lower to `TensorView` MIR ops. Layout metadata follows the view so agents can legally reorder or fuse kernels while respecting strides.
- **Interop:** `@extern` imports may accept tensors by raw pointer/stride pairs. The compiler guarantees layout compatibility when the Tensor’s `Layout` and `MemSpace` implement the `@extern` bridging traits documented in the FFI guide.
- **Diagnostics:** Shape mismatches report the symbolic and resolved forms and list legal conversions (`broadcast`, `reduce`, `expand`). Agents can consume the same data via `mir.json` (§16.12).

### 16.2 Differentiable Programming as a MIR Transform (P0)

- **Attribute:** `@diff(mode)` can be applied to free functions, methods, or `graph` blocks. Supported modes are `reverse` and `forward`. Applying the attribute emits a derivative companion with the suffix `$grad` (e.g. `loss$grad`) that takes `@grad` output tensors for differentiable parameters.
- **Pure MIR transform:** AD runs after semantic checks and before borrow checking. The transform introduces `AdjointAlloc`, `AdjointAccumulate`, `Checkpoint`, and `Remat` ops. No runtime tape or hidden allocation is permitted; checkpointing decisions are encoded in MIR and surfaced in `mir.json`.
- **Type rules:** Tensors, scalars, tuples, and structs containing differentiable fields can be differentiated. Effects (`random`, `network`, `cancel`) are prohibited inside `@diff` bodies unless the mode and downstream tooling explicitly support them. The compiler rejects AD across opaque control-flow constructs (e.g. `panic`) to keep gradients deterministic.
- **User controls:** Statements may be tagged with `@checkpoint` or `@remat` to influence the planner. The transform preserves these hints and emits diagnostics if they cannot be honoured.

### 16.3 Quantized & Mixed-Precision Numerics (P0)

- **Scalar families:** Built-in identifiers `bf16`, `fp8_e4m3`, `fp8_e5m2`, `i4`, and `i2` complement the existing scalar set. The lexer treats them as ordinary identifiers so the grammar stays LL(1).
- **Quantized wrappers:** `Q<TInner, Policy>` encodes quantization policy at the type level. Policies include `PerTensor<S>`, `PerChannel<const AXIS: usize, S>`, and user-defined policy structs implementing `QuantPolicy`.
- **Rounding & saturation:** Functions can request deterministic rounding/saturation via attributes like `@round(nearest_even)` and `@saturate`. These attributes lower to explicit MIR conversions with enumerated rounding modes. Illegal combinations (e.g. `@round(bankers)` on integer sources) trigger diagnostics.
- **Arithmetic APIs:** Library intrinsics (`qgemm`, `qconv`, `qcast`) accept quantized tensors and emit MIR sequences that propagate scales explicitly. No implicit widening occurs; programmers must insert conversions, enabling agents to reason about numerical stability.

### 16.4 Deterministic PRNG & Randomness Effect (P0)

- **Core type:** `struct RNG { state: u128; }` with `split(rng: ref RNG) -> (RNG, RNG)` and `advance(rng: ref RNG, n: u64)` built-ins. The generator is counter-based and portable across targets.
- **Effects:** Any function that allocates or consumes random bits declares `effects(random)`. The compiler threads RNG handles through async frames and lambdas so replay tooling can deterministically reproduce runs.
- **MIR:** `Rand`, `SplitRng`, and `AdvanceRng` ops include provenance metadata (source span, lexical path) and record which tensor or scalar received the random values. Schedulers and agents use this to align seeds with compute graphs (§16.6).
- **Tooling:** `chic seed --from-run <trace>` rehydrates runs by reading `perf.json` tracepoints that list RNG usage in order.

### 16.5 Accelerator & Stream Model (P0)

- **Devices & memory spaces:** `MemSpace` implementations denote host (`Host`, `PinnedHost`), unified (`Unified`), or device memory (`Gpu<const ID: u16>`, `Npu<const ID: u16>`). Borrowing rules treat memory space as part of the type identity, preventing accidental host/device aliasing.
- **Streams & events:** `Stream<M>` and `Event<M>` are linear capabilities. `memcpy_async`, `record_event`, and `wait(event)` form the primitive API. Enqueue operations require exclusive borrow access to the stream to prevent double submission race conditions.
- **Async integration:** `@streamed async fn` indicates the function expects a stream parameter. Awaiting a compute future implicitly waits on its completion event, but explicit waits remain available. The borrow checker verifies that tensors outlive queued work until the corresponding `Event` is awaited.
- **Pinned memory:** Capturing host allocations across async boundaries requires either `PinnedHost` tensors or `@pinned` annotations on stack locals. The compiler enforces this so DMA transfers remain valid.

### 16.6 Static Compute Graphs & Scheduling DSL (P0)

- **Graph syntax:** `graph Identifier { ... }` defines a static dataflow graph. Nodes reference Chic functions or intrinsics; edges carry typed tensors. Graphs are compile-time entities—no runtime graph construction is allowed.
- **Schedules:** `schedule Identifier { ... }` attaches transform directives to graph nodes: `tile`, `fuse`, `vectorize`, `unroll`, `place`, `memory_plan`. Schedules are validated against graph metadata to ensure transformations keep shapes/layouts compatible.
- **Profiles:** `@use_schedule(GraphName, profile = "...")` binds a graph to a precomputed profile artifact. Profiles capture tuned parameters (tile sizes, launch dimensions) and are hashed to guarantee deterministic builds. Missing or mismatched profiles emit build-time errors with suggested remediation (`chic schedule tune`).
- **MIR lowering:** Graphs lower to region-scoped `TensorAlloc`, `EnqueueKernel`, and `RecordEvent` sequences. Schedules influence loop nests and kernel launch metadata but never introduce runtime code generation.

### 16.7 Probabilistic Programming Effects (P1)

- **Traits:** `trait Dist<T>` exposes `sample(rng: ref RNG) effects(random, measure) -> T` and `logpdf(value: in T) -> f64`.
- **Surface operations:** `sample(dist, rng)` and `observe(dist, value)` are expressions. `observe` lowers to `Observe` MIR ops that accumulate log-probabilities. `logprob_scope` blocks allow users to checkpoint the accumulated measure.
- **Effects:** `effects(measure)` is mandatory for functions that create or mutate log-probability state. Higher-order inference engines consume the same MIR to drive SVI/MCMC algorithms deterministically.

### 16.8 Collectives & Distributed Actors (P1)

- **Collectives:** The standard library exposes `Collective::allreduce`, `Collective::broadcast`, and `Collective::pshard`, each declared with `effects(network)`. Group membership and topology are part of the type so mismatched devices are rejected at compile time.
- **Distributed actors:** `actor` blocks define RPC-like endpoints with typed messages. Actor handles carry capability tokens; only holders can send. Serialization is deterministic and versioned, enabling offline replay.
- **MIR:** `Collective`, `ActorSend`, and `ActorRecv` ops encode the operation, group descriptor, and effect metadata. The backend ensures consistent ordering and error handling across devices.

### 16.9 Region/Arena Memory Planning (P1)

- **Syntax:** `region name { ... }` introduces a lexical region. `alloc_in<name>(expr)` allocates buffers tied to that region.
- **Lifetime rules:** Values allocated in a region cannot escape by move or borrow. The drop order is deterministic—exiting the block frees all allocations en masse. Region-backed containers reject new allocations after teardown, surfacing allocation failures instead of touching freed memory.
- **Planner integration:** Graph schedules can request `memory_plan(region: "...")`. The compiler then emits allocation plans that reuse buffers when lifetimes do not overlap. Plans are exported alongside `mir.json` so agents can reason about peak memory.
- **Tooling:** `Region.Telemetry` exposes alloc/zeroed/free counters for profiling, and `Vec.NewIn/WithCapacityIn` route container allocations through explicit regions.

### 16.10 Structured Concurrency & Cancellation (P1)

- **Primitives:** `cancel_scope { ... }`, `spawn(expr)`, `await_any(tasks)`, `.on_timeout(duration)`, and `.cancel_rest()` provide structured concurrency with deterministic cancellation.
- **Effects:** Initiating cancellation requires `effects(cancel)`. Functions that merely respond to cancellation tokens do not.
- **MIR:** New ops (`ScopeBegin`, `ScopeEnd`, `Spawn`, `AwaitAny`, `InstallTimeout`, `CancelToken`) capture scope structure. Borrow checking ensures all tasks either finish or are cancelled before leaving the scope, preventing detached work.
- **Diagnostics:** Forgetting to await or cancel a spawned task is an error. The compiler points to the spawn site and the enclosing scope.

### 16.11 GPU Kernel Formalisation (P1)

- **Kernel declaration:** `@gpu_target(cuda)` or `@gpu_target(spirv)` annotates `kernel` functions. Kernels may only use supported value types (scalars, tensors, POD structs).
- **Built-ins:** Read-only identifiers `blockIdx`, `blockDim`, `threadIdx`, `gridDim`, `laneId`, and `warpSize` are available inside kernels. Shared memory is requested via `shared<T>(count)` which lowers to `SharedAlloc<T>` MIR ops.
- **Barriers:** `barrier()` enforces intra-block synchronization. Additional scoped barriers (`barrier_async`) are planned once backends grow support.
- **Host launches:** Launching a kernel produces an `EnqueueKernel` MIR op linked to a stream (§16.5). Parameter ownership rules are checked at compile time.

### 16.12 Agent-Grade Compiler Introspection (P0)

- **Structured artifacts:** `chic build --emit mir.json,hints.json,perf.json` produces machine-readable MIR dumps, diagnostics with suggested fixes, and deterministic performance traces. The schema is versioned and documented in `docs/tooling/mir_json.md`.
- **Why diagnostics:** `chic explain --why <symbol>` returns a cause graph that names violating borrows, shape mismatches, or effect leaks along with legal fix-its. The same data powers IDE quick-fixes and agent workflows.
- **Explain MIR:** `chic explain-mir <symbol>` renders SSA graphs annotated with ownership, layout, and stream metadata. Agents rely on these to validate schedule proposals before editing source code.

### 16.13 Typed Program Holes & Obligations (P2)

- **Syntax:** `??Type` produces a placeholder value. Optional `where` clauses attach obligations (`??bool where must_call(LogAudit)`).
- **Obligation reporting:** Compilation fails with a structured obligation list. Each entry enumerates viable replacements (existing functions, constructors, constants) and constraints that must be satisfied (traits, shape, effect). The report is emitted to `hints.json`.
- **Workflow:** Agents fill holes by selecting from the obligation list, guaranteeing that completions honour type/effect requirements. Holes do not emit runtime code; they are purely compile-time constructs.

### 16.14 Contracts & Refinements for Shapes/Effects (P2)

- **Syntax:** `requires(condition);` and `ensures(condition);` statements may appear at the start/end of functions. Conditions can reference parameter shapes (`a.shape[0]`), tensor layouts, and effect usage (`effects.contains(random)`).
- **Semantics:** Contracts are evaluated at compile time when possible. Otherwise, debug builds insert runtime `Assert` MIR ops; release builds erase them once proven or assume success.
- **Diagnostics:** Failing to prove a contract triggers a compile-time warning prompting developers (or agents) to supply proofs or adjust code. Contracts feed into schedule validation and AD transforms to catch illegal shape manipulations early.

### 16.15 Deterministic Tracing & Cost Models (P2)

- **Annotations:** `@trace("label", level = Perf)` attaches MIR tracepoints; `@cost(cpu = 42_µs, mem = 1_048_576)` embeds static cost estimates.
- **Artifacts:** `perf.json` records measured latencies, memory footprints, and RNG usage per MIR instruction. Traces are keyed by stable MIR IDs so regressions are comparable across builds.
- **Usage:** The profiler and scheduling tools consume trace data to rank hotspots and evaluate candidate transformations. Agents can suggest optimisations only when they keep within declared budgets.
- **Runtime:** The compiler lowers `@trace`/`@cost` into `chic_rt_trace_{enter,exit,flush}` hooks with deterministic clocks (`CHIC_TRACE_FAKE_CLOCK=1` for tests). Native startup and the WASM executor flush `perf.json` on completion; `CHIC_TRACE_OUTPUT`/`CHIC_TRACE_PROFILE`/`CHIC_TRACE_TARGET` customise emission.
- **Summary:** `perf.json` embeds `summary` (wall-clock, CPU, IO block counts, allocation counters, sampling interval) and emits sidecars `perf.summary.json`/`perf.folded` derived from the `CHIC_TRACE_OUTPUT` base path.
- **Auto-instrumentation:** Setting `CHIC_PROFILE_AUTO_TRACE=1` forces tracepoints onto all lowered functions and tests and enables sampling (default 1 ms; override with `CHIC_TRACE_SAMPLE_MS`/`CHIC_TRACE_SAMPLE_HZ`). Folded stacks include `[idle]` samples when no tracepoints are active to keep flamegraphs wall-time aligned.
- **CLI:** `chic run|test --profile [--profile-out <path> --profile-sample-ms <ms> --profile-flamegraph]` and `chic profile <inputs>` build with auto instrumentation, capture perf/summary/folded outputs, and optionally render `perf.svg` flamegraphs from the folded stacks.
- **Tooling:** `chic perf [--json]` (see `docs/tooling/perf_reporting.md`) summarises overruns and baseline regressions from `perf.json`, aligning budgets declared in metadata with observed metrics.

### 16.16 MLIR/StableHLO AOT Bridge (P2)

- **Import/export:** `chic graph import stablehlo <file>` produces Chic `graph`/`schedule` definitions plus profile sidecars. `chic graph export stablehlo <symbol>` performs the reverse. Both operations are offline and deterministic.
- **Versioning:** Sidecar files contain the StableHLO dialect version and a hash of the Chic schedule/profile. Builds fail if the sidecar version drifts to keep reproducibility intact.
- **No JIT:** Conversions happen at build time only. Runtime code never parses StableHLO; it executes the already-lowered MIR emitted during compilation.

---

### Static Items

### Static members

Static items allocate storage with program-wide lifetime. They complement constants (`const`) by permitting a single shared location that code can borrow or mutate.

- **Syntax:**\
  `attributes? visibility? static ('const' | 'mut') Type IDENTIFIER (= initializer)? { , IDENTIFIER (= initializer)? }? ;`\
  `static const` is the default form and may be spelled simply as `static const`. `static mut` declares a mutable slot. Initialisers are optional; missing initialisers are zeroed by the backends.
- **Lifetime:** Values stored in a static acquire the `'static` lifetime. Any references obtained from the static are treated as `'static` by the borrow checker, so the contained type must not capture non-static borrows.
- **Initialisers:** Static initialisers are evaluated at compile time. They must satisfy the constant-evaluation rules (no heap allocations, no user code execution, no side-effects that rely on runtime state). Types that require drop glue are rejected because Chic does not run user-defined destructors at shutdown. Missing initialisers produce a zero-initialised payload.
- **Visibility & linkage:** `public`, `internal`, and `private` behave the same as other namespace items; other modifiers are currently rejected. Code generation emits one symbol per static using the target backend’s global storage model. Separate compilation units receive distinct mangled names so linkers can merge or reference statics correctly.
- **Mutability & safety:** `static const` is read-only and may be accessed without `unsafe`. `static mut` stores allow interior mutation but require `unsafe` blocks at each access site. Users must supply their own synchronisation primitives if the static is shared across threads.
- **Borrow checking:** The borrow checker treats reads/writes of `static mut` as raw pointer operations—no automatic tracking of aliasing occurs. Shared (`in`) borrows from immutable statics behave like `'static` references and participate in lifetime inference normally.
- **Runtime considerations:** The LLVM backend emits either data or constant sections with the requested alignment. The WASM backend represents statics as linear-memory segments. Runtime/executor support currently assumes eager initialisation at module load; reflection sidecars will be extended to list statics in a follow-on milestone.

### Linting and Code Standards

- Impact categories: `style` (warn), `correctness` (error), `perf` (warn), `pedantic` (allow). Rule defaults may tighten categories (e.g., `dead_code` is an error).
- Configuration: `lint.yaml`/`chiclint.yaml` or `manifest.yaml` `lint:` sections are discovered from the input directory upward (root-first), supporting `extends:` chains and `CHIC_LINT_CONFIG` overrides. Levels accept `allow|warn|error|deny|off`.
- Suppression: `@allow(<lint>|<category>|all)` scopes to namespaces/containers/functions/parameters; nearest scope wins. Attribute aliases such as `@dead_code` or `@unused_param` are equivalent to `@allow(<lint>)`.
- Available lints: `LINT001 dead_code` (error) flags unreachable user-defined functions/constructors/testcases; `LINT002 unused_param` (warn) flags unused parameters and suggests prefixing them with `_`.
- CLI: `chic lint` fails on any error-level lint; `chic check/build/test` always run lints and respect `CHIC_DIAGNOSTICS_FATAL=1` to turn diagnostics (including lints) into hard failures. See `docs/cli/linting.md` for the YAML schema and examples.

### IConvertible and culture-aware conversions

- `Std.Globalization.IFormatProvider` now supplies the culture id string consumed by numeric and date/time formatting and parsing; `null` maps to the invariant culture.
- `IConvertible` exposes `ToXxx(IFormatProvider)` for all primitives (bool/char, signed/unsigned integers including `Int128`/`UInt128` and pointer-sized types, floating point including `Float128`, `decimal`, `DateTime`, and `string`) plus `ToType(Type, IFormatProvider)`.
- Invalid conversions throw `InvalidCastException`, range failures throw `OverflowException`, and parse failures throw `FormatException`, aligning with the rest of the C#-style exception model.
- String and numeric conversions respect the existing two-string formatting model and the date/time culture resolver so `"fr-FR"`/`"en-US"`/`"invariant"` stay consistent across parse/format entry points.

### First-class span types

- The compiler recognises `Std.Span.Span<T>` and `Std.Span.ReadOnlySpan<T>` as language-defined spans using their fully qualified names.
- Built-in implicit conversions:
  - `T[]` → `Span<T>` for single-dimensional arrays.
  - `T[]` → `ReadOnlySpan<U>` when `T` is implicitly reference-convertible to `U`.
  - `Span<T>` → `ReadOnlySpan<U>` and `ReadOnlySpan<T>` → `ReadOnlySpan<U>` when the element layouts match and `T` reference-converts to `U`.
  - `string` → `ReadOnlySpan<byte>` as the UTF-8 view.
- Explicit conversions include the implicit set plus `T[]` → `Span<U>`/`ReadOnlySpan<U>` when only explicit reference conversions exist between the element types.
- Span conversions participate in overload resolution, generic inference, and extension-method receiver matching. User-defined conversions between the same pairs are ignored while the language-defined span conversions are in scope.
