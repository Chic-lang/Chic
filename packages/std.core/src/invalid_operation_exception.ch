namespace Std;
import Std.Core;
import Std.Core.Testing;
/// <summary>
/// Thrown when an operation is invalid for the current object state.
/// </summary>
public class InvalidOperationException : Exception
{
    public init() : super() {
    }
    public init(str message) : super(ExceptionRuntime.FromStr(message)) {
    }
    public init(string message) : super(message) {
    }
}

testcase Given_invalid_operation_default_message_empty_When_executed_Then_invalid_operation_default_message_empty()
{
    let ex = new InvalidOperationException();
    Assert.That(ex.Message == "").IsTrue();
    let _ = ex;
}

testcase Given_invalid_operation_default_to_string_empty_When_executed_Then_invalid_operation_default_to_string_empty()
{
    let ex = new InvalidOperationException();
    Assert.That(ex.ToString() == "").IsTrue();
    let _ = ex;
}

testcase Given_invalid_operation_message_preserved_When_executed_Then_invalid_operation_message_preserved()
{
    let ex = new InvalidOperationException("invalid");
    Assert.That(ex.Message == "invalid").IsTrue();
    let _ = ex;
}

testcase Given_invalid_operation_to_string_preserved_When_executed_Then_invalid_operation_to_string_preserved()
{
    let ex = new InvalidOperationException("invalid");
    Assert.That(ex.ToString() == "invalid").IsTrue();
    let _ = ex;
}
