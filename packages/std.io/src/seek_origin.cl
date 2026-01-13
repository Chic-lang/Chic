namespace Std.IO;
/// <summary>Specifies the reference point used to obtain a new position within a stream.</summary>
public enum SeekOrigin
{
    /// <summary>Seek relative to the beginning of the stream.</summary>
    Begin = 0,
    /// <summary>Seek relative to the current position.</summary>
    Current = 1,
    /// <summary>Seek relative to the end of the stream.</summary>
    End = 2,
}
