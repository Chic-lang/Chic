# Chic MIR Design

This document captures the mid-level intermediate representation (MIR) for Chic. MIR sits between HIR (typed, desugared surface syntax) and the backend IRs (LLVM/WASM). It makes ownership, borrowing, and control flow explicit so we can borrow-check, optimize, and generate code efficiently.

The contents below distill the design agreement and the Chic language specification sections that touch MIR semantics. Keep this document authoritative and update it alongside implementation changes.

---

## 1. Pipeline Context

```
Source (.cl)
  └── Attribute/Macro Expansion (@derive, @memoize, @extern, …)
      └── AST (C#-like surface: namespaces, structs, classes, optional OOP)
          └── HIR (typed + desugared statements: using/lock/fixed/await lowered to core forms)
              └── Monomorphization (per concrete generic instantiation)
                  └── MIR (this document: ownership-aware CFG)
                      ├── Borrow Checker (non-lexical lifetimes; many in OR one ref rule)
                      ├── MIR Optimizations (copy elision, const-prop, inlining hints)
                      └── Backend IR (SSA)
                           ├── LLVM (release profile; LTO/PGO)
                           ├── WASM (dev profile; fast iteration)
                           ├── PTX (NVIDIA GPU kernels; assembled into cubins at link/run time)
                           ├── Intel Xe (SPIR-V/VISA modules for Level Zero runtime)
                           └── Other GPU backends (future: Metal IR, Vulkan SPIR-V)
```

Key points:
- Attribute/macro system runs before type checking when possible, and during/after HIR build for attributes requiring type information.
- Monomorphization happens before MIR generation, so MIR operates on concrete types.
- Borrow checking and optimizations operate on MIR before lowering to backend IR.

---

## 2. Ownership & Borrowing Invariants

Surface rules (from the spec):
- `let` introduces immutable bindings; `var` introduces mutable bindings.
- Passing a value by value moves ownership (source becomes invalid until re-assigned).
- `in` parameters are read-only borrows (aliasable).
- `ref` parameters are unique, mutable borrows (exclusive).
- `out` parameters are unique borrows into uninitialised storage; they must be assigned exactly once before return.
- Methods use `in this` / `ref this` receivers for read/write semantics.
- `using` and deterministic destruction: `dispose(ref this)` runs when a value’s
  lifetime ends. `using` is sugar scheduling a drop at scope exit and the drop
  lowering pass materialises explicit `Deinit`/`Drop` statements on every exit
  path.
- `await` cannot be crossed by a live `ref` borrow unless the referent is pinned (e.g., `fixed` or `Pin<T>`). `lock` forbids `await` inside.

MIR enforces at any program point:
- Many shared (`in`) borrows OR one unique (`ref`) borrow of the same location.
- Moves invalidate the source until reinitialised.
- Returning a borrow is only legal if the referent outlives the function.
- `fixed`/pinning forbids moves of the pinned place within the region.

---

## 3. MIR Goals

- Explicit control flow graph (CFG): basic blocks with statements and terminators.
- Explicit storage: `StorageLive`/`StorageDead` for local lifetimes.
- Explicit ownership ops: `Move`, `Copy`, `Borrow`, `Drop`. The builder may also emit
  transient `DeferDrop` markers, but the drop-lowering pass converts those into explicit
  `Deinit`/`Drop` statements before MIR leaves lowering.
- Borrow-friendly: references, region variables, and constraints are easily derived.
- Suitable for dataflow analysis, borrow checking, and later optimization passes.
- Amenable to interpretation (for testing) and eventual backend lowering.

---

## 4. Core Data Structures

