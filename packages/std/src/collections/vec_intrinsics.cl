namespace Std.Collections;
import Std.Runtime.Collections;
import Std.Memory;
import Std.Span;
import FVec = Foundation.Collections.Vec;
import FVecIntrinsics = Foundation.Collections.VecIntrinsics;
public static class VecIntrinsics
{
    public static VecPtr Create <T >() {
        let size = (usize) __sizeof <T >();
        let align = (usize) __alignof <T >();
        let dropFn = (isize) __drop_glue_of <T >();
        return FVecIntrinsics.chic_rt_vec_with_capacity(size, align, 0usize, dropFn);
    }
    public static VecPtr FromArray <T >(Span <T >array) {
        let size = (usize) __sizeof <T >();
        let align = (usize) __alignof <T >();
        let dropFn = (isize) __drop_glue_of <T >();
        var vec = FVecIntrinsics.chic_rt_vec_with_capacity(size, align, array.Length, dropFn);
        var raw = array.Raw;
        var readonlyRaw = SpanIntrinsics.chic_rt_span_to_readonly(ref raw);
        let length = array.Length;
        var idx = 0usize;
        while (idx <length)
        {
            var status = VecError.Success;
            unsafe {
                let elementPtr = SpanIntrinsics.chic_rt_span_ptr_at_readonly(ref readonlyRaw, idx);
                let handle = Std.Runtime.Collections.ValuePointer.CreateConst(elementPtr, size, align);
                status = FVecIntrinsics.chic_rt_vec_push(ref vec, in handle);
            }
            if (status != VecError.Success)
            {
                break;
            }
            idx += 1usize;
        }
        return vec;
    }
    public static Span <T >AsSpan <T >(ref this VecPtr vec) {
        return Vec.AsSpan <T >(ref vec);
    }
    public static ReadOnlySpan <T >AsReadOnlySpan <T >(in this VecPtr vec) {
        let data = FVecIntrinsics.chic_rt_vec_data(in vec);
        let length = FVec.Len(in vec);
        return ReadOnlySpan <T >.FromValuePointer(data, length);
    }
}
