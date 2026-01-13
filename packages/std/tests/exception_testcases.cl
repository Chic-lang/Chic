namespace Std;
import Std.Testing;
static class ExceptionTestHelpers
{
    public static bool ExpectMessage(Exception ex, string expected) {
        return ex.Message == expected;
    }
}
testcase Given_exception_default_message_is_empty_When_executed_Then_exception_default_message_is_empty()
{
    let baseEx = new Exception();
    Assert.That(baseEx.Message.Length).IsEqualTo(0);
}
testcase Given_exception_message_is_preserved_When_executed_Then_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new Exception("message"), "message")).IsTrue();
}
testcase Given_argument_exception_message_is_preserved_When_executed_Then_argument_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new ArgumentException("arg"), "arg")).IsTrue();
}
testcase Given_argument_null_exception_message_is_preserved_When_executed_Then_argument_null_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new ArgumentNullException("null"), "null")).IsTrue();
}
testcase Given_argument_out_of_range_exception_message_is_preserved_When_executed_Then_argument_out_of_range_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new ArgumentOutOfRangeException("range"), "range")).IsTrue();
}
testcase Given_index_out_of_range_exception_message_is_preserved_When_executed_Then_index_out_of_range_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new IndexOutOfRangeException("index"), "index")).IsTrue();
}
testcase Given_format_exception_message_is_preserved_When_executed_Then_format_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new FormatException("format"), "format")).IsTrue();
}
testcase Given_divide_by_zero_exception_message_is_preserved_When_executed_Then_divide_by_zero_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new DivideByZeroException("divide"), "divide")).IsTrue();
}
testcase Given_invalid_cast_exception_message_is_preserved_When_executed_Then_invalid_cast_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new InvalidCastException("cast"), "cast")).IsTrue();
}
testcase Given_invalid_operation_exception_message_is_preserved_When_executed_Then_invalid_operation_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new InvalidOperationException("op"), "op")).IsTrue();
}
testcase Given_not_supported_exception_message_is_preserved_When_executed_Then_not_supported_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new NotSupportedException("nope"), "nope")).IsTrue();
}
testcase Given_overflow_exception_message_is_preserved_When_executed_Then_overflow_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new OverflowException("overflow"), "overflow")).IsTrue();
}
testcase Given_task_canceled_exception_message_is_preserved_When_executed_Then_task_canceled_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new TaskCanceledException("cancel"), "cancel")).IsTrue();
}
testcase Given_io_exception_message_is_preserved_When_executed_Then_io_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new IOException("io"), "io")).IsTrue();
}
testcase Given_end_of_stream_exception_message_is_preserved_When_executed_Then_end_of_stream_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new EndOfStreamException("eos"), "eos")).IsTrue();
}
testcase Given_object_disposed_exception_message_is_preserved_When_executed_Then_object_disposed_exception_message_is_preserved()
{
    Assert.That(ExceptionTestHelpers.ExpectMessage(new ObjectDisposedException("disposed"), "disposed")).IsTrue();
}
