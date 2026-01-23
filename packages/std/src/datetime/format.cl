namespace Std.Datetime;
import Std.Span;
import Std.Strings;
import Std.Numeric;
import Std.Memory;
import Std.Core;
internal static class DateTimeFormatting
{
    private const usize MAX_BUFFER = 128;
    private static void WriteTwoDigits(int value, Span <byte >destination, ref usize offset) {
        let tens = value / 10;
        let ones = value - tens * 10;
        destination[offset] = NumericUnchecked.ToByte(48 + tens);
        destination[offset + 1] = NumericUnchecked.ToByte(48 + ones);
        offset += 2;
    }
    private static void WriteFourDigits(int value, Span <byte >destination, ref usize offset) {
        let thousands = value / 1000;
        let hundreds = (value / 100) % 10;
        let tens = (value / 10) % 10;
        let ones = value % 10;
        destination[offset] = NumericUnchecked.ToByte(48 + thousands);
        destination[offset + 1] = NumericUnchecked.ToByte(48 + hundreds);
        destination[offset + 2] = NumericUnchecked.ToByte(48 + tens);
        destination[offset + 3] = NumericUnchecked.ToByte(48 + ones);
        offset += 4;
    }
    private static void WriteOffset(int minutes, Span <byte >destination, ref usize offset) {
        if (minutes == 0)
        {
            destination[offset] = NumericUnchecked.ToByte(90);
            offset += 1;
            return;
        }
        if (minutes <0)
        {
            destination[offset] = NumericUnchecked.ToByte(45);
            minutes = - minutes;
        }
        else
        {
            destination[offset] = NumericUnchecked.ToByte(43);
        }
        offset += 1;
        let hours = minutes / 60;
        let mins = minutes % 60;
        WriteTwoDigits(hours, destination, ref offset);
        destination[offset] = NumericUnchecked.ToByte(58);
        offset += 1;
        WriteTwoDigits(mins, destination, ref offset);
    }
    private static void WriteFraction(int fractionTicks, int digits, bool trimTrailing, Span <byte >dest, ref usize offset) {
        var working = fractionTicks;
        var divisor = 1;
        if (digits >7)
        {
            digits = 7;
        }
        for (var i = 0; i <digits; i += 1) {
            divisor *= 10;
        }
        let scaled = fractionTicks * (10000000 / divisor);
        var value = scaled;
        var scratch = Std.Memory.StackAlloc.Span <byte >(7);
        var scratchLen = digits;
        for (var i = digits - 1; i >= 0; i -= 1) {
            let digit = value % 10;
            scratch[i] = NumericUnchecked.ToByte(48 + digit);
            value /= 10;
        }
        var start = 0;
        if (trimTrailing)
        {
            let zeroByte = NumericUnchecked.ToByte(48);
            while (scratchLen >0 && scratch[scratchLen - 1] == zeroByte)
            {
                scratchLen -= 1;
            }
        }
        for (var i = start; i <scratchLen; i += 1) {
            dest[offset] = scratch[i];
            offset += 1;
        }
    }
    private static void WriteComponent(DateTime value, string format, Span <byte >dest, ref usize offset, IDateTimeCulture culture) {
        if (format == null || format == "")
        {
            format = "O";
        }
        // Break into date/time parts once to avoid recomputation.
        let ticks = value.Ticks;
        DateTimeUtil.GetDateParts(ticks, out var year, out var month, out var day);
        let ticksOfDay = ticks % DateTimeConstants.TicksPerDay;
        DateTimeUtil.GetTimeParts(ticksOfDay, out var hour, out var minute, out var second, out var fractionTicks);
        var index = 0usize;
        while (index <format.Length)
        {
            let c = format[index];
            var run = 1usize;
            while (index + run <format.Length && format[index + run] == c)
            {
                run += 1;
            }
            if (c == 'y')
            {
                if (run == 2)
                {
                    let twoDigitYear = year % 100;
                    WriteTwoDigits(twoDigitYear, dest, ref offset);
                }
                else
                {
                    WriteFourDigits(year, dest, ref offset);
                }
            }
            else if (c == 'M')
            {
                if (run == 2)
                {
                    WriteTwoDigits(month, dest, ref offset);
                }
                else
                {
                    dest[offset] = NumericUnchecked.ToByte(48 + month);
                    offset += 1;
                }
            }
            else if (c == 'd')
            {
                if (run == 2)
                {
                    WriteTwoDigits(day, dest, ref offset);
                }
                else
                {
                    dest[offset] = NumericUnchecked.ToByte(48 + day);
                    offset += 1;
                }
            }
            else if (c == 'H')
            {
                WriteTwoDigits(hour, dest, ref offset);
            }
            else if (c == 'm')
            {
                WriteTwoDigits(minute, dest, ref offset);
            }
            else if (c == 's')
            {
                WriteTwoDigits(second, dest, ref offset);
            }
            else if (c == 'f' || c == 'F')
            {
                let trim = c == 'F';
                let digits = NumericUnchecked.ToInt32(run);
                WriteFraction(NumericUnchecked.ToInt32(fractionTicks), digits, trim, dest, ref offset);
            }
            else if (c == 'K' || c == 'z')
            {
                let minutes = value.Kind == DateTimeKind.Utc ?0 : value.OffsetMinutes;
                if (value.Kind == DateTimeKind.Unspecified)
                {
                    if (c == 'K')
                    {
                        // omit
                    }
                    else
                    {
                        WriteOffset(minutes, dest, ref offset);
                    }
                }
                else
                {
                    WriteOffset(minutes, dest, ref offset);
                }
            }
            else
            {
                for (var i = 0usize; i <run; i += 1) {
                    dest[offset] = NumericUnchecked.ToByte(c);
                    offset += 1;
                }
            }
            index += run;
        }
    }
    public static bool TryFormatDateTime(DateTime value, Span <byte >destination, out usize written, string format, IDateTimeCulture culture) {
        written = 0;
        if (culture == null)
        {
            culture = InvariantDateTimeCulture.Instance;
        }
        var offset = 0usize;
        if (format == null || format == "")
        {
            format = "O";
        }
        var effectiveFormat = format;
        if (format == "O" || format == "o")
        {
            effectiveFormat = "yyyy-MM-ddTHH:mm:ss.fffffffK";
        }
        else if (format == "s")
        {
            effectiveFormat = "yyyy-MM-ddTHH:mm:ss";
        }
        else if (format == "R" || format == "r")
        {
            effectiveFormat = "yyyy-MM-ddTHH:mm:ssK";
        }
        WriteComponent(value, effectiveFormat, destination, ref offset, culture);
        written = offset;
        return true;
    }
    public static string FormatDateTime(DateTime value, string format, IDateTimeCulture culture) {
        var buffer = Std.Memory.StackAlloc.Span <byte >(MAX_BUFFER);
        var written = 0usize;
        if (!TryFormatDateTime (value, buffer, out written, format, culture)) {
            throw new Std.InvalidOperationException("failed to format DateTime");
        }
        return Finish(buffer, written);
    }
    public static bool TryFormatDuration(Duration value, Span <byte >destination, out usize written, string format) {
        written = 0;
        var ticks = value.Ticks;
        let negative = ticks <0;
        if (negative)
        {
            ticks = - ticks;
        }
        var days = ticks / DateTimeConstants.TicksPerDay;
        ticks -= days * DateTimeConstants.TicksPerDay;
        let hours = NumericUnchecked.ToInt32(ticks / DateTimeConstants.TicksPerHour);
        ticks -= NumericUnchecked.ToInt64(hours) * DateTimeConstants.TicksPerHour;
        let minutes = NumericUnchecked.ToInt32(ticks / DateTimeConstants.TicksPerMinute);
        ticks -= NumericUnchecked.ToInt64(minutes) * DateTimeConstants.TicksPerMinute;
        let seconds = NumericUnchecked.ToInt32(ticks / DateTimeConstants.TicksPerSecond);
        let fraction = NumericUnchecked.ToInt32(ticks % DateTimeConstants.TicksPerSecond);
        var offset = 0usize;
        if (negative)
        {
            destination[offset] = NumericUnchecked.ToByte(45);
            offset += 1;
        }
        // days
        var daysText = Std.Int64.From(days).ToString();
        let daysSpan = Std.Span.ReadOnlySpan.FromString(daysText);
        destination.Slice(offset, daysSpan.Length).CopyFrom(daysSpan);
        offset += daysSpan.Length;
        destination[offset] = NumericUnchecked.ToByte(46);
        offset += 1;
        WriteTwoDigits(hours, destination, ref offset);
        destination[offset] = NumericUnchecked.ToByte(58);
        offset += 1;
        WriteTwoDigits(minutes, destination, ref offset);
        destination[offset] = NumericUnchecked.ToByte(58);
        offset += 1;
        WriteTwoDigits(seconds, destination, ref offset);
        if (fraction != 0)
        {
            destination[offset] = NumericUnchecked.ToByte(46);
            offset += 1;
            WriteFraction(fraction, 7, true, destination, ref offset);
        }
        written = offset;
        return true;
    }
    public static string FormatDuration(Duration value, string format) {
        var buffer = Std.Memory.StackAlloc.Span <byte >(MAX_BUFFER);
        var written = 0usize;
        if (!TryFormatDuration (value, buffer, out written, format)) {
            throw new Std.InvalidOperationException("failed to format Duration");
        }
        return Finish(buffer, written);
    }
    private static string Finish(Span <byte >buffer, usize written) {
        var readonlyView = buffer.AsReadOnly();
        var handle = readonlyView.Raw;
        if (written >handle.Length)
        {
            written = handle.Length;
        }
        var slice = CoreIntrinsics.DefaultValue <StrPtr >();
        slice.Pointer = handle.Data.Pointer;
        slice.Length = written;
        return SpanIntrinsics.chic_rt_string_from_slice(slice);
    }
}
