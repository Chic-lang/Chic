namespace Std.Collections;
import Std.Range;
import Std.Span;
import FVec = Foundation.Collections.Vec;
import FVecIntrinsics = Foundation.Collections.VecIntrinsics;
public extension VecPtr
{
    public Span <T >AsSpan <T >(ref this VecPtr vec) {
        return Vec.AsSpan <T >(ref vec);
    }
    public ReadOnlySpan <T >AsReadOnlySpan <T >(in this VecPtr vec) {
        let data = FVecIntrinsics.chic_rt_vec_data(in vec);
        let length = FVec.Len(in vec);
        return ReadOnlySpan <T >.FromValuePointer(data, length);
    }
    public Span <T >Slice <T >(ref this VecPtr vec, Range range) {
        let bounds = SpanGuards.ResolveRangeExclusive(range, vec.Length);
        return Vec.AsSpan <T >(ref vec).Slice(bounds.Start, bounds.Length);
    }
    public Span <T >Slice <T >(ref this VecPtr vec, RangeInclusive range) {
        let bounds = SpanGuards.ResolveRangeInclusive(range, vec.Length);
        return Vec.AsSpan <T >(ref vec).Slice(bounds.Start, bounds.Length);
    }
    public Span <T >Slice <T >(ref this VecPtr vec, RangeFrom range) {
        let bounds = SpanGuards.ResolveRangeFrom(range, vec.Length);
        return Vec.AsSpan <T >(ref vec).Slice(bounds.Start, bounds.Length);
    }
    public Span <T >Slice <T >(ref this VecPtr vec, RangeTo range) {
        let bounds = SpanGuards.ResolveRangeTo(range, vec.Length);
        return Vec.AsSpan <T >(ref vec).Slice(bounds.Start, bounds.Length);
    }
    public Span <T >Slice <T >(ref this VecPtr vec, RangeFull range) {
        return Vec.AsSpan <T >(ref vec);
    }
}
