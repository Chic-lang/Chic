namespace Std.Data;
/// <summary>Indicates how a parameter flows through a command invocation.</summary>
public enum ParameterDirection
{
    /// <summary>Parameter supplies a value.</summary>
    Input = 0,
    /// <summary>Parameter receives a value from the command.</summary>
    Output = 1,
    /// <summary>Parameter both supplies and receives a value.</summary>
    InputOutput = 2,
    /// <summary>Parameter receives a return value.</summary>
    ReturnValue = 3,
}
