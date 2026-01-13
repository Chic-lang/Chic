namespace Std.Data.Mapping;
import Std.Runtime;
/// <summary>Raised when a row cannot be mapped to the requested shape.</summary>
public class DataMappingException : DbException
{
    public init() : super() {
    }
    public init(str message) : super(StringRuntime.FromStr(message)) {
    }
    public init(string message) : super(message) {
    }
}
