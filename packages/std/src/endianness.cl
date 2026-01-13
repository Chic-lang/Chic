namespace Std;
/// <summary>
/// Represents the byte ordering used when encoding or decoding multibyte values.
/// </summary>
public enum Endianness
{
    /// <summary>
    /// Least significant byte is stored first.
    /// </summary>
    Little = 0,
    /// <summary>
    /// Most significant byte is stored first.
    /// </summary>
    Big = 1,
}
