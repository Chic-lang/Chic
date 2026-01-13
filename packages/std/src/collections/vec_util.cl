namespace Std.Collections;
import Std.Runtime.Collections;
import Std.Memory;
import Std.Span;
import FVec = Foundation.Collections.Vec;
import FVecIntrinsics = Foundation.Collections.VecIntrinsics;
public static class VecUtil
{
    public static VecError Push <T >(ref VecPtr vec, T value) {
        var slot = Std.Memory.MaybeUninit <T >.Init(value);
        let handle = slot.AsValueConstPtr();
        let status = FVecIntrinsics.chic_rt_vec_push(ref vec, in handle);
        if (status == VecError.Success)
        {
            slot.ForgetInit();
        }
        return status;
    }
    public static bool TryPop <T >(ref VecPtr vec, out T value) {
        var slot = Std.Memory.MaybeUninit <T >.Uninit();
        if (FVec.Len (in vec) == 0usize) {
            value = Std.Memory.Intrinsics.ZeroValue <T >();
            return false;
        }
        let handle = slot.AsValueMutPtr();
        let status = FVecIntrinsics.chic_rt_vec_pop(ref vec, in handle);
        if (status == VecError.Success)
        {
            slot.MarkInitialized();
            value = slot.AssumeInit();
            return true;
        }
        value = Std.Memory.Intrinsics.ZeroValue <T >();
        return false;
    }
}
