namespace Std;
import Std.Runtime;
public class NotSupportedException : Exception
{
    public init() : super() {
    }
    public init(str message) : super(StringRuntime.FromStr(message)) {
    }
    public init(string message) : super(message) {
    }
}
