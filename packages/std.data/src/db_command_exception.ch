namespace Std.Data;
import Std.Runtime;
/// <summary>Exception raised when a command operation fails.</summary>
public class DbCommandException : DbException
{
    public init() : super() {
    }
    public init(str message) : super(StringRuntime.FromStr(message)) {
    }
    public init(string message) : super(message) {
    }
}
