# HashSet<T> Runtime & Language Design

## 1. Goals

Provide a deterministic, borrow-aware hash set for Chic that mirrors the
ergonomics of common set APIs while aligning with Chic ownership,
lifetime, and multi-backend code-generation requirements.

Key properties:

- **Deterministic layout:** iteration order is stable across runs for identical
  insertion sequences and hashers (no hidden randomness).
- **Borrow integration:** shared borrows (`in`/`ref readonly`) allow concurrent
  reads; unique borrows (`ref`/`out`) guarantee exclusive access.
- **Hasher abstraction:** pluggable hasher trait enabling deterministic default
  hashing plus user-provided implementations.
- **Set operations / iterator surface** covering insertions, removals, drains,
  and algebraic operations without unexpected allocations.

## 2. Core Semantics

### 2.1 Ownership Model

- `HashSet<T>` owns its elements. Insertion moves values into the table; removal
  or draining moves them out.
- Shared borrows (`in HashSet<T>`) permit lookups (`contains`, `get`, `len`,
  `iter`) without mutation.
- Unique borrows (`ref HashSet<T>`) are required for mutation (`insert`,
  `remove`, `clear`, `drain`, `extend`, `retain`, `split_off`).
- Entry API: `set.entry(key)` returns `OccupiedEntry<T>` or `VacantEntry<T>`,
  modelling exclusive access to the bucket while the entry guard is alive.

### 2.2 Hashing & Equality

- Requires `T: Hash + Eq` (Chic traits – see §4 for compiler support).
- Hash value + key equality determine bucket membership. Default hasher ensures
  deterministic results; user can supply custom deterministic hashers.
- Load factor threshold (default 0.75) triggers rehashing; capacity grows by
  doubling (clamped to architecture limits).
- Rehashing preserves iteration order by reinserting buckets in index order.

### 2.3 Iterator & Drain Rules

- Iterators borrow the set (`iter` => shared, `iter_mut` => unique). Mutation
  invalidates outstanding iterators; borrow checker enforces exclusivity.
- `drain(range)` unique borrow, yields owned elements while removing them.
- `drain_filter` executes predicate closure with `ref this` entry guard (unique
  borrow); removal occurs when predicate returns true.
- All iterators expose `len`, `is_empty`, and implement Chic iterator
  interfaces for composition.

### 2.4 Set Operations

- `union`, `intersection`, `difference`, `symmetric_difference` return lazy
  iterators referencing the original set(s) via shared borrows.
- `retain` accepts unique borrow and predicate closure with shared element view.
- `split_off(&mut set, &T pivot)` removes all elements whose hashed bucket index
  ≥ pivot bucket (deterministic order) producing a new set.

### 2.5 Entry API

- `entry(key)` takes unique borrow, returning:
  - `OccupiedEntry` exposing `get`, `get_mut`, `into_mut`, `remove`, `remove_entry`.
  - `VacantEntry` exposing `insert`, `insert_with`, `or_insert`, `or_insert_with`.
- Entry guards implement `Drop` to release internal borrow when leaving scope.

## 3. Runtime Implementation Sketch

### 3.1 Layout

- Backing store uses open addressing with Robin Hood probing for predictable
  performance and iteration order. Buckets store `(hash, state, payload_ptr)`.
- Metadata arrays:
  - `control` byte per bucket encoding occupancy / probe length.
  - `slots` pointer array for element storage.
- Memory allocated via `chic_rt_alloc` with reallocation helper.

### 3.2 Key Operations

- **Insert:** compute hash, probe using Robin Hood, evict less displaced entry if
  needed, update control bytes, increment length, rehash if load factor exceeded.
- **Remove:** locate bucket, mark control as deleted tombstone, shift backwards
  to fill gaps (backward shift deletion), decrement length.
- **Find:** identical to insert but stops at empty slot or matching key.
- **Iterators:** maintain pointer to buckets and step through occupied entries.

### 3.3 Hasher Abstraction

- Runtime exposes trait-like vtable with methods:
  - `init(&mut state)`
  - `update(&mut state, *const u8, usize)`
  - `finish(&mut state) -> u64`
- Default hasher: 64-bit AHash-inspired deterministic variant seeded with fixed
  constants.
- Custom hasher stored alongside set (pointer + drop hook).

## 4. Compiler & Standard Library Integration

- `Std.Collections.HashSet<T>` façade forwards to runtime functions, mirroring
  conventional set APIs where feasible.
- Provide `Std.Hash` trait with derive support; `HashSet` bounded by `T: Hash`.
- Entry types (`OccupiedEntry<T>`, `VacantEntry<T>`) implemented in `Std.Collections`.
- Reflection / metadata: update type layout table for `HashSet<T>` struct shape.
- WASM & LLVM codegen: declare runtime externs (`chic_rt_hashset_*`) mirroring
  Vec/Array patterns (len/capacity metadata).

## 5. API Surface (Chic)

```chic
namespace Std.Collections;

public struct HashSet<T, THasher = DefaultHasher>
{
    public init();
    public static HashSet<T> WithHasher(in THasher hasher);
    public HashSet<T> WithCapacity(usize capacity);
    public bool Insert(ref this, T value);
    public Option<T> Replace(ref this, T value);
    public bool Contains(in this, in T value);
    public Option<in T> Get(in this, in T value);
    public Option<T> Take(ref this, in T value);
    public bool Remove(ref this, in T value);
    public void Clear(ref this);
    public usize Len(in this);
    public usize Capacity(in this);
    public void Reserve(ref this, usize additional);
    public void ShrinkToFit(ref this);

    public Entry<T> Entry(ref this, T value);
    public Iterator<T> Iter(in this);
    public Iterator<T> Drain(ref this);
    public Iterator<T> DrainFilter(ref this, Func<ref T, bool> predicate);
    public IntoIterator<T> IntoIter(this);

    public SetUnion<T> Union(in this, in HashSet<T> other);
    // Intersection, Difference, SymmetricDifference, Retain, SplitOff, Extend...
}
```

*Exact signatures subject to Chic generics/type inference constraints.*

## 6. Testing Strategy

- Runtime unit tests (Rust) for insertion/removal, load-factor rehashing,
  iterator order, drain operations, entry API, custom hashers.
- Property tests comparing sequences of operations against Rust `HashSet`.
- Integration tests through `Std.Collections` façade (WASM & LLVM backends).
- Borrow checker regression tests ensuring iterators require appropriate borrows.

## 7. Documentation Updates

- Spec section describing hashing traits, API summary, complexity table, sample
  usage showing iteration, set ops, and entry API.
- Runtime guide documenting hasher interface, determinism requirements, and
  memory footprint.

## 8. Open Questions

- Hash trait derivation integration (auto-derive for struct/enum?) – requires
  coordination with type checking pipeline.
- Async iterator support (for future async drain) – postpone.
- Interaction with `Span<T>`/`ReadOnlySpan<T>` for set iteration – optional helper?

## 9. Next Steps

1. Prototype runtime container + façade.
2. Extend hashing trait infrastructure and derive support.
3. Implement full set ops + entry API.
4. Add exhaustive tests and documentation updates.