```text
FnBody {
  sig: FnSig
  locals: Vec<LocalDecl>
  blocks: Vec<BasicBlock> // blocks[0] is entry
}

FnSig {
  params: Vec<Ty>
  ret: Ty
  abi: Abi // e.g., chic internal, extern "C"
}

LocalDecl {
  ty: Ty
  name: Option<String> // debug info
  mutable: bool
  is_pinned: bool      // true for locals introduced by fixed-guards, `@pin`, or `Pin<T>` annotations
}

BasicBlock {
  stmts: Vec<Statement>
  term: Terminator
}

Place {
  base: Local
  projection: Vec<ProjectionElem>
}

ProjectionElem =
  Field(u32)      // struct/tuple fields
  | FieldNamed(String) // bootstrapper: member access before field order is known
  | Index(Local)  // array/vector indexing
  | ConstantIndex { offset: usize, length: usize, from_end: bool }
  | Deref
  | Downcast { variant: VariantIdx }

Operand = Copy(Place) | Move(Place) | Const(ConstVal)

Rvalue =
  Use(Operand)
  | Unary(UnOp, Operand)
  | Binary(BinOp, Operand, Operand)
  | Aggregate { kind: AggregateKind, fields: Vec<Operand> }
  | AddressOf { mutability: MutKind, place: Place }
  | Ref { mutability: MutKind, place: Place } // sugar; usually lowered to Borrow + Copy
  | Len(Place)
  | Cast { kind: CastKind, operand: Operand, target: Ty }
```

### Statements

```text
Statement =
  Assign { place: Place, value: Rvalue }
  | StorageLive(Local)
  | StorageDead(Local)
  | Deinit(Place)                       // explicit destructors via `dispose(ref this)`
  | Drop { place: Place, target: Block, unwind: Option<Block> }
  | Borrow { borrow_id: BorrowId, kind: BorrowKind, place: Place, region: RegionVar }
  | Retag { place: Place }              // optional, for alias tracking
  | DeferDrop { place: Place }          // builder-only marker for drop scheduling
  | Assert { cond: Operand, expected: bool, msg: String, target: Block, cleanup: Option<Block> }
  | Nop
```

> **Bootstrap note:** The Rust bootstrapper now materialises these structures in `src/mir/data.rs`.
> Until full expression lowering lands, we wrap unsupported constructs in `PendingRvalue`,
> `PendingStatement`, and `PendingOperand`. They preserve the original source text + span so
> later passes can either emit diagnostics or finish the lowering once the interpreter/codegen
> layers are ready.

### Terminators

```text
Terminator =
  Goto(Block)
  | SwitchInt { discr: Operand, targets: Vec<(i128, Block)>, otherwise: Block }
  | Match { value: Place, arms: Vec<MatchArm>, otherwise: Block }
  | Return
  | Call { func: Operand, args: Vec<Operand>, dest: Option<Place>, target: Block, unwind: Option<Block> }
  | Yield { value: Operand, resume: Block, drop: Block }   // iterators
  | Await { future: Place, resume: Block, drop: Block }    // async-await
  | Panic                                                  // abort path (no_std)
  | Unreachable
```

Bootstrap shortcuts:

- Until HIR records explicit field order, the Rust bootstrapper lowers `obj.Member`
  into a place projection using `FieldNamed(member)` so later passes can resolve
  the actual offset.
- Subscript expressions evaluate the index into a temporary local when needed and
  append a `ProjectionElem::Index(local)` projection before the backend understands
  concrete layouts.
- Assignments targeting expressions that look like static members (e.g., `Logger.Level`)
  stay pending so the later type checker can resolve whether the root identifier names
  a type or a missing variable; the heuristic treats identifiers beginning with an
  uppercase character as “type-like”.
- Instance method calls (`obj.Method(args)`) evaluate the receiver expression and
  prepend it to the call argument list. When the receiver expression cannot be
  resolved to a local (e.g., `Type.Method()`), the callee remains pending and the
  argument list stays empty so static/member qualification can be disambiguated
  later.

`Match` models high-level pattern dispatch and preserves the arm ordering from the
source `switch`. Each `MatchArm` carries a `Pattern` tree, any bindings introduced by
that pattern, plus an optional guard:

