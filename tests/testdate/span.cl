import Std;
import Std.Span;
import Std.Strings;

namespace Exec;

public int Main()
{
    Span<int> primary = Span<int>.StackAlloc(4);
    ReadOnlySpan<int> readonlyView = primary.AsReadOnly();
    if (primary.Length != 4 || primary.Length != readonlyView.Length)
    {
        return 90;
    }

    Span<int> mirror = Span<int>.StackAlloc(readonlyView.Length);
    mirror.CopyFrom(readonlyView);
    if (mirror.Length != readonlyView.Length)
    {
        return 91;
    }

    Span<int> window = primary.Slice(1, 2);
    if (window.Length != 2)
    {
        return 92;
    }

    Span<int> remainder = primary.Slice(2);
    if (remainder.Length != 2)
    {
        return 93;
    }

    Span<int> staging = Span<int>.StackAlloc(window.Length);
    staging.CopyFrom(window.AsReadOnly());
    if (staging.Length != window.Length)
    {
        return 94;
    }

    Span<byte> utf8Scratch = Span<byte>.StackAlloc(8);
    if (!"stack".TryCopyUtf8(utf8Scratch, out var copied) || copied != 5)
    {
        return 95;
    }
    Span<byte> scratchTail = utf8Scratch.Slice(copied);
    if (!scratchTail.IsEmpty)
    {
        return 96;
    }
    Span<byte> tiny = Span<byte>.StackAlloc(3);
    if ("stack".TryCopyUtf8(tiny, out var tinyWritten))
    {
        return 97;
    }
    if (tinyWritten != 0)
    {
        return 98;
    }

    ReadOnlySpan<byte> rawDigits = "+42".AsUtf8Span();
    ReadOnlySpan<byte> trimmedDigits = rawDigits.Slice(1);
    Span<byte> digitsBuffer = Span<byte>.StackAlloc(trimmedDigits.Length);
    digitsBuffer.CopyFrom(trimmedDigits);

    Span<byte> digitsMirror = Span<byte>.StackAlloc(trimmedDigits.Length);
    trimmedDigits.CopyTo(digitsMirror);
    digitsBuffer.CopyTo(digitsMirror);

    ReadOnlySpan<byte> copiedDigits = trimmedDigits;
    Span<byte> digitsCopy = Span<byte>.StackAlloc(trimmedDigits.Length);
    copiedDigits.CopyTo(digitsCopy);
    if (Utf8String.FromSpan(digitsCopy.AsReadOnly()) != "42")
    {
        return 99;
    }

    Span<byte> digitsClone = Span<byte>.StackAlloc(trimmedDigits);
    digitsMirror.CopyTo(digitsClone);

    Std.Int32 parsedSpan;
    if (!Std.Int32.TryParse(digitsClone.AsReadOnly(), out parsedSpan) || parsedSpan.ToInt32() != 42)
    {
        return 100;
    }

    Span<byte> formatted = Span<byte>.StackAlloc(16);
    if (!parsedSpan.TryFormat(formatted, out var written, "x"))
    {
        return 101;
    }

    ReadOnlySpan<byte> formattedView = formatted.AsReadOnly().Slice(0, written);
    Span<byte> formattedCopy = Span<byte>.StackAlloc(formattedView);
    var hexText = Utf8String.FromSpan(formattedCopy.AsReadOnly());
    if (hexText != "2a")
    {
        return 102;
    }

    Span<byte> formattedTail = formattedCopy.Slice(1);
    if (formattedTail.Length != 1 || Utf8String.FromSpan(formattedTail.AsReadOnly()) != "a")
    {
        return 103;
    }

    ReadOnlySpan<byte> emptyPayload = ReadOnlySpan<byte>.Empty;
    if (Utf8String.FromSpan(emptyPayload).Length != 0)
    {
        return 104;
    }

    return 0;
}
