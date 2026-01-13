# Region/Arena Allocation Profiles

Region blocks (`region name { ... }`) allocate from arena-style pools that free every allocation when the block exits. The compiler lowers each region to a runtime handle created via `Std.Memory.Region.Enter(name)`. The handle is dropped automatically at the end of the block (including early returns and unwinding), which calls `chic_rt_region_exit` to release all recorded allocations.

## APIs

- `Std.Memory.Region.Enter(string profile = "default") -> RegionHandle`  
  Creates a region keyed by the provided profile hash.
- `RegionHandle.dispose` / `Std.Memory.Region.Exit(handle)`  
  Deterministically tears down the region; re-entrant calls are ignored.
- Allocation helpers:  
  - `Region.Alloc<T>(handle, usize count = 1)` / `Region.AllocZeroed<T>(...)`  
  - `Region.Span<T>(handle, usize length)` to materialise a `Span<T>` over region memory.
  - Raw hooks: `Region.Alloc(handle, usize size, usize align)`
- Telemetry/debugging:  
  - `Region.Telemetry(handle)` returns allocation/drop counters (calls/bytes/freed).  
  - `Region.ResetTelemetry(handle)` clears counters for profiling loops.
- Container integration:  
  - `Vec.NewIn<T>(RegionHandle)` / `Vec.WithCapacityIn<T>(..., RegionHandle)` route all allocations through the region allocator. Pushing after region teardown fails with `AllocationFailed` instead of corrupting memory.

## Semantics & Guards

- Region allocations may not outlive the region scope. Operations on region-backed containers after teardown return allocation failures; region telemetry still reports the final freed bytes so leaks are visible in tests/benchmarks.
- Nested regions are independent—each handle tracks its own profile and generation. Exiting a parent region does not implicitly exit children; the compiler schedules drops in lexical order.
- Teardown is idempotent: repeated drops or explicit `Region.Exit` calls are ignored after the first successful release.
- Runtime adapters (native/wasm) use deterministic counters; freeing after exit is ignored, and overlapping allocations are forbidden by construction.

## Troubleshooting

- **Unexpected `AllocationFailed`**: The region was torn down; ensure the region handle and any containers do not escape the block. Telemetry should show freed bytes for the released region.
- **Leaked allocations in benchmarks**: Call `Region.ResetTelemetry` at the start of the measured block and inspect `Region.Telemetry` before exit to verify byte counters return to zero.
- **Overflowing profile hashes**: Profiles are hashed with FNV-1a into `u64`. Collisions are supported (profiles only label telemetry); use descriptive names for dashboards.

## Notes (Games/HPC)

- Replace ad hoc arena structs with `region name { ... }` and `Vec.WithCapacityIn` inside tight loops. Region teardown is constant-time and deterministic, eliminating per-object frees in frame boundaries.
- For temporary buffers, prefer `Region.Span<T>` over stack allocations to keep peak stack usage predictable on embedded/console targets.