```text
MatchArm = { pattern: Pattern, guard: Option<MatchGuard>, bindings: Vec<PatternBinding>, target: Block }
MatchGuard = { expr: String, span: Span?, parsed: bool }

PatternBinding = {
  name: String,
  local: Local,
  projection: Vec<PatternProjectionElem>,
  span: Span?,
  mutability: PatternBindingMutability,
  mode: PatternBindingMode
}

PatternBindingMode = Value | In | Ref | RefReadonly | Move
PatternBindingMutability = Immutable | Mutable

PatternProjectionElem =
  Variant { path: Vec<String>, variant: String }
  | FieldNamed(String)
  | FieldIndex(u32)

Pattern =
  Wildcard
  | Literal(Const)
  | Binding(BindingPattern)
  | Tuple(Vec<Pattern>)
  | Struct { path: Vec<String>, fields: Vec<PatternField> }
  | Enum { path: Vec<String>, variant: String, fields: VariantPatternFields }

BindingPattern = {
  name: String,
  mutability: PatternBindingMutability,
  mode: PatternBindingMode
}

VariantPatternFields = Unit | Tuple(Vec<Pattern>) | Struct(Vec<PatternField>)
PatternField = { name: String, pattern: Pattern }
```

The bootstrapper emits `Match` when a `switch` contains destructuring patterns, tuple
payloads, or other cases that cannot be represented as a simple `SwitchInt`. Guards
are stored as raw source text today; once the expression lowering hooks are wired up
they will become MIR operands. Literal-only switches continue to lower to `SwitchInt`
for backend compatibility. `PatternBinding` records the locals created for pattern
variables together with their binding semantics (`value`/`move`) and the structured
projection needed to materialise the values once type layout information is available.
When enum layout metadata is present, variant
projections are emitted as `Downcast { variant }` terminators and the subsequent field
accesses use `Field(idx)` rather than placeholder names; the bootstrapper falls back to
`FieldNamed` only when layout data has not been discovered yet.

Binding modifiers now translate into differentiated MIR:

- `Value` (including `let/var` without modifiers) copies the projection when possible.
- `Move` always emits `Operand::Move`, so reusing the scrutinee later reports a move error.

The binding block executes before the optional guard, keeping the move alive for
the guard evaluation and the case body. When the guard fails, the compiler releases the
binding before evaluating the next case so the loan checker sees the same behaviour as if
the program had performed the move manually.

### Type Layout Metadata

`MirModule` now carries a `TypeLayoutTable` alongside the lowered functions. The table
captures the field/variant ordering for user-defined structs, classes, and enums so that
backends can calculate byte offsets without re-walking the AST. The bootstrapper stores
the following records (see `src/mir/data.rs`):

```text
TypeLayoutTable = HashMap<String, TypeLayout>

TypeLayout =
  Struct(StructLayout)
  | Class(StructLayout)
  | Enum(EnumLayout)
  | Union(UnionLayout)

StructLayout = {
    name: String,
    fields: Vec<FieldLayout>,
    repr: TypeRepr,
    size: Option<usize>,
    align: Option<usize>,
    auto_traits: AutoTraitSet,
    overrides: AutoTraitOverride,
}
EnumLayout   = { name: String, variants: Vec<EnumVariantLayout>, repr: TypeRepr, size: Option<usize>, align: Option<usize>, auto_traits: AutoTraitSet, overrides: AutoTraitOverride }
UnionLayout  = { name: String, views: Vec<UnionFieldLayout>, repr: TypeRepr, size: Option<usize>, align: Option<usize>, auto_traits: AutoTraitSet, overrides: AutoTraitOverride }

AutoTraitSet      = { thread_safe: AutoTraitStatus, shareable: AutoTraitStatus }
AutoTraitStatus   = Yes | No | Unknown
AutoTraitOverride = { thread_safe: Option<bool>, shareable: Option<bool> }

FieldLayout        = { name: String, ty: Ty, index: u32, offset: Option<usize>, span: Span? }
EnumVariantLayout  = { name: String, index: u32, fields: Vec<FieldLayout> }
UnionFieldLayout   = { name: String, ty: Ty, index: u32, mode: UnionFieldMode, span: Span? }
UnionFieldMode     = Value | Readonly | Ref | RefReadonly
TypeRepr           = Default | C | Packed(u32?)
```

