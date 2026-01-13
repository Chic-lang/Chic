namespace Std;
/// <summary>
/// Raised when an async task or HTTP request is canceled or times out.
/// </summary>
public class TaskCanceledException : Exception
{
    public init() : base() {
    }
    public init(str message) : base(message) {
    }
    public init(string message) : base(message) {
    }
}
