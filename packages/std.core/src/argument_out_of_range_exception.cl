namespace Std;
import Std.Core;
import Std.Core.Testing;
public class ArgumentOutOfRangeException : Exception
{
    public init() : super() {
    }
    public init(str message) : super(ExceptionRuntime.FromStr(message)) {
    }
    public init(string message) : super(message) {
    }
    public init(string paramName, string message) : super(message) {
    }
}

testcase Given_argument_out_of_range_default_message_empty_When_executed_Then_argument_out_of_range_default_message_empty()
{
    let ex = new ArgumentOutOfRangeException();
    Assert.That(ex.Message == "").IsTrue();
    let _ = ex;
}

testcase Given_argument_out_of_range_default_to_string_empty_When_executed_Then_argument_out_of_range_default_to_string_empty()
{
    let ex = new ArgumentOutOfRangeException();
    Assert.That(ex.ToString() == "").IsTrue();
    let _ = ex;
}

testcase Given_argument_out_of_range_message_preserved_When_executed_Then_argument_out_of_range_message_preserved()
{
    let ex = new ArgumentOutOfRangeException("range");
    Assert.That(ex.Message == "range").IsTrue();
    let _ = ex;
}

testcase Given_argument_out_of_range_to_string_preserved_When_executed_Then_argument_out_of_range_to_string_preserved()
{
    let ex = new ArgumentOutOfRangeException("range");
    Assert.That(ex.ToString() == "range").IsTrue();
    let _ = ex;
}

testcase Given_argument_out_of_range_message2_preserved_When_executed_Then_argument_out_of_range_message2_preserved()
{
    let ex = new ArgumentOutOfRangeException("range2");
    Assert.That(ex.Message == "range2").IsTrue();
    let _ = ex;
}

testcase Given_argument_out_of_range_to_string2_preserved_When_executed_Then_argument_out_of_range_to_string2_preserved()
{
    let ex = new ArgumentOutOfRangeException("range2");
    Assert.That(ex.ToString() == "range2").IsTrue();
    let _ = ex;
}