Offsets are computed eagerly for primitives and any type whose layout metadata is already
available; otherwise we leave them as `None` so later passes can finish the calculation once
the full type graph is known. Enum layouts currently assume a 32-bit discriminant and record
the maximum variant size discovered during lowering. Union layouts track all declared views
and compute their size/align contributions individually. Every union view is assigned a stable
index so that MIR projections (`UnionField { index }`) are deterministic, and the borrow
checker can record which view is currently active for each union-typed local.

When lowering expressions, the builder rewrites field/member access against a union into a
`ProjectionElem::UnionField { index, name }`. Pattern bindings emit the same projection
segments so that destructuring a union view activates the correct storage slice. The borrow
checker consults the layout table to map `Ty::Named` values to their union metadata and
maintains an `active_view` slot per union-typed local; assigning through one view deactivates
previous loans and prevents reads from non-active views until the union is reassigned.

As part of module lowering we run a fix-point pass (`TypeLayoutTable::finalize_auto_traits`) that
derives `ThreadSafe`/`Shareable` statuses for every nominal type. Overrides supplied via
`@thread_safe`, `@not_shareable`, etc., seed the computation; the pass walks each field/variant and
propagates `No`/`Unknown` states outward until a stable assignment is reached. The resulting
`AutoTraitSet` is cached inside the layout entry so later stages (type checking, borrow checking,
codegen) can query trait availability without re-evaluating the type graph. Built-in primitives and
`std.sync` guard types are treated as trait-safe roots so user code can rely on them without extra
annotations. Primitive spellings stay identifier-like at the lexer layer; the primitive registry
decides their meaning during type binding, so adding aliases via `@primitive(...)` or registry
hooks never requires keyword or parser tweaks.

Module lowering now executes through a staged `LoweringPipeline`. Each invocation resets context
state, then runs the following stages in order:

1. `prepare_context` — rebuilds the `SymbolIndex`, `UsingResolver`, clears exports, and resets
   cache counters.
2. `module_attributes` — dispatches attribute gathering to the attribute collector module and
   records the DI manifest.
3. `collect_type_layouts` — walks every item eagerly so later cache hits still have refreshed
   layouts.
4. `evaluate_constants` — runs constant evaluation before we materialise MIR bodies.
5. `queue_setup` and `lower_root_items` — delegate item dispatch to `module_lowering::queue`.
6. `finalise_auto_traits` — calls the auto-trait fix point described above.

Each stage records a stopwatch sample that lands in `LoweringResult::pass_metrics`. The `cargo xtask
budget` runner executes `tests/lowering_budget.rs`, captures those samples, and compares the
resulting stage list/counts against `coverage/metrics/pass_budgets.json`. Adding, removing, or
running stages multiple times requires updating the budget file (via `--update`) so reviewers can
see the explicit opt-in.

The queue layer drives item lowering and now maintains a memoised cache. Fingerprints are computed
from a `blake3` digest of the item AST plus its namespace/unit, so unchanged items across
incremental runs are replayed straight from the cache without re-running lowering passes. Cached
entries store mir bodies, diagnostics, type constraints, and the interner snapshot produced during
the original lowering so the replay step can reconstruct identical state. Hit/miss counters are
exposed through `LoweringResult::cache_metrics`, and the pipeline emits `tracing` spans for each
stage and cache event (`lowering::cache` target) to make incremental behaviour visible during
debug builds.

Layout registration now lives under `module_lowering/layout/{mod,structs,enums,unions,mmio,auto_traits}.rs`,
with class-specific lowering and operator registration in `module_lowering/classes.rs`, and
interface/extension lowering in `module_lowering/interfaces.rs`. The façade in `driver.rs` is reduced to orchestration helpers and signature validation, keeping responsibility boundaries narrow and module sizes
well under the 600 LOC ceiling.

`using` statements now lower to explicit resource slots: the builder emits `StorageLive` and
`Assign` for each bound resource, records a deferred drop in reverse order, and finally inserts
`StorageDead` instructions on the lexical fallthrough path. During drop lowering those markers
are expanded into concrete `Deinit` and `Drop` statements so every exit path performs the
required cleanup.

