#![no_std] namespace Std.Runtime.Native;
// Chic-native runtime entry point. Each submodule provides the frozen `chic_rt_*`
// symbols backed by Chic implementations for strings/vectors/shared/test execution.
public static class NativeRuntime
{
}
// Decimal drop stubs ---------------------------------------------------------
// Decimal runtime support is not provided by the native runtime; the drop
// entrypoints are exported as no-ops to satisfy link-time expectations.
@export("__cl_drop__DecimalBinary") public unsafe static void __cl_drop__DecimalBinary(* mut @expose_address byte _ptr) {
}
@export("__cl_drop__DecimalTernary") public unsafe static void __cl_drop__DecimalTernary(* mut @expose_address byte _ptr) {
}
@export("__cl_drop__Std__Runtime__Native__DecimalBinary") public unsafe static void __cl_drop__Std__Runtime__Native__DecimalBinary(* mut @expose_address byte _ptr) {
}
@export("__cl_drop__Std__Runtime__Native__DecimalTernary") public unsafe static void __cl_drop__Std__Runtime__Native__DecimalTernary(* mut @expose_address byte _ptr) {
}
@export("__cl_drop__fn_decimal__decimal_____decimal") public unsafe static void __cl_drop__fn_decimal__decimal_____decimal(* mut @expose_address byte _ptr) {
}
@export("__cl_drop__fn_decimal__decimal__decimal_____decimal") public unsafe static void __cl_drop__fn_decimal__decimal__decimal_____decimal(* mut @expose_address byte _ptr) {
}
