namespace Std;
import Std.Core;
import Std.Core.Testing;
public class ArgumentNullException : ArgumentException
{
    public init() : super() {
    }
    public init(str message) : super(ExceptionRuntime.FromStr(message)) {
    }
    public init(string message) : super(message) {
    }
}
testcase Given_argument_null_exception_default_message_empty_When_executed_Then_argument_null_exception_default_message_empty()
{
    let ex = new ArgumentNullException();
    Assert.That(ex.Message == "").IsTrue();
    let _ = ex;
}
testcase Given_argument_null_exception_default_to_string_empty_When_executed_Then_argument_null_exception_default_to_string_empty()
{
    let ex = new ArgumentNullException();
    Assert.That(ex.ToString() == "").IsTrue();
    let _ = ex;
}
testcase Given_argument_null_exception_message_preserved_When_executed_Then_argument_null_exception_message_preserved()
{
    let ex = new ArgumentNullException("missing");
    Assert.That(ex.Message == "missing").IsTrue();
    let _ = ex;
}
testcase Given_argument_null_exception_to_string_preserved_When_executed_Then_argument_null_exception_to_string_preserved()
{
    let ex = new ArgumentNullException("missing");
    Assert.That(ex.ToString() == "missing").IsTrue();
    let _ = ex;
}
testcase Given_argument_null_exception_message2_preserved_When_executed_Then_argument_null_exception_message2_preserved()
{
    let ex = new ArgumentNullException("missing2");
    Assert.That(ex.Message == "missing2").IsTrue();
    let _ = ex;
}
testcase Given_argument_null_exception_to_string2_preserved_When_executed_Then_argument_null_exception_to_string2_preserved()
{
    let ex = new ArgumentNullException("missing2");
    Assert.That(ex.ToString() == "missing2").IsTrue();
    let _ = ex;
}
