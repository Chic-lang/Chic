namespace Std;
import Std.Core;
import Std.Core.Testing;
public readonly struct Option <T >
{
    private readonly bool _hasValue;
    private readonly T _value;
    private init(bool hasValue, T value) {
        _hasValue = hasValue;
        _value = value;
    }
    public static Option <T >Some(T value) {
        return new Option <T >(true, value);
    }
    public static Option <T >None() {
        CoreIntrinsics.InitializeDefault(out var value);
        return new Option <T >(false, value);
    }
    public bool IsSome(out T value) {
        if (_hasValue)
        {
            value = _value;
            return true;
        }
        CoreIntrinsics.InitializeDefault(out value);
        return false;
    }
    public bool IsNone() {
        return !_hasValue;
    }
    public T UnwrapOr(T fallback) {
        if (_hasValue)
        {
            return _value;
        }
        return fallback;
    }
    public T Expect(string message) {
        if (_hasValue)
        {
            return _value;
        }
        throw new Std.InvalidOperationException("Option value is missing.");
    }
}
testcase Given_option_some_is_some_flag_When_executed_Then_option_some_is_some_flag()
{
    let opt = Option <int >.Some(7);
    var value = 0;
    let is_some = opt.IsSome(out value);
    let _ = value;
    Assert.That(is_some).IsTrue();
}
testcase Given_option_some_is_some_value_When_executed_Then_option_some_is_some_value()
{
    let opt = Option <int >.Some(7);
    var value = 0;
    let is_some = opt.IsSome(out value);
    let _ = is_some;
    Assert.That(value == 7).IsTrue();
}
testcase Given_option_some_is_some_not_none_When_executed_Then_option_some_is_some_not_none()
{
    let opt = Option <int >.Some(7);
    Assert.That(opt.IsNone()).IsFalse();
}
testcase Given_option_none_is_none_flag_When_executed_Then_option_none_is_none_flag()
{
    let opt = Option <int >.None();
    var value = 0;
    let is_some = opt.IsSome(out value);
    let _ = value;
    Assert.That(is_some).IsFalse();
}
testcase Given_option_none_is_none_value_When_executed_Then_option_none_is_none_value()
{
    let opt = Option <int >.None();
    var value = 0;
    let is_some = opt.IsSome(out value);
    let _ = is_some;
    Assert.That(value == 0).IsTrue();
}
testcase Given_option_none_is_none_is_none_When_executed_Then_option_none_is_none_is_none()
{
    let opt = Option <int >.None();
    Assert.That(opt.IsNone()).IsTrue();
}
testcase Given_option_unwrap_or_returns_fallback_for_none_When_executed_Then_option_unwrap_or_returns_fallback_for_none()
{
    let none = Option <int >.None();
    var value : int = none.UnwrapOr(3);
    Assert.That(value == 3).IsTrue();
}
testcase Given_option_unwrap_or_returns_fallback_for_some_When_executed_Then_option_unwrap_or_returns_fallback_for_some()
{
    let some = Option <int >.Some(9);
    var value : int = some.UnwrapOr(3);
    Assert.That(value == 9).IsTrue();
}
testcase Given_option_expect_throws_for_none_When_executed_Then_option_expect_throws_for_none()
{
    let none = Option <int >.None();
    var threw = false;
    try {
        let _ = none.Expect("missing");
    }
    catch(InvalidOperationException) {
        threw = true;
    }
    Assert.That(threw).IsTrue();
}
