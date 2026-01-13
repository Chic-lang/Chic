# Embedded Memory-Mapped I/O

Chic exposes first-class affordances for declaring and using memory-mapped
register blocks. The compiler recognises annotated struct overlays, enforces
safety defaults, and lowers register accesses directly into ordered runtime
hooks so optimisers never rewrite or elide them.

## Declaring Register Blocks

Annotate a struct with `@mmio` to describe the block overlay:

```cl
@mmio(base = 0x4000_1000,
      size = 0x100,
      address_space = "apb",
      endian = "little",
      unsafe = true)
public struct TimerRegisters
{
    @register(offset = 0x00, width = 32, access = "rw")
    public uint Control;

    @register(offset = 0x04, width = 32, access = "ro")
    public uint Counter;

    @register(offset = 0x08, width = 16, access = "wo")
    public ushort InterruptClear;
}
```

- `base` (required) fixes the physical base address.
- `size` (optional) constrains the overlay span; the compiler reports
  misaligned or overflowing register definitions.
- `address_space` (optional) names the bus segment. The compiler hashes the
  name into a stable identifier so simulated environments can keep per-space
  state. Omit it to use the default space.
- `endian`/`endianness` selects little or big endian byte order.
- `unsafe` / `requires_unsafe` controls whether raw accesses must appear in an
  `unsafe` block. The default (`true`) forces explicit acknowledgement when a
  driver touches hardware.

Each field marked with `@register` becomes an indivisible register access:

- `offset` is always interpreted relative to the struct’s `base`.
- `width` must be 8, 16, 32, or 64; the compiler cross-checks the field type.
- `access` accepts `"rw"`, `"ro"`, or `"wo"` aliases; violations surface during
  MIR lowering instead of running undefined code.

## Safety Model

The MIR builder injects diagnostics if code:

- Touches a register that requires `unsafe` outside an unsafe block.
- Writes to read-only registers.
- Reads from write-only registers (borrowed values remain pending so additional
  diagnostics can still refer to the offending expression).

Unsafe blocks become the syntactic fence around hardware interaction:

```cl
public uint ReadCounter(ref TimerRegisters regs)
{
    unsafe
    {
        return regs.Counter;
    }
}
```

## Volatile Semantics

MIR lowering converts register reads into specialised `Operand::Mmio`
operands and writes into `StatementKind::MmioStore`. LLVM and WASM backends
emit direct calls to `chic_rt.mmio_read/mmio_write`, so optimisers cannot
reorder or coalesce the accesses. Width and endianness are passed explicitly;
address spaces ride along in the encoded flag word.

## Address Spaces & Concurrency

The runtime derives a compact `AddressSpaceId` from the `address_space`
directive. Interpreter and WASM executors keep separate `HashMap`s keyed by
`(AddressSpaceId, address)`, preventing accidental overlap when peripherals on
different buses share offsets. This separation, combined with the `unsafe`
fence, forms the concurrency story:

- Different address spaces never alias in simulation.
- Within an address space, the compiler’s borrow checker still governs
  aliasing of register overlays, so safe code cannot concurrently mutate the
  same peripheral without explicit coordination (e.g., channels, locks).

## Testing & Simulation

The in-tree executors understand MMIO out of the box:

- Width validation traps fast, producing the same diagnostics seen at MIR
  evaluation time.
- Big-endian registers transparently swap bytes on read/write.
- Conformance tests in `runtime/test_executor` and `runtime/wasm_executor`
  cover positive and negative paths for width, endianness, and address-space
  isolation.

You can pre-load simulated state in unit tests via helper methods:

```rust
interpreter.set_mmio_value(AddressSpaceId::DEFAULT, 0x4000_1004, 0xDEAD_BEEF);
let value = interpreter
    .read_mmio(&spec)
    .expect("read succeeds");
```

## Sample Driver

```cl
namespace Board.Timer;

@mmio(base = 0x4000_1000, size = 0x100, address_space = "apb")
public struct TimerRegisters
{
    @register(offset = 0x00, width = 32) public uint Control;
    @register(offset = 0x04, width = 32, access = "ro") public uint Counter;
    @register(offset = 0x08, width = 16, access = "wo") public ushort InterruptClear;
}

public static class Timer
{
    public static TimerRegisters* Map() => (TimerRegisters*)0x4000_1000;

    public static void Start()
    {
        unsafe
        {
            Map()->Control = 0x1;
        }
    }

    public static uint Snapshot()
    {
        unsafe
        {
            return Map()->Counter;
        }
    }

    public static void AckInterrupt()
    {
        unsafe
        {
            Map()->InterruptClear = 0x1;
        }
    }
}
```

This pattern keeps the unsafety tightly scoped while still delivering
zero-cost access once the overlay pointer is established.
