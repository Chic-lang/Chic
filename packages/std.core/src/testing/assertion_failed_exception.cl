namespace Std.Core.Testing;
import Std;
public class AssertionFailedException : Exception
{
    public init() : super("Assertion failed") {
    }
    public init(string message) : super(message) {
    }
}
