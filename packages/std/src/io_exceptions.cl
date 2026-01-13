namespace Std;
/// <summary>Base class for IO-related errors.</summary>
public class IOException : Exception
{
    public init(string message) : base(message) {
    }
}
/// <summary>Thrown when attempting to read beyond the end of a stream.</summary>
public class EndOfStreamException : IOException
{
    public init(string message) : base(message) {
    }
}
/// <summary>Thrown when an operation is attempted on a disposed object.</summary>
public class ObjectDisposedException : Exception
{
    public init(string message) : base(message) {
    }
}
