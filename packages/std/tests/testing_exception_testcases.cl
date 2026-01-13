namespace Std.Testing;
import Std;
testcase Given_assertion_failed_exception_message_When_executed_Then_assertion_failed_exception_message()
{
    let ex = new AssertionFailedException("failed");
    Assert.That(ex.Message).IsEqualTo("failed");
}
