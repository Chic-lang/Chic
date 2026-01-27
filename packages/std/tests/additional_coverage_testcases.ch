namespace Std;
import Std.Collections;
import Std.Core;
import Std.Datetime;
import Std.Numeric;
import Std.Numeric.Decimal;
import Std.Range;
import Std.Span;
import Std.Testing;
import FVecIntrinsics = Foundation.Collections.VecIntrinsics;
testcase Given_decimal_intrinsics_division_by_zero_When_executed_Then_status_reports_division_by_zero()
{
    let result = DecimalIntrinsics.Div(1m, 0m);
    Assert.That(result.Status == DecimalStatus.DivisionByZero).IsTrue();
}
testcase Given_decimal_intrinsics_add_When_executed_Then_status_success_and_value_matches()
{
    let result = DecimalIntrinsics.Add(1.25m, 2.75m);
    Assert.That(result.Status == DecimalStatus.Success).IsTrue();
    Assert.That(result.Value.ToString("G", "invariant")).IsEqualTo("4");
}
testcase Given_vecptr_ranges_slice_exclusive_When_executed_Then_slice_contains_expected_values()
{
    var vec = VecIntrinsics.Create <int >();
    let _ = VecUtil.Push <int >(ref vec, 10);
    let _ = VecUtil.Push <int >(ref vec, 20);
    let _ = VecUtil.Push <int >(ref vec, 30);
    let _ = VecUtil.Push <int >(ref vec, 40);
    let range = new Range {
        Start = Index.FromStart(1usize), End = Index.FromStart(3usize)
    }
    ;
    let slice = VecPtr.Slice <int >(ref vec, range);
    Assert.That(slice.Length).IsEqualTo(2usize);
    Assert.That(slice[0]).IsEqualTo(20);
    Assert.That(slice[1]).IsEqualTo(30);
    FVecIntrinsics.chic_rt_vec_drop(ref vec);
}
testcase Given_arrayptr_ranges_slice_exclusive_When_executed_Then_slice_contains_expected_values()
{
    var arr = new int[5];
    arr[0] = 1;
    arr[1] = 2;
    arr[2] = 3;
    arr[3] = 4;
    arr[4] = 5;
    let range = new Range {
        Start = Index.FromStart(1usize), End = Index.FromStart(3usize)
    }
    ;
    let slice = ArrayPtr.Slice <int >(ref arr, range);
    Assert.That(slice.Length).IsEqualTo(2usize);
    Assert.That(slice[0]).IsEqualTo(2);
    Assert.That(slice[1]).IsEqualTo(3);
}
testcase Given_invariant_datetime_culture_When_executed_Then_separators_are_expected()
{
    let culture = InvariantDateTimeCulture.Instance;
    Assert.That(culture.DateSeparator()).IsEqualTo("-");
    Assert.That(culture.TimeSeparator()).IsEqualTo(":");
    Assert.That(culture.UtcDesignator()).IsEqualTo("Z");
}
testcase Given_float32_and_float128_format_special_values_When_executed_Then_outputs_tokens()
{
    let nan32 = Float32.NaN;
    Assert.That(nan32.ToString("G", "invariant")).IsEqualTo("NaN");
    let posInf32 = Float32.PositiveInfinity;
    Assert.That(posInf32.ToString("G", "invariant")).IsEqualTo("Infinity");
    let f128 = new Float128(1.25d);
    Assert.That(f128.ToString("F2", "invariant")).Contains(".");
}
