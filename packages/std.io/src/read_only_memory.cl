namespace Std.IO;
import Std.Numeric;
import Std.Span;
/// <summary>Represents a read-only contiguous block of memory with span access.</summary>
public struct ReadOnlyMemory <T >
{
    private T[] ?_array;
    private int _start;
    private int _length;
    /// <summary>Initializes a read-only memory view over the entire array.</summary>
    /// <param name="array">Source array; null produces an empty memory.</param>
    public init(T[] ?array) {
        _array = array;
        _start = 0;
        _length = array == null ?0 : array.Length;
    }
    /// <summary>Initializes a read-only memory view over a portion of the array.</summary>
    /// <param name="array">Source array.</param>
    /// <param name="start">Start offset in the array.</param>
    /// <param name="length">Number of elements to include.</param>
    /// <exception cref="Std.ArgumentNullException">Thrown when <paramref name="array" /> is null and length is non-zero.</exception>
    /// <exception cref="Std.ArgumentOutOfRangeException">Thrown when the range is outside the bounds of the array.</exception>
    public init(T[] ?array, int start, int length) {
        if (array == null)
        {
            if (start != 0 || length != 0)
            {
                throw new Std.ArgumentNullException("array");
            }
            _array = null;
            _start = 0;
            _length = 0;
            return;
        }
        if (start <0 || length <0 || (start + length) >array.Length)
        {
            throw new Std.ArgumentOutOfRangeException("start/length");
        }
        _array = array;
        _start = start;
        _length = length;
    }
    /// <summary>Gets the number of elements contained in the memory block.</summary>
    public int Length => _length;
    /// <summary>Gets a span that represents the same read-only memory.</summary>
    public ReadOnlySpan <T >Span {
        get {
            var array = _array;
            if (array == null)
            {
                return ReadOnlySpan <T >.Empty;
            }
            var span = ReadOnlySpan <T >.FromArray(ref array);
            return span.Slice(NumericUnchecked.ToUSize(_start), NumericUnchecked.ToUSize(_length));
        }
    }
}
