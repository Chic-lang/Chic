namespace Std.Data;
/// <summary>Hints for how a command should produce results.</summary>
public enum CommandBehavior
{
    /// <summary>No additional hints.</summary>
    Default = 0,
    /// <summary>Return only a single result set.</summary>
    SingleResult = 1,
    /// <summary>Return a single row.</summary>
    SingleRow = 2,
    /// <summary>Retrieve schema only.</summary>
    SchemaOnly = 3,
}
