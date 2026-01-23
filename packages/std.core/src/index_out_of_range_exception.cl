namespace Std;
import Std.Core;
import Std.Core.Testing;
public class IndexOutOfRangeException : Exception
{
    public init() : super() {
    }
    public init(str message) : super(ExceptionRuntime.FromStr(message)) {
    }
    public init(string message) : super(message) {
    }
}
testcase Given_index_out_of_range_default_message_empty_When_executed_Then_index_out_of_range_default_message_empty()
{
    let ex = new IndexOutOfRangeException();
    Assert.That(ex.Message == "").IsTrue();
    let _ = ex;
}
testcase Given_index_out_of_range_default_to_string_empty_When_executed_Then_index_out_of_range_default_to_string_empty()
{
    let ex = new IndexOutOfRangeException();
    Assert.That(ex.ToString() == "").IsTrue();
    let _ = ex;
}
testcase Given_index_out_of_range_message_preserved_When_executed_Then_index_out_of_range_message_preserved()
{
    let ex = new IndexOutOfRangeException("oops");
    Assert.That(ex.Message == "oops").IsTrue();
    let _ = ex;
}
testcase Given_index_out_of_range_to_string_preserved_When_executed_Then_index_out_of_range_to_string_preserved()
{
    let ex = new IndexOutOfRangeException("oops");
    Assert.That(ex.ToString() == "oops").IsTrue();
    let _ = ex;
}
testcase Given_index_out_of_range_message2_preserved_When_executed_Then_index_out_of_range_message2_preserved()
{
    let ex = new IndexOutOfRangeException("oops2");
    Assert.That(ex.Message == "oops2").IsTrue();
    let _ = ex;
}
testcase Given_index_out_of_range_to_string2_preserved_When_executed_Then_index_out_of_range_to_string2_preserved()
{
    let ex = new IndexOutOfRangeException("oops2");
    Assert.That(ex.ToString() == "oops2").IsTrue();
    let _ = ex;
}
