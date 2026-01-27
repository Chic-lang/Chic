namespace Std;
import Std.Runtime.Collections;
import Std.Span;
import Std.Core;
import Std.Core.Testing;
internal static class ExceptionRuntime
{
    public static string FromStr(str message) {
        let slice = StrPtr.FromStr(message);
        return SpanIntrinsics.chic_rt_string_from_slice(slice);
    }
}
/// <summary>
/// Base exception type used by the bootstrap runtime.
/// </summary>
public class Exception
{
    public string Message;
    public init() {
        this.Message = ExceptionRuntime.FromStr("");
    }
    public init(str message) {
        this.Message = ExceptionRuntime.FromStr(message);
    }
    public init(string message) {
        if (message is null) {
            this.Message = ExceptionRuntime.FromStr("");
            return;
        }
        this.Message = message;
    }
    public virtual string ToString() => Message;
}

testcase Given_exception_default_message_empty_When_executed_Then_exception_default_message_empty()
{
    let ex = new Exception();
    Assert.That(ex.Message == "").IsTrue();
    let _ = ex;
}

testcase Given_exception_default_to_string_empty_When_executed_Then_exception_default_to_string_empty()
{
    let ex = new Exception();
    Assert.That(ex.ToString() == "").IsTrue();
    let _ = ex;
}

testcase Given_exception_string_message_preserved_When_executed_Then_exception_string_message_preserved()
{
    let ex = new Exception("hello");
    Assert.That(ex.Message == "hello").IsTrue();
    let _ = ex;
}

testcase Given_exception_string_to_string_preserved_When_executed_Then_exception_string_to_string_preserved()
{
    let ex = new Exception("hello");
    Assert.That(ex.ToString() == "hello").IsTrue();
    let _ = ex;
}

testcase Given_exception_string_variable_message_preserved_When_executed_Then_exception_string_variable_message_preserved()
{
    let message = "world";
    let ex = new Exception(message);
    Assert.That(ex.Message == "world").IsTrue();
    let _ = ex;
}

testcase Given_exception_string_variable_to_string_preserved_When_executed_Then_exception_string_variable_to_string_preserved()
{
    let message = "world";
    let ex = new Exception(message);
    Assert.That(ex.ToString() == "world").IsTrue();
    let _ = ex;
}

testcase Given_exception_null_message_defaults_empty_When_executed_Then_exception_null_message_defaults_empty()
{
    let nullMessage = CoreIntrinsics.DefaultValue<string>();
    let ex = new Exception(nullMessage);
    Assert.That(ex.Message == "").IsTrue();
    let _ = ex;
}

testcase Given_exception_null_to_string_defaults_empty_When_executed_Then_exception_null_to_string_defaults_empty()
{
    let nullMessage = CoreIntrinsics.DefaultValue<string>();
    let ex = new Exception(nullMessage);
    Assert.That(ex.ToString() == "").IsTrue();
    let _ = ex;
}