`unsafe` blocks introduce two lightweight statements, `EnterUnsafe` and `ExitUnsafe`, that bracket
the lowered body. The borrow checker tracks the current unsafe depth while traversing MIR so that
future work on raw pointer escapes has precise scope information even in the presence of complex
control flow.

When a guard is present, the bootstrapper creates a dedicated basic block that evaluates the
guard expression and branches either to the arm body or to the continuation block that handles
the remaining match arms. Guards therefore participate in ordinary MIR control flow and no
longer rely solely on raw source text at the terminator level.

### Borrow Kind

```text
BorrowKind = Shared | Unique | Raw
```

- Shared: read-only (`in`), aliasable.
- Unique: mutable (`ref`), exclusive.
- Raw: unsafe borrows (unsafe blocks only; not subject to borrow rules).

### Regions

- Each borrow references a `RegionVar`, solved by the borrow checker using non-lexical lifetime (NLL) constraints.
- Regions do not appear in generated code; they exist for analysis only.

### Types (partial Slice-1 view)

```text
Ty =
  Void
  | Bool
  | I32 | I64 | I128
  | U32 | U64 | U128
  | F32 | F64
  | Ref { kind: BorrowKind, ty: Box<Ty> }   // MIR-level references
  | RawPtr { mutability: MutKind, ty: Box<Ty> }
  | Struct(StructId)
  | Enum(EnumId)
  | Array { element: Box<Ty>, len: usize }
  | Task { output: Box<Ty> }  // async futures
  | ...
```

---

## 5. Example MIR

### SumTo Example

```chic
public static int SumTo(int n)
{
    var s = 0;
    for (var i = 0; i <= n; i += 1) { s += i; }
    return s;
}
```

MIR Sketch:

```text
fn SumTo(n: i32) -> i32 {
  locals: _0: i32 (ret), _1: i32 (s), _2: i32 (i), _t: bool
  bb0:
    StorageLive(_1)
    _1 = 0
    StorageLive(_2)
    _2 = 0
    goto bb1
  bb1:
    _t = _2 <= Copy(n)
    Assert { cond: Copy(_t), expected: true, msg: "for condition", target: bb2 }
  bb2:
    SwitchInt { discr: Copy(_t), targets: [(1, bb3)], otherwise: bb5 }
  bb3:
    _1 = _1 + _2
    _2 = _2 + 1
    goto bb1
  bb5:
    _0 = Move(_1)
    StorageDead(_2)
    StorageDead(_1)
    Return
}
```

Notes:
- Scalars (`i32`) are `Copy`; `Move(_1)` is effectively a copy here but sets the stage for non-Copy types.
- `Assert` is used for debug builds; may be elided in release unless explicitly requested.

### Using/Lock/Fixed Lowering

**Using:**
```chic
using var h = Open();
if (h.Ready) return 1;
return 0;
```

Core MIR:
```text
bb0:
  StorageLive(_h)
  _h = Call Open() -> bb1
bb1:
  DeferDrop(_h)              // rewritten into Deinit/Drop during drop lowering
  _ready = Copy(_h).Ready
  SwitchInt { discr: Copy(_ready), targets: [(1, bb2)], otherwise: bb3 }
bb2:
  Drop { place: Place(_h), target: bb4, unwind: None }
bb4:
  _0 = 1
  Return
bb3:
  _0 = 0
  Drop { place: Place(_h), target: bb5, unwind: None }
bb5:
  Return
```

**Lock:**
```chic
StorageLive(_guard)
_guard = Call mutex.lock() -> bb1
bb1:
  DeferDrop(_guard)          // becomes explicit Deinit/Drop before returning
  Call Mutate()
  StorageDead(_guard)
  Return
```

**Fixed:**
```chic
StorageLive(_p)
StorageLive(_guard0)
Borrow { kind: Unique, place: buf.Bytes }
_guard0 = Borrow(buf.Bytes)
DeferDrop(_guard0)          // becomes explicit Deinit/Drop before returning
_p = AddressOf(Mutable, buf.Bytes)
Call Use(_p)
StorageDead(_p)
StorageDead(_guard0)
Return
```

