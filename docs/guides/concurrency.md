## Practical Concurrency Guide

Chic’s concurrency toolchain combines compile-time ownership checks with explicit atomic APIs. This guide walks through the pieces you use to write correct, portable concurrent code.

### Memory Orders at a Glance

Use `Std.Sync::MemoryOrder` whenever you invoke an atomic operation:

```chic
let ready = flag.Load(Std.Sync.MemoryOrder.Acquire);
if (!ready)
{
    work();
    flag.Store(1, Std.Sync.MemoryOrder.Release);
}
```

- `Relaxed` preserves atomicity but does not create a happens-before edge.
- `Acquire`/`Release` publish and observe data across threads.
- `AcqRel` is required for read-modify-write operations that both consume and publish state.
- `SeqCst` is the default—use it whenever you need the simplest reasoning model.

Mixing success and failure orderings? Remember that the failure ordering must always be **less than or equal** to the success ordering. Invalid combinations are rejected by the compiler.

### Working with `Std.Sync` Atomics

Chic provides dedicated structs (`AtomicBool`, `AtomicI32`, `AtomicU32`, `AtomicI64`, `AtomicU64`, `AtomicUsize`) for the most common lock-free patterns. They expose `Load`/`Store`, `CompareExchange`, and (for integer types) `FetchAdd`/`FetchSub`, matching the runtime intrinsics.

```chic
public struct TicketLock
{
    private Std.Sync.AtomicU32 _next;
    private Std.Sync.AtomicU32 _owner;

    public uint Acquire()
    {
        var ticket = _next.FetchAdd(1, Std.Sync.MemoryOrder.AcqRel);
        while (_owner.Load(Std.Sync.MemoryOrder.Acquire) != ticket)
        {
            Std.Platform.Thread::SpinWait();
        }
        return ticket;
    }

    public void Release(uint ticket)
    {
        _owner.Store(ticket + 1, Std.Sync.MemoryOrder.Release);
    }
}
```

Key rules:

- Atomic structs are `@repr(c)` single-field wrappers, so you can safely embed them in other aggregates without worrying about padding.
- Methods require you to supply orderings explicitly for non-trivial behaviour.
- Atomics automatically implement `ThreadSafe`/`Shareable`, so values can cross async or thread boundaries without extra annotations.
- Use `Std.Sync.Fences.Fence(order)` when you need an ordering edge without a read or write (for example, publishing a batch of non-atomic data guarded by a separate flag).

### Coordinating with Mutexes and Locks

Higher-level primitives keep call sites tidy while delegating blocking to the runtime:

- `Mutex<T>` serialises access to a value. `Lock` blocks the calling thread/task until the runtime acquires the underlying handle; `TryLock` returns `false` immediately when contended. Always call `Release()` (or let the guard drop) so other threads can make progress.
- `RwLock<T>` allows many readers or a single writer. Use `Read` for snapshot access, `Write` for exclusive updates, and `TryRead`/`TryWrite` when you want to probe without blocking.
- `Condvar` pairs with a `Mutex<T>` guard — call `guard = condvar.Wait(mutex, guard);` to hand ownership to the runtime. It releases the mutex, parks the caller, and reacquires the mutex before returning a fresh guard.
- `Once` ensures initialisation executes at most once. Combine `TryBegin`/`Complete` manually or pass a `OnceCallback`; other callers block on `Wait` until the initializer completes.

All primitives are backed by Chic runtime handles on both native and WASM targets, so they park instead of spinning when contended. The surface API remains identical even as the runtime evolves to integrate tightly with async executors.

### `atomic { }` Blocks

For shared sections that should carry a uniform ordering, an `atomic` block keeps the source tidy while signalling intent:

```chic
atomic
{
    // Implied SeqCst barrier at entry/exit once MIR lowering lands.
    state = Transition.Ready;
    flag.Store(1, Std.Sync.MemoryOrder.SeqCst);
}

atomic(Std.Sync.MemoryOrder.AcqRel)
{
    let snapshot = buffer.Load();
    process(snapshot);
}
```

- `atomic { ... }` defaults to `SeqCst`.
- `atomic(ordering) { ... }` attaches the supplied ordering to the implicit fences the compiler will generate during MIR lowering.
- The construct plays nicely with `checked`/`unchecked`/`unsafe` scopes—the innermost modifier controls the generated code.

### Tests

Litmus tests live under `tests/concurrency/litmus`. Each scenario is a `testcase`
(`StoreBuffering`, `LoadBuffering`, `IRIW`, `MessagePassing`) that spawns real
`Std.Platform.Thread` actors, waits on a shared start gate, and fails if a forbidden
outcome is ever observed. CI runs the suite on both LLVM and WASM backends.

For deeper internal notes, see `docs/compiler/concurrency_model.md`.

### Shared Ownership (`Arc<T>` and `Weak<T>`)

