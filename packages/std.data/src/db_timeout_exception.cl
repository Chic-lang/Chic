namespace Std.Data;
import Std.Runtime;
/// <summary>Exception raised when a database operation exceeds a timeout.</summary>
public class DbTimeoutException : DbException
{
    public init() : super() {
    }
    public init(str message) : super(StringRuntime.FromStr(message)) {
    }
    public init(string message) : super(message) {
    }
}