- The guard local created by `lock` is deferred with `DeferDrop`. Drop lowering rewrites the
  marker into an explicit `Deinit`/`Drop` sequence on every exit path.
- `fixed` introduces a unique borrow for the pinned place, stores it in a guard local marked `is_pinned`, and materialises an address-of assignment into the pointer binding. The borrow checker sees both the loan and the guard lifetime, preventing moves of the pinned storage while the guard is active.

### Foreach Lowering

`foreach` statements expand into a structured CFG that mirrors the enumerator protocol while keeping borrow scopes explicit:

1. The sequence expression is evaluated once and stored in a synthetic local `__foreach_seq_n`. The builder then calls `GetEnumerator()` on that local, captures the result in `__foreach_enum_n`, marks the enumerator `StorageLive`, and records a deferred drop so its `dispose` runs along every exit path.
2. The builder appends five blocks:
   - **Condition:** calls `MoveNext()` on the enumerator and branches to the prepare block when it returns `true`, or to the exit block otherwise.
   - **Prepare:** runs `StorageLive(_item)`, then binds the iteration variable to `enumerator.Current`. Value iteration copies the returned value, whereas `in`/`ref readonly` create shared borrow statements and `ref` produces a unique borrow so the element can be mutated in place.
   - **Body:** lowers the user statements with the loop context configured so `continue` points at the cleanup block and `break` points at a dedicated break-cleanup block.
   - **Cleanup:** issues `StorageDead(_item)` and jumps back to the condition block, ensuring borrows end before the next `MoveNext`.
   - **Break cleanup:** mirrors the cleanup block but jumps to the exit block so `break` statements still release the iteration variable before exiting.
3. The **Exit** block performs `StorageDead(_enum)`; the deferred drop expands
   to explicit `Deinit`/`Drop` statements when the drop-scheduling pass runs so
   destructors execute even on early exits.

No `Pending` statements are emitted for the loop scaffolding, keeping diagnostics precise. Borrow statements introduced for `in`/`ref`/`ref readonly` feed directly into the borrow checker.

### Yield Lowering

Iterator blocks translate into explicit suspension points. A `yield return expr;` first lowers the expression, moves the result into a temporary, and then terminates the current block with:

```text
Yield {
  value: Move(Place(_tmp)),
  resume: bb_resume,
  drop: bb_drop
}
```

`bb_resume` becomes the active block for subsequent lowering, while `bb_drop` is initialised with a `Return` terminator so abandoning the generator tears it down deterministically. Every suspension point is recorded in `MirBody.generator` as a `GeneratorYieldPoint` (yielding block, resume target, drop target, yielded local, and source span). Backends and the borrow checker use this metadata to reason about generator state without rediscovering the layout.

`yield break;` is desugared to a plain `Return` but still marks the body as a generator so metadata consumers can distinguish iterators from ordinary functions that happen to return enumerables.

### Labels & Goto

Labels mark block boundaries inside the current scope. Each `goto` lowers to a plain `Goto` terminator and the builder inserts explicit `StorageDead` statements for every variable that would otherwise fall out of scope along that branch. The drop order mirrors lexical scope exit: locals from the innermost scope are torn down first, followed by outer scopes until the target scope depth is reached. When a `goto` jumps to a label that has not yet been lowered, the builder records a snapshot of the live locals and patches the block once the label is defined so the resulting MIR is free of `Pending` markers.

Illegal control-flow (for example, jumping into a nested scope or referencing an undefined label) is diagnosed during lowering. Duplicated labels also produce diagnostics while preserving best-effort MIR so downstream passes can continue to run.

### Try/Catch/Finally Lowering

`try` statements introduce an `ExceptionRegion` that records the entry/exit points of the protected block, the catch-dispatch block, and any optional `finally` clause:

1. The builder allocates two synthetic locals per region:
   - `__exceptionN` holds the active exception object.
   - `__pending_exceptionN` (bool) tracks whether control is flowing through the region as part of unwinding. The flag is only created when a `finally` clause is present.
   Both locals receive `StorageLive` before the `try` body and `StorageDead` in the post-`try` join block.
2. The `try` body is lowered in place. If it falls through (no early `return`/`goto`), the builder sets the pending flag to `false` and emits a `Goto` to the `finally` entry (when present) or directly to the post-`try` block.
3. `throw` statements assign the thrown operand into `__exceptionN`, set the pending flag to `true` when a `finally` exists, and jump either to the `finally` entry or directly to the catch dispatch block.
4. Each `catch` clause receives three blocks:
   - **Entry:** binds the catch identifier (if any) to `__exceptionN`, evaluates the optional `when` filter, and branches to the body when the filter is `true`; otherwise it falls through to the next catch or the rethrow path.
     The lowering records the canonical exception type for each catch clause so later passes can enforce type compatibility and recover metadata for code generation.
   - **Body:** lowers the user-provided statements.
   - **Cleanup:** releases the binding (`StorageDead`), clears the pending flag, and branches to the `finally` entry (if present) or to the post-`try` block.
   Catch metadata records the binding local, filter text, and whether the guard expression parsed successfully so later passes can recover full semantics.
5. The catch dispatch block currently performs a simple `Goto` into the first catch entry. Structured type tests will be installed by later passes.
6. Unhandled exceptions (no matching catch or `throw;` after the `finally`) branch to a dedicated block that ends in `Terminator::Throw { exception: Some(Operand::Copy(__exceptionN)), ty: Some(Ty::named("Exception")) }`. Recording the canonical exception type here keeps the exceptional edge visible to later phases; codegen forwards the operand and a hashed type identity to `chic_rt_throw`, giving the runtime enough metadata to route the exception once the unwinder surface is available.
7. A `finally` clause lowers into two blocks:
   - **Entry:** lowers the user statements.
   - **Exit:** inspects `__pending_exceptionN` with a `SwitchInt`. `true` routes control to either the catch dispatch block (when catches exist) or the throw block described above; `false` continues to the post-`try` join.

This structure keeps all exception-specific control flow explicit in MIR while deferring the runtime details (type filtering, rethrow implementation) to later passes.

### Async/Await Lowering

Each `await` lowers to an `Await` terminator that makes the suspension point explicit and records the block that resumes execution once the awaited future is ready. The builder allocates a temporary to receive the awaited value, ends the current block with the terminator, and materialises dedicated resume and drop blocks:

```text
Await {
  future: Place(_f),           // evaluated future
  destination: Place(_tmp),    // temp receiving await result (optional for statement awaits)
  resume: bb_resume,           // continuation once the future is ready
  drop: bb_drop                // cancellation path (currently returns)
}
```

- The resume block becomes the active block for subsequent lowering, giving MIR a CFG-level state machine shape without additional passes.
- Borrow checker verifies all outstanding borrows are compatible with suspension.
- Async monomorphization may later rewrite MIR into explicit state machines, but the terminator form is already state-oriented and provides precise checkpoints for NLL.
- `MirBody.async_machine` records this layout. Each `AsyncSuspendPoint` stores the await block, resume/drop targets, the local holding the future, and the destination local that receives the awaited value, and the enclosing metadata now includes the list of pinned locals so backends/runtimes can avoid relocating them between suspensions. Later passes (borrow checking, codegen) consume this metadata instead of re-discovering the CFG structure.
- Generators follow the same pattern. Whenever the surface code uses `yield return` the builder emits a `Terminator::Yield` plus a resume/drop pair of blocks and records the site in `MirBody.generator`. Each `GeneratorYieldPoint` notes the yielding block, its resume/drop counterparts, the local carrying the yielded value, and the source span so borrow checking and codegen can reason about iterator state machines without re-deriving the structure.

---

## 6. Borrow Checking

- Every borrow introduces a `BorrowId` and region variable (`RegionVar`).
- Constraints derive from MIR:
  - Shared vs unique borrow conflicts.
  - Move from a place invalidates outstanding loans.
