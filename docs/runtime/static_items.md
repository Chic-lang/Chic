# Static items at runtime

Chic statics are emitted as eager, process-wide storage slots. The compiler folds every initializer through the constant-evaluation engine and writes the resulting bytes into the backend-specific data segment:

- **LLVM:** each static lowers to a global symbol with alignment derived from the type layout. `public` statics use `dso_local global`, while non-public statics are emitted `internal`.
- **WASM:** statics become linear-memory data segments. The emitter enforces scalar widths and alignment before reserving space in the module’s linear memory.

Mutable statics (`static mut`) are not synchronised; both reads and writes require an `unsafe` block and the caller must provide locking if the value is shared across threads. Immutable statics (`static const`) may be read safely from any context.

Initialisation is eager: loading a module materialises the static bytes immediately. Reflection sidecars will be extended in a follow-on milestone to publish the list of statics and their qualified names so executors can surface them to dynamic loaders.
