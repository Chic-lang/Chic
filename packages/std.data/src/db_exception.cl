namespace Std.Data;
import Std.Runtime;
/// <summary>Base exception type for database-related failures.</summary>
public class DbException : Std.Exception
{
    public init() : super() {
    }
    public init(str message) : super(StringRuntime.FromStr(message)) {
    }
    public init(string message) : super(message) {
    }
}
