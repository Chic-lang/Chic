namespace Std.Data;
/// <summary>Identifies how a command's text should be interpreted.</summary>
public enum CommandType
{
    /// <summary>Raw text command (default).</summary>
    Text = 0,
    /// <summary>Stored procedure call.</summary>
    StoredProcedure = 1,
}
