namespace Std.Runtime.Native;
// Ensure the startup descriptor weak-import is materialised as an LLVM global declaration in
// every unit that references it (some synthetic startup IR units may not carry the original
// `__chic_startup_descriptor` static through lowering).
@extern("C", alias = "__chic_startup_descriptor") @weak_import public extern static mut StartupDescriptor __chic_startup_descriptor_import;
