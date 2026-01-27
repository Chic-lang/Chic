import Std.Numeric.Decimal;
import Std.Runtime.Collections;
import Std.Runtime;
import Std;
namespace Std.Clone;
public static class Runtime
{
    public static T CloneField <T >(in T value) {
        let glue = __clone_glue_of <T >();
        if (glue == 0)
        {
            if (CloneHelpers.TryClone (in value, out var decimalClone)) {
                return decimalClone;
            }
            var slot = Std.Memory.MaybeUninit <T >.Uninit();
            slot.Write(value);
            return slot.AssumeInit();
        }
        var slot = Std.Memory.MaybeUninit <T >.Uninit();
        var source = Std.Memory.MaybeUninit <T >.Uninit();
        source.Write(value);
        let srcHandle = source.AsValueConstPtr();
        let destHandle = slot.AsValueMutPtr();
        Std.Runtime.CloneRuntime.Invoke(glue, srcHandle, destHandle);
        source.ForgetInit();
        slot.MarkInitialized();
        return slot.AssumeInit();
    }
}
