namespace Std.Collections;
import Std.Core;
import Std.Span;
import Std.Testing;
import FVecIntrinsics = Foundation.Collections.VecIntrinsics;
testcase Given_hashset_drain_iterates_all_elements_When_executed_Then_set_is_empty()
{
    var hashSet = new HashSet <int >();
    var inserted = false;
    let _ = hashSet.Insert(1, out inserted);
    let _ = hashSet.Insert(2, out inserted);
    let _ = hashSet.Insert(3, out inserted);
    var drain = hashSet.Drain();
    var count = 0;
    while (drain.Next (out var value)) {
        count += 1;
    }
    drain.dispose();
    // dispose twice should be safe
    drain.dispose();
    Assert.That(count).IsEqualTo(3);
    Assert.That(hashSet.Len()).IsEqualTo(0usize);
    hashSet.dispose();
}
testcase Given_vecptr_as_readonly_span_When_executed_Then_length_matches()
{
    var vec = VecIntrinsics.Create <int >();
    let _ = VecUtil.Push <int >(ref vec, 1);
    let _ = VecUtil.Push <int >(ref vec, 2);
    let ro = VecPtr.AsReadOnlySpan <int >(in vec);
    Assert.That(ro.Length).IsEqualTo(2usize);
    FVecIntrinsics.chic_rt_vec_drop(ref vec);
}
testcase Given_vecintrinsics_as_readonly_span_When_executed_Then_contents_match()
{
    var vec = VecIntrinsics.Create <int >();
    let _ = VecUtil.Push <int >(ref vec, 7);
    let ro = VecIntrinsics.AsReadOnlySpan <int >(in vec);
    Assert.That(ro.Length).IsEqualTo(1usize);
    Assert.That(ro[0]).IsEqualTo(7);
    FVecIntrinsics.chic_rt_vec_drop(ref vec);
}
testcase Given_array_as_readonly_span_When_executed_Then_contents_match()
{
    var arr = new int[3];
    arr[0] = 10;
    arr[1] = 20;
    arr[2] = 30;
    let ro = ArrayPtr.AsReadOnlySpan <int >(in arr);
    Assert.That(ro.Length).IsEqualTo(3usize);
    Assert.That(ro[1]).IsEqualTo(20);
}