- `Std.Sync::Arc<T>` is the thread-safe shared ownership primitive. Cloning an `Arc<T>` increments an
  atomic reference count; dropping it decrements the count and frees the allocation when the last
  owner disappears. `Arc<T>::Downgrade()` produces a `Weak<T>` that can be upgraded back into an
  `Arc<T>` while strong references remain (see `docs/runtime/weak_pointers.md` for full semantics).
- Use `Arc<T>::StrongCount()`/`WeakCount()` for diagnostics and instrumentation (they are not free,
  so avoid calling them in hot loops). When you only need to observe whether an allocation is still
  alive, hold a `Weak<T>` and call `Upgrade()` before performing work.
- `Arc<T>` automatically inherits `ThreadSafe`/`Shareable` from its payload. If the payload does not
  implement these auto traits, `Thread::Spawn` and async lowerings emit `[MM0102]` to prevent data
  races.
- Call `Arc<T>::GetMut()` to obtain an `ArcMutableRef<T>` only when the handle is the sole owner.
  When contention exists, `Arc<T>::MakeMut()` transparently clones the payload (requiring `T :
  Std.Clone`) and returns a `ref T` pointing at the fresh allocation. This keeps copy-on-write logic
  out of user code while still surfacing the runtime’s uniqueness check.
- `Arc<T>::TryUnwrap()` consumes the handle and returns `Std.Result<T, Arc<T>>`: `Ok` moves the
  payload out when both the strong and weak counts equal one, while `Err` hands the caller back the
  original `Arc<T>` so they can retry or fall back to cloning.
- Reach for `Rc<T>` only in single-threaded code—`Arc<T>` carries the additional cost of atomic
  updates but keeps the intent explicit. Prefer `Weak<T>` over raw `usize` handles whenever you need
  to break ownership cycles without leaking memory.

### Native Threads & Builders

OS threads complement the async executor when you need to integrate with blocking code or isolate workloads:

```chic
namespace Samples.Threads;

import Std.Sync;
import Std.Platform.Thread;

public class Counter : ThreadStart
{
    private Mutex<int> _counter;

    public Counter(Mutex<int> counter) { _counter = counter; }

    public void Run()
    {
        var guard = _counter.Lock();
        guard.Value += 1;
        guard.Release();
    }
}

public static void Main()
{
    var shared = new Mutex<int>(0);
    var thread = Thread.Spawn(ThreadStartFactory.From(new Counter(shared)));
    thread.Join();

    var builder = new ThreadBuilder().WithName("worker");
    builder.Spawn(ThreadStartFactory.Function(() => Thread.Sleep(10))).Join();
}
```

- `ThreadStart` is the required entry contract. Implementations can carry arbitrary state; `ThreadStartFactory.From` wraps them in `Arc<T>` so the runtime keeps the payload alive for the lifetime of the OS thread. `ThreadStartFactory.Function(fn() -> void)` adapts Chic function pointers into a `ThreadStart` without boilerplate.
- `Thread::Spawn` consumes the `Arc<T>` and schedules the work. The returned `Thread` tracks whether the OS thread is joinable and exposes `Join`, `Detach`, `Sleep`, `Yield`, and `SpinWait`. Dropping a joinable `Thread` detaches automatically so destructors never block.
- `ThreadBuilder` is the structured escape hatch for future runtime hints. Today it forwards to `Thread::Spawn` while recording optional metadata (names, priorities). `WithName` stores the UTF-8 thread name on the Chic `Thread` and forwards it to the runtime; names are truncated to 15 bytes (POSIX limit) and allocation failures surface as `ThreadStatus.SpawnFailed`. Linux applies the name via `pthread_setname_np`, other native targets treat naming as a no-op but still expose `Thread.Name`.
- Thread payloads and native handles are passed to the runtime as typed pointers, not raw `usize` values. `Std.Platform.Thread` converts between the leaked `Arc<T>` handle and the runtime ABI via the `Std.Numeric.UIntPtr` helpers (for example, `AddressOf<T>` / `PointerFromAddress<T>`), so `chic_thread_invoke`/`drop` trampolines never need to traffic in integers.
- Every payload must be `ThreadSafe`. Failing that requirement produces `[MM0102] THREADSAFE_REQUIRED` with actionable guidance (wrap the value in `Std.Sync.Mutex`/`RwLock`, use atomics, or adjust the type’s auto-trait annotations).
- Backends that do not support OS threads (current WASM targets) report `[MM0101] THREADS_UNAVAILABLE_ON_TARGET`. Gate `Thread` usage on `Target::supports_threads()` or split code paths by backend when portability matters.

The runtime trampolines (`@export("chic_thread_invoke")`, `@export("chic_thread_drop")`) consume the leaked `Arc<T>` handles directly, so thread payloads remain pure Chic without Rust glue. Use `Arc<T>.IntoRaw()`/`FromRaw()` sparingly—they power the runtime but should not leak into day-to-day application code.
