namespace Std.Runtime.Collections;
import Std.Memory;
import Std.Core;
import Std.Numeric;
import Std.Numeric;
import Std.Core.Testing;
@extern("C") public static extern CharSpanPtr chic_rt_str_as_chars(StrPtr slice);
@repr(c) public struct InlinePadding7
{
    public byte b0;
    public byte b1;
    public byte b2;
    public byte b3;
    public byte b4;
    public byte b5;
    public byte b6;
}
@repr(c) public struct InlineBytes64
{
    public byte b00;
    public byte b01;
    public byte b02;
    public byte b03;
    public byte b04;
    public byte b05;
    public byte b06;
    public byte b07;
    public byte b08;
    public byte b09;
    public byte b10;
    public byte b11;
    public byte b12;
    public byte b13;
    public byte b14;
    public byte b15;
    public byte b16;
    public byte b17;
    public byte b18;
    public byte b19;
    public byte b20;
    public byte b21;
    public byte b22;
    public byte b23;
    public byte b24;
    public byte b25;
    public byte b26;
    public byte b27;
    public byte b28;
    public byte b29;
    public byte b30;
    public byte b31;
    public byte b32;
    public byte b33;
    public byte b34;
    public byte b35;
    public byte b36;
    public byte b37;
    public byte b38;
    public byte b39;
    public byte b40;
    public byte b41;
    public byte b42;
    public byte b43;
    public byte b44;
    public byte b45;
    public byte b46;
    public byte b47;
    public byte b48;
    public byte b49;
    public byte b50;
    public byte b51;
    public byte b52;
    public byte b53;
    public byte b54;
    public byte b55;
    public byte b56;
    public byte b57;
    public byte b58;
    public byte b59;
    public byte b60;
    public byte b61;
    public byte b62;
    public byte b63;
}
@repr(c) public struct VecPtr
{
    public * mut @expose_address byte Pointer;
    public usize Length;
    public usize Capacity;
    public usize ElementSize;
    public usize ElementAlignment;
    public isize DropCallback;
    public * mut @expose_address byte RegionPtr;
    public byte UsesInline;
    public InlinePadding7 InlinePad;
    public InlineBytes64 InlineStorage;
}
@repr(c) public struct VecViewPtr
{
    public * const @readonly @expose_address byte Pointer;
    public usize Length;
    public usize ElementSize;
    public usize ElementAlignment;
}
@repr(c) public struct VecIterPtr
{
    public * const @readonly @expose_address byte Pointer;
    public usize Index;
    public usize Length;
    public usize ElementSize;
    public usize ElementAlignment;
}
@repr(c) public struct HashSetPtr
{
    public * mut @expose_address byte Entries;
    public * mut @expose_address byte States;
    public * mut @expose_address ulong Hashes;
    public usize Length;
    public usize Capacity;
    public usize Tombstones;
    public usize ElementSize;
    public usize ElementAlignment;
    public isize DropCallback;
    public isize EqCallback;
}
@repr(c) public struct HashSetIterPtr
{
    public * const @readonly @expose_address byte Entries;
    public * const @readonly @expose_address byte States;
    public usize Index;
    public usize Capacity;
    public usize ElementSize;
    public usize ElementAlignment;
}
@repr(c) public struct HashMapPtr
{
    public * mut @expose_address byte Entries;
    public * mut @expose_address byte States;
    public * mut @expose_address byte Hashes;
    public usize Length;
    public usize Capacity;
    public usize Tombstones;
    public usize KeySize;
    public usize KeyAlignment;
    public usize ValueSize;
    public usize ValueAlignment;
    public usize EntrySize;
    public usize ValueOffset;
    public isize KeyDropCallback;
    public isize ValueDropCallback;
    public isize KeyEqCallback;
}
@repr(c) public struct HashMapIterPtr
{
    public * const @readonly @expose_address byte Entries;
    public * const @readonly @expose_address byte States;
    public usize Index;
    public usize Capacity;
    public usize EntrySize;
    public usize KeySize;
    public usize KeyAlignment;
    public usize ValueSize;
    public usize ValueAlignment;
    public usize ValueOffset;
}
@repr(c) public struct ArrayPtr
{
    public * mut @expose_address byte Pointer;
    public usize Length;
    public usize Capacity;
    public usize ElementSize;
    public usize ElementAlignment;
    public isize DropCallback;
}
@repr(c) public struct SpanPtr
{
    public ValueMutPtr Data;
    public usize Length;
    public usize ElementSize;
    public usize ElementAlignment;
}
@repr(c) public struct ReadOnlySpanPtr
{
    public ValueConstPtr Data;
    public usize Length;
    public usize ElementSize;
    public usize ElementAlignment;
}
@repr(c) public struct StrPtr
{
    public * const @readonly @expose_address byte Pointer;
    public usize Length;
    public static StrPtr FromStr(str value) {
        var slice = CoreIntrinsics.DefaultValue <StrPtr >();
        unsafe {
            var * mut @expose_address StrPtr destPtr = & slice;
            var * mut @expose_address str sourcePtr = & value;
            let destBytes = Std.Numeric.PointerIntrinsics.AsByteMut(destPtr);
            let sourceBytes = Std.Numeric.PointerIntrinsics.AsByteConstFromMut(sourcePtr);
            let size = __sizeof <StrPtr >();
            let alignment = __alignof <StrPtr >();
            let destination = ValuePointer.CreateMut(destBytes, size, alignment);
            let source = ValuePointer.CreateConst(sourceBytes, size, alignment);
            Std.Memory.GlobalAllocator.Copy(destination, source, size);
        }
        return slice;
    }
}
@repr(c) public struct CharSpanPtr
{
    public * const @readonly @expose_address ushort Pointer;
    public usize Length;
}
@repr(c) public struct ValueConstPtr
{
    public * const @readonly @expose_address byte Pointer;
    public usize Size;
    public usize Alignment;
    public init() {
        unsafe {
            Pointer = Std.Numeric.PointerIntrinsics.AsByteConst(Std.Numeric.Pointer.NullConst <byte >());
        }
        Size = 0usize;
        Alignment = 0usize;
    }
    public init(* const @readonly @expose_address byte pointer, usize size, usize alignment) {
        Pointer = pointer;
        Size = size;
        Alignment = alignment;
    }
    public init(ValueMutPtr handle) {
        unsafe {
            Pointer = Std.Numeric.PointerIntrinsics.AsByteConstFromMut(handle.Pointer);
        }
        Size = handle.Size;
        Alignment = handle.Alignment;
    }
    public static ValueConstPtr FromMut(ValueMutPtr handle) {
        return new ValueConstPtr(handle);
    }
    public static implicit operator ValueConstPtr(ValueMutPtr handle) {
        return new ValueConstPtr(handle);
    }
}
@repr(c) public struct ValueMutPtr
{
    public * mut @expose_address byte Pointer;
    public usize Size;
    public usize Alignment;
    public init() {
        unsafe {
            Pointer = Std.Numeric.PointerIntrinsics.AsByteMut(Std.Numeric.Pointer.NullMut <byte >());
        }
        Size = 0usize;
        Alignment = 0usize;
    }
    public init(* mut @expose_address byte pointer, usize size, usize alignment) {
        Pointer = pointer;
        Size = size;
        Alignment = alignment;
    }
}
public static class ValuePointer
{
    public static ValueMutPtr CreateMut(* mut @expose_address byte pointer, usize size, usize alignment) {
        return new ValueMutPtr(pointer, size, alignment);
    }
    public static ValueConstPtr CreateConst(* const @readonly @expose_address byte pointer, usize size, usize alignment) {
        return new ValueConstPtr(pointer, size, alignment);
    }
    public static ValueMutPtr NullMut(usize size = 0, usize alignment = 0) {
        var handle = new ValueMutPtr();
        unsafe {
            handle.Pointer = Std.Numeric.PointerIntrinsics.AsByteMut(Std.Numeric.Pointer.NullMut <byte >());
        }
        handle.Size = size;
        handle.Alignment = alignment;
        return handle;
    }
    public static ValueConstPtr NullConst(usize size = 0, usize alignment = 0) {
        var handle = new ValueConstPtr();
        unsafe {
            handle.Pointer = Std.Numeric.PointerIntrinsics.AsByteConst(Std.Numeric.Pointer.NullConst <byte >());
        }
        handle.Size = size;
        handle.Alignment = alignment;
        return handle;
    }
    public static bool IsNullConst(ValueConstPtr handle) {
        unsafe {
            return Std.Numeric.Pointer.IsNullConst(handle.Pointer);
        }
    }
    public static bool IsNullMut(ValueMutPtr handle) {
        unsafe {
            return Std.Numeric.Pointer.IsNull(handle.Pointer);
        }
    }
    public static bool AreEqualConst(ValueConstPtr left, ValueConstPtr right) {
        unsafe {
            return Std.Numeric.Pointer.AreEqualConst(left.Pointer, right.Pointer);
        }
    }
    public static bool AreEqualMut(ValueMutPtr left, ValueMutPtr right) {
        unsafe {
            return Std.Numeric.Pointer.AreEqual(left.Pointer, right.Pointer);
        }
    }
}
testcase Given_value_pointer_null_mut_is_null_When_executed_Then_value_pointer_null_mut_is_null()
{
    let nullMut = ValuePointer.NullMut(4usize, 4usize);
    let nullConst = ValuePointer.NullConst(4usize, 4usize);
    let _ = nullConst;
    Assert.That(ValuePointer.IsNullMut(nullMut)).IsTrue();
}
testcase Given_value_pointer_null_const_is_null_When_executed_Then_value_pointer_null_const_is_null()
{
    let nullConst = ValuePointer.NullConst(4usize, 4usize);
    Assert.That(ValuePointer.IsNullConst(nullConst)).IsTrue();
}
testcase Given_value_pointer_create_is_not_null_When_executed_Then_value_pointer_create_is_not_null()
{
    var value = 12;
    unsafe {
        var * mut @expose_address int ptr = & value;
        let handle = ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(ptr), __sizeof <int >(), __alignof <int >());
        Assert.That(ValuePointer.IsNullMut(handle)).IsFalse();
    }
}
testcase Given_value_pointer_compare_equal_When_executed_Then_value_pointer_compare_equal()
{
    var value = 12;
    unsafe {
        var * mut @expose_address int ptr = & value;
        let handle = ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(ptr), __sizeof <int >(), __alignof <int >());
        Assert.That(ValuePointer.AreEqualMut(handle, handle)).IsTrue();
    }
}
testcase Given_value_pointer_const_is_not_null_When_executed_Then_value_pointer_const_is_not_null()
{
    var value = 3;
    unsafe {
        var * const @readonly @expose_address int ptr = & value;
        let handle = ValuePointer.CreateConst(PointerIntrinsics.AsByteConst(ptr), __sizeof <int >(), __alignof <int >());
        Assert.That(ValuePointer.IsNullConst(handle)).IsFalse();
    }
}
testcase Given_value_pointer_const_are_equal_When_executed_Then_value_pointer_const_are_equal()
{
    var value = 3;
    unsafe {
        var * const @readonly @expose_address int ptr = & value;
        let handle = ValuePointer.CreateConst(PointerIntrinsics.AsByteConst(ptr), __sizeof <int >(), __alignof <int >());
        Assert.That(ValuePointer.AreEqualConst(handle, handle)).IsTrue();
    }
}
testcase Given_value_pointer_const_from_mut_is_null_When_executed_Then_value_pointer_const_from_mut_is_null()
{
    let fromMut = ValueConstPtr.FromMut(ValuePointer.NullMut());
    Assert.That(ValuePointer.IsNullConst(fromMut)).IsTrue();
}
testcase Given_strptr_from_str_sets_length_When_executed_Then_strptr_from_str_sets_length()
{
    let slice = StrPtr.FromStr("hello");
    Assert.That(slice.Length == 5usize).IsTrue();
}
