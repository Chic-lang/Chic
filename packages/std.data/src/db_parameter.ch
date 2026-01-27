namespace Std.Data;
/// <summary>Defines a parameter supplied to a <see cref="DbCommand"/>.</summary>
public abstract class DbParameter
{
    /// <summary>Gets or sets the parameter name (including any prefix required by the provider).</summary>
    public abstract string ParameterName {
        get;
        set;
    }
    /// <summary>Gets or sets the database type for this parameter.</summary>
    public abstract DbType DbType {
        get;
        set;
    }
    /// <summary>Gets or sets how the parameter participates in command execution.</summary>
    public abstract ParameterDirection Direction {
        get;
        set;
    }
    /// <summary>Gets or sets whether null values are allowed.</summary>
    public abstract bool IsNullable {
        get;
        set;
    }
    /// <summary>Gets or sets the size for variable-length types.</summary>
    public abstract int Size {
        get;
        set;
    }
    /// <summary>Gets or sets the parameter value.</summary>
    public abstract object ?Value {
        get;
        set;
    }
}
