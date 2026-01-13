namespace Std.Numeric;
import Std.Core;
import Std.Memory;
import Std.Runtime.Collections;
import Std.Core.Testing;
internal static class PointerIntrinsics
{
    public static unsafe * mut @expose_address T AsMutPtr <T >(* mut @expose_address T pointer) {
        return pointer;
    }
    public static unsafe * const @readonly @expose_address T AsConstPtr <T >(* const @readonly @expose_address T pointer) {
        return pointer;
    }
    public static unsafe * mut @expose_address byte AsByteMut <T >(* mut @expose_address T pointer) {
        return pointer;
    }
    public static unsafe * const @readonly @expose_address byte AsByteConst <T >(* const @readonly @expose_address T pointer) {
        return pointer;
    }
    public static unsafe * const @readonly @expose_address byte AsByteConstFromMut <T >(* mut @expose_address T pointer) {
        return pointer;
    }
}
/// <summary>
/// Helper methods that expose raw pointers through sanctioned numeric types
/// without depending on the full standard library surface.
/// </summary>
public static class Pointer
{
    public static unsafe nuint HandleFrom <T >(* mut @expose_address T pointer) {
        return(nuint) pointer;
    }
    public static unsafe nuint HandleFromConst <T >(* const @readonly @expose_address T pointer) {
        return(nuint) pointer;
    }
    public static unsafe nuint AddressOf <T >(* mut @expose_address T pointer) {
        return(nuint) pointer;
    }
    public static unsafe nuint AddressOfConst <T >(* const @readonly @expose_address T pointer) {
        return(nuint) pointer;
    }
    public static unsafe * mut @expose_address T NullMut <T >() {
        return(* mut @expose_address T) 0;
    }
    public static unsafe * const @readonly @expose_address T NullConst <T >() {
        return(* const @readonly @expose_address T) 0;
    }
    public static unsafe bool AreEqual <T >(* mut @expose_address T left, * mut @expose_address T right) {
        return(nuint) left == (nuint) right;
    }
    public static unsafe bool AreEqualConst <T >(* const @readonly @expose_address T left, * const @readonly @expose_address T right) {
        return(nuint) left == (nuint) right;
    }
    public static unsafe bool IsNull <T >(* mut @expose_address T pointer) {
        return(nuint) pointer == 0;
    }
    public static unsafe bool IsNullConst <T >(* const @readonly @expose_address T pointer) {
        return(nuint) pointer == 0;
    }
}

testcase Given_pointer_null_mut_is_null_When_executed_Then_pointer_null_mut_is_null()
{
    unsafe {
        let ptr = Pointer.NullMut<byte>();
        Assert.That(Pointer.IsNull(ptr)).IsTrue();
    }
}

testcase Given_pointer_null_const_is_null_When_executed_Then_pointer_null_const_is_null()
{
    unsafe {
        let cptr = Pointer.NullConst<byte>();
        Assert.That(Pointer.IsNullConst(cptr)).IsTrue();
    }
}

testcase Given_pointer_ptr_is_not_null_When_executed_Then_pointer_ptr_is_not_null()
{
    var value = 5;
    unsafe {
        var * mut @expose_address int ptr = & value;
        Assert.That(Pointer.IsNull(ptr)).IsFalse();
    }
}

testcase Given_pointer_ptr_are_equal_When_executed_Then_pointer_ptr_are_equal()
{
    var value = 5;
    unsafe {
        var * mut @expose_address int ptr = & value;
        Assert.That(Pointer.AreEqual(ptr, ptr)).IsTrue();
    }
}

testcase Given_pointer_address_is_nonzero_When_executed_Then_pointer_address_is_nonzero()
{
    var value = 5;
    unsafe {
        var * mut @expose_address int ptr = & value;
        let address = Pointer.AddressOf(ptr);
        Assert.That(address != (nuint) 0).IsTrue();
    }
}

testcase Given_pointer_const_is_not_null_When_executed_Then_pointer_const_is_not_null()
{
    var value = 7;
    unsafe {
        var * const @readonly @expose_address int cptr = & value;
        Assert.That(Pointer.IsNullConst(cptr)).IsFalse();
    }
}

testcase Given_pointer_const_are_equal_When_executed_Then_pointer_const_are_equal()
{
    var value = 7;
    unsafe {
        var * const @readonly @expose_address int cptr = & value;
        Assert.That(Pointer.AreEqualConst(cptr, cptr)).IsTrue();
    }
}

testcase Given_pointer_const_handle_matches_address_When_executed_Then_pointer_const_handle_matches_address()
{
    var value = 7;
    unsafe {
        var * const @readonly @expose_address int cptr = & value;
        let handle = Pointer.HandleFromConst(cptr);
        let address = Pointer.AddressOfConst(cptr);
        Assert.That(handle == address).IsTrue();
    }
}

testcase Given_pointer_intrinsics_bytes_mut_not_null_When_executed_Then_pointer_intrinsics_bytes_mut_not_null()
{
    var value = 11;
    unsafe {
        var * mut @expose_address int ptr = & value;
        let mutPtr = PointerIntrinsics.AsMutPtr(ptr);
        let constPtr = PointerIntrinsics.AsConstPtr(mutPtr);
        let bytesMut = PointerIntrinsics.AsByteMut(mutPtr);
        let bytesConst = PointerIntrinsics.AsByteConst(constPtr);
        let bytesConstFromMut = PointerIntrinsics.AsByteConstFromMut(mutPtr);
        let _ = bytesConst;
        let _ = bytesConstFromMut;
        Assert.That(Pointer.IsNull(bytesMut)).IsFalse();
    }
}

testcase Given_pointer_intrinsics_bytes_const_not_null_When_executed_Then_pointer_intrinsics_bytes_const_not_null()
{
    var value = 11;
    unsafe {
        var * mut @expose_address int ptr = & value;
        let mutPtr = PointerIntrinsics.AsMutPtr(ptr);
        let constPtr = PointerIntrinsics.AsConstPtr(mutPtr);
        let bytesConst = PointerIntrinsics.AsByteConst(constPtr);
        Assert.That(Pointer.IsNullConst(bytesConst)).IsFalse();
    }
}

testcase Given_pointer_intrinsics_bytes_mut_roundtrip_When_executed_Then_pointer_intrinsics_bytes_mut_roundtrip()
{
    var value = 11;
    unsafe {
        var * mut @expose_address int ptr = & value;
        let mutPtr = PointerIntrinsics.AsMutPtr(ptr);
        let bytesMut = PointerIntrinsics.AsByteMut(mutPtr);
        Assert.That(Pointer.AreEqual(bytesMut, PointerIntrinsics.AsByteMut(mutPtr))).IsTrue();
    }
}

testcase Given_pointer_intrinsics_bytes_const_roundtrip_When_executed_Then_pointer_intrinsics_bytes_const_roundtrip()
{
    var value = 11;
    unsafe {
        var * mut @expose_address int ptr = & value;
        let mutPtr = PointerIntrinsics.AsMutPtr(ptr);
        let constPtr = PointerIntrinsics.AsConstPtr(mutPtr);
        let bytesConst = PointerIntrinsics.AsByteConst(constPtr);
        let bytesConstFromMut = PointerIntrinsics.AsByteConstFromMut(mutPtr);
        Assert.That(Pointer.AreEqualConst(bytesConst, bytesConstFromMut)).IsTrue();
    }
}
