namespace Std
{
    import Std.Core;
    import Std.Core.Testing;
    import Std.Span;
    public readonly struct Result <T, E >
    {
        private readonly bool _isOk;
        private readonly T _ok;
        private readonly E _err;
        private init(bool isOk, T ok, E err) {
            _isOk = isOk;
            _ok = ok;
            _err = err;
        }
        public static Result <T, E >FromOk(T value) {
            CoreIntrinsics.InitializeDefault(out var errValue);
            return new Result <T, E >(true, value, errValue);
        }
        public static Result <T, E >FromErr(E errValue) {
            CoreIntrinsics.InitializeDefault(out var okValue);
            return new Result <T, E >(false, okValue, errValue);
        }
        public bool IsOk(out T value) {
            if (_isOk)
            {
                value = _ok;
                return true;
            }
            CoreIntrinsics.InitializeDefault(out value);
            return false;
        }
        public bool IsErr(out E errValue) {
            if (!_isOk)
            {
                errValue = _err;
                return true;
            }
            CoreIntrinsics.InitializeDefault(out errValue);
            return false;
        }
    }
    testcase Given_result_from_ok_is_ok_flag_When_executed_Then_result_from_ok_is_ok_flag()
    {
        let result = Result <int, string >.FromOk(4);
        var value = 0;
        let ok = result.IsOk(out value);
        let _ = value;
        Assert.That(ok).IsTrue();
    }
    testcase Given_result_from_ok_is_err_flag_When_executed_Then_result_from_ok_is_err_flag()
    {
        let result = Result <int, string >.FromOk(4);
        var error = SpanIntrinsics.chic_rt_string_from_slice("");
        let err = result.IsErr(out error);
        let _ = error;
        Assert.That(err).IsFalse();
    }
    testcase Given_result_from_ok_value_When_executed_Then_result_from_ok_value()
    {
        let result = Result <int, string >.FromOk(4);
        var value = 0;
        let ok = result.IsOk(out value);
        let _ = ok;
        Assert.That(value == 4).IsTrue();
    }
    testcase Given_result_from_ok_error_default_When_executed_Then_result_from_ok_error_default()
    {
        let result = Result <int, string >.FromOk(4);
        var error = SpanIntrinsics.chic_rt_string_from_slice("");
        let err = result.IsErr(out error);
        let _ = err;
        Assert.That(error == "").IsTrue();
    }
    testcase Given_result_from_err_is_ok_flag_When_executed_Then_result_from_err_is_ok_flag()
    {
        let result = Result <int, string >.FromErr("fail");
        var value = 0;
        let ok = result.IsOk(out value);
        let _ = value;
        Assert.That(ok).IsFalse();
    }
    testcase Given_result_from_err_is_err_flag_When_executed_Then_result_from_err_is_err_flag()
    {
        let result = Result <int, string >.FromErr("fail");
        var error = SpanIntrinsics.chic_rt_string_from_slice("");
        let err = result.IsErr(out error);
        let _ = error;
        Assert.That(err).IsTrue();
    }
    testcase Given_result_from_err_value_default_When_executed_Then_result_from_err_value_default()
    {
        let result = Result <int, string >.FromErr("fail");
        var value = 0;
        let ok = result.IsOk(out value);
        let _ = ok;
        Assert.That(value == 0).IsTrue();
    }
    testcase Given_result_from_err_error_value_When_executed_Then_result_from_err_error_value()
    {
        let result = Result <int, string >.FromErr("fail");
        var error = SpanIntrinsics.chic_rt_string_from_slice("");
        let err = result.IsErr(out error);
        let _ = err;
        Assert.That(error == "fail").IsTrue();
    }
}
