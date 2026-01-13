# Extern C Function Pointers and Callbacks

Raw C function pointers use the syntax `fn @extern("C")(params) -> ret`. They are **thin** pointers
(no environment) that always obey the platform C ABI, including hidden `sret` pointers for large
aggregate returns and by-value aggregate parameters.

Key rules:

- Calling or casting a raw function pointer requires `unsafe`.
- Chic closures never coerce to raw function pointers.
- Hidden `sret`/`byval` parameters are inserted for indirect calls exactly as for direct calls, so
  callbacks Chic→C and C→Chic share one ABI path.

## End-to-end example

**C side (`ffi_fnptr.c`):**

```c
#include <stdint.h>

struct Big { int64_t a, b, c; };
typedef struct Big (*make_big_fn)(int64_t base);
typedef int64_t (*sum_big_fn)(struct Big);

struct Big c_make_big(int64_t base) { return (struct Big){base, base + 1, base + 2}; }

int64_t c_call_chic_make(make_big_fn cb) {
  struct Big v = cb(40);
  return v.a + v.b + v.c;
}

int64_t c_call_chic_sum(sum_big_fn cb);

make_big_fn c_provide_big_cb(void) { return &c_make_big; }
sum_big_fn c_provide_sum_cb(void);
```

```sh
clang -c ffi_fnptr.c -o ffi_fnptr.o
ar rcs libffi_fnptr.a ffi_fnptr.o
```

**Chic side (`manifest.yaml` + `Main.cl`):**

```yaml
package:
  name: ffi-fnptr
  namespace: Samples.FfiFnPtr

build:
  kind: exe

sources:
  - path: .
    namespace_prefix: Samples.FfiFnPtr
```

```chic
import Std.Runtime.InteropServices;

@StructLayout(LayoutKind.Sequential)
public struct Big { public long a; public long b; public long c; }

public static class Native
{
    @extern("C") @link("ffi_fnptr")
    public static extern fn @extern("C")(long) -> Big c_provide_big_cb();

    @extern("C") @link("ffi_fnptr")
    public static extern fn @extern("C")(Big) -> long c_provide_sum_cb();

    @extern("C") @link("ffi_fnptr")
    public static extern long c_call_chic_make(fn @extern("C")(long) -> Big cb);

    @extern("C") @link("ffi_fnptr")
    public static extern long c_call_chic_sum(fn @extern("C")(Big) -> long cb);
}

@extern("C") @export("chic_make_big")
public static Big ChicMakeBig(long base) { return new Big(base, base + 1, base + 2); }

@extern("C") @export("chic_sum_big")
public static long ChicSumBig(Big value) { return value.a + value.b + value.c; }

public static int Main()
{
    unsafe
    {
        let c_big = Native.c_provide_big_cb();
        let via_c = c_big(10); // Chic → C indirect call (sret is inserted automatically)
        if (via_c.c != 12) { return 1; }

        let sum = Native.c_call_chic_make(ChicMakeBig); // C → Chic callback
        if (sum != (40 + 41 + 42)) { return 2; }

        let sum_cb = Native.c_provide_sum_cb();
        let total = sum_cb(new Big(5, 6, 7)); // C function pointer with by-value aggregate param
        if (total != 18) { return 3; }

        let back = Native.c_call_chic_sum(ChicSumBig);
        if (back != (7 + 8 + 9)) { return 4; }
    }
    return 0;
}
```

Build and run:

```sh
chic build manifest.yaml --backend llvm --target $(chic target host) \
  --ffi-search . -o ./ffi_fnptr_demo
./ffi_fnptr_demo
```