- Drops release loans.
- `StorageDead` ends the lifetime of locals; ensures no borrow outlives storage.
- Across `Await`, ensure no unique borrows survive unless the loan’s base local is marked `is_pinned`. Locals become pinned via `@pin`, `Pin<T>` annotations, or synthetic fixed guards, and the checker treats moves of pinned locals as errors while allowing their unique borrows to flow across suspension points. `lock` still forbids `Await` entirely.
- Async suspend metadata feeds the checker: awaited destinations are marked initialized on resume and diagnostics use the precise suspension span recorded in `AsyncSuspendPoint`.
- Non-lexical lifetimes (NLL): lifetimes end at last use, not block scope.
- Later enhancements may add two-phase borrows and reborrows for better method ergonomics.

Implementation status: the bootstrap pass in `src/mir/borrow.rs` seeds `in/ref/out` parameter modes, tracks region start/end spans, enforces exclusive/shared borrow invariants, blocks unique borrows across `await`, and performs definite-assignment checks for locals, return slots, and `out` parameters.

---

## 7. Slice-Based Implementation Plan

## 8. Backend Lowering (LLVM + WASM)

The bootstrap compiler currently ships the LLVM backend (default) alongside the in-house WASM backend for fast iteration. The earlier Cranelift experiment has been removed. Regardless of backend, the following MIR lowering guarantees hold:

- **Value types:** MIR `bool` values lower to 8-bit integers (`i8`) so Chic and the C ABI agree on layout. Arithmetic/comparison operators choose the appropriate integer or floating-point opcode in either backend, and comparison results are normalised for branching (`i1` → `i8` in LLVM).
- **Match lowering:** Irrefutable tuple/struct patterns that only bind locals now translate to direct branch targets in the WASM backend; the MIR builder continues to model guard evaluation and binding assignments in dedicated blocks so control flow remains explicit.
- **Call destinations:** during MIR construction we resolve callees to their `FunctionSignature`, ensuring return temporaries are created with the correct backend types. This applies equally when materialising LLVM IR prototypes and when emitting WASM signatures.
- **WASM path:** MIR lowers into WASM bytecode using `wasm-encoder`, with execution delegated to the in-tree interpreter (`runtime::wasm_executor`) for `chic run`/`chic test`. The same MIR invariants (value types, temporaries, borrow/shadow semantics) apply. Future host integrations (WASI/Wasmtime) build on top of this representation.
- **LLVM path (release profile):** MIR is rendered to textual LLVM IR (with module/DI metadata), compiled with `clang -c`, and linked using the same driver helpers. Optimisation hooks (`OptLevel`, `-flto`, `-fprofile-instr-*`) are threaded through `CodegenOptions`. The CLI now honours cross-target triples for the supported 64-bit platforms (`x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`). The generated `.ll` file is left next to the requested artefact when `keep_object` is set.
- **Debug aides:** regardless of backend the driver records the textual MIR dump inside the compile report, and (when requested) leaves intermediate objects/IR on disk to ease backend debugging.

Keep these assumptions synchronised with implementation work—changes to MIR lowering or backend behaviour must be reflected here and in the specification.

## 9. Tooling & Modules

Suggested crate/module layout:

- `chic_syntax`: lexer/parser producing AST.
- `chic_hir`: type-checked, desugared intermediate representation.
- `chic_mir`: MIR data structures, builders, pretty printer.
- `chic_interp`: MIR interpreter (Slice-1).
- `chic_borrowck`: static borrow checker (Slice-2).
- `chic_cli`: driver (parse → HIR → MIR → run/test/emit).

---

## 10. References & Further Reading

- Rust MIR design (`rustc_middle::mir`) for inspiration, tailored to Chic semantics.
- Ownership/borrowing rules in `SPEC.md` §3.
- Async & testing sections (`SPEC.md` §§2.18–2.19).

Keep this document synchronised with implementation progress. When MIR data structures or semantics change, update this reference and the spec simultaneously.
