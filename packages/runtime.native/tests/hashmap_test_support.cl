namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
public static class HashMapTestSupport
{
    @extern("C") public unsafe static void DropNoop(* mut @expose_address byte _ptr) {
    }
    @extern("C") public unsafe static int KeyEq(* const @readonly @expose_address byte left,
    * const @readonly @expose_address byte right) {
        if (left == null || right == null)
        {
            return 0;
        }
        var * const @readonly @expose_address int leftPtr = left;
        var * const @readonly @expose_address int rightPtr = right;
        return * leftPtr == * rightPtr ?1 : 0;
    }
}
