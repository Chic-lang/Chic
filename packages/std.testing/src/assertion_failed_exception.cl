namespace Std.Testing;
import Std;
/// <summary>
/// Raised when an assertion fails.
/// </summary>
public class AssertionFailedException : Exception
{
    public init() : super("Assertion failed") {
    }
    public init(str message) : super(message) {
    }
    public init(string message) : super(message) {
    }
}
