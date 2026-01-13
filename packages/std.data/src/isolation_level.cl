namespace Std.Data;
/// <summary>Defines the transaction isolation semantics requested by a command.</summary>
public enum IsolationLevel
{
    /// <summary>Use the provider default.</summary>
    Unspecified = 0,
    /// <summary>Dirty reads allowed.</summary>
    ReadUncommitted = 1,
    /// <summary>Prevents dirty reads but not non-repeatable reads.</summary>
    ReadCommitted = 2,
    /// <summary>Locks data for the duration of the transaction.</summary>
    RepeatableRead = 3,
    /// <summary>Serializable isolation.</summary>
    Serializable = 4,
    /// <summary>Snapshot-based isolation when supported.</summary>
    Snapshot = 5,
}
