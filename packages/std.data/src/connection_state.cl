namespace Std.Data;
/// <summary>Represents the lifecycle state of a database connection.</summary>
public enum ConnectionState
{
    Closed = 0, Connecting = 1, Open = 2, Broken = 3,
}
