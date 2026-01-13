namespace Std.Data;
/// <summary>Defines the generalized database type for a parameter or column.</summary>
public enum DbType
{
    /// <summary>Boolean value.</summary>
    Boolean = 0,
    /// <summary>32-bit signed integer.</summary>
    Int32 = 1,
    /// <summary>64-bit signed integer.</summary>
    Int64 = 2,
    /// <summary>Double-precision floating-point.</summary>
    Double = 3,
    /// <summary>Decimal value.</summary>
    Decimal = 4,
    /// <summary>UTF-16 text.</summary>
    String = 5,
    /// <summary>Binary data.</summary>
    Binary = 6,
    /// <summary>Uuid value.</summary>
    Uuid = 7,
    /// <summary>Date/time value.</summary>
    DateTime = 8,
}
