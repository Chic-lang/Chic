namespace Std;
import Std.Core;
import Std.Core.Testing;
public class ArgumentException : Exception
{
    public init() : super() {
    }
    public init(str message) : super(message) {
    }
    public init(string message) : super(message) {
    }
}
testcase Given_argument_exception_default_message_empty_When_executed_Then_argument_exception_default_message_empty()
{
    let ex = new ArgumentException();
    Assert.That(ex.Message == "").IsTrue();
    let _ = ex;
}
testcase Given_argument_exception_default_to_string_empty_When_executed_Then_argument_exception_default_to_string_empty()
{
    let ex = new ArgumentException();
    Assert.That(ex.ToString() == "").IsTrue();
    let _ = ex;
}
testcase Given_argument_exception_message_preserved_When_executed_Then_argument_exception_message_preserved()
{
    let ex = new ArgumentException("bad");
    Assert.That(ex.Message == "bad").IsTrue();
    let _ = ex;
}
testcase Given_argument_exception_to_string_preserved_When_executed_Then_argument_exception_to_string_preserved()
{
    let ex = new ArgumentException("bad");
    Assert.That(ex.ToString() == "bad").IsTrue();
    let _ = ex;
}
testcase Given_argument_exception_string_variable_message_preserved_When_executed_Then_argument_exception_string_variable_message_preserved()
{
    let msg = "bad2";
    let ex = new ArgumentException(msg);
    Assert.That(ex.Message == "bad2").IsTrue();
    let _ = ex;
}
testcase Given_argument_exception_string_variable_to_string_preserved_When_executed_Then_argument_exception_string_variable_to_string_preserved()
{
    let msg = "bad2";
    let ex = new ArgumentException(msg);
    Assert.That(ex.ToString() == "bad2").IsTrue();
    let _ = ex;
}
