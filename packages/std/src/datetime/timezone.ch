namespace Std.Datetime;
import Std.Collections;
import Std.Numeric;
import Std.Strings;
import Std.Span;
import Std.Core;
import FVec = Foundation.Collections.Vec;
import FVecIntrinsics = Foundation.Collections.VecIntrinsics;
public struct ZoneOffset
{
    public int TotalOffsetMinutes;
    public bool IsDst;
    public string Name;
}
internal struct TransitionRule
{
    internal int Month;
    internal int Week;
    internal int Weekday;
    // 0 = Sunday
    internal int Hour;
    internal int Minute;
}
internal struct TimeZoneRecord
{
    internal string Name;
    internal int BaseOffsetMinutes;
    internal bool HasDst;
    internal int DstDeltaMinutes;
    internal TransitionRule StartRule;
    internal TransitionRule EndRule;
}
public static class TimeZones
{
    private const byte ASCII_NEWLINE = 0x0Au8;
    private const byte ASCII_SEMICOLON = 0x3Bu8;
    private const byte ASCII_DASH = 0x2Du8;
    private const byte ASCII_ZERO = 0x30u8;
    private const byte ASCII_NINE = 0x39u8;
    private static VecPtr BuiltInZones() {
        var zones = FVecIntrinsics.chic_rt_vec_new((usize) __sizeof <TimeZoneRecord >(), (usize) __alignof <TimeZoneRecord >(),
        (isize) __drop_glue_of <TimeZoneRecord >());
        // UTC
        var utc = CoreIntrinsics.DefaultValue <TimeZoneRecord >();
        utc.Name = "UTC";
        utc.BaseOffsetMinutes = 0;
        utc.HasDst = false;
        utc.DstDeltaMinutes = 0;
        var zeroRule = CoreIntrinsics.DefaultValue <TransitionRule >();
        zeroRule.Month = 1;
        zeroRule.Week = 1;
        zeroRule.Weekday = 0;
        zeroRule.Hour = 0;
        zeroRule.Minute = 0;
        utc.StartRule = zeroRule;
        utc.EndRule = zeroRule;
        FVec.Push <TimeZoneRecord >(ref zones, utc);
        // America/New_York (US Eastern)
        var ny = CoreIntrinsics.DefaultValue <TimeZoneRecord >();
        ny.Name = "America/New_York";
        ny.BaseOffsetMinutes = - 300;
        ny.HasDst = true;
        ny.DstDeltaMinutes = 60;
        var start = CoreIntrinsics.DefaultValue <TransitionRule >();
        start.Month = 3;
        start.Week = 2;
        // second
        start.Weekday = 0;
        // Sunday
        start.Hour = 2;
        start.Minute = 0;
        var end = CoreIntrinsics.DefaultValue <TransitionRule >();
        end.Month = 11;
        end.Week = 1;
        // first
        end.Weekday = 0;
        end.Hour = 2;
        end.Minute = 0;
        ny.StartRule = start;
        ny.EndRule = end;
        FVec.Push <TimeZoneRecord >(ref zones, ny);
        // Europe/London (UK)
        var uk = CoreIntrinsics.DefaultValue <TimeZoneRecord >();
        uk.Name = "Europe/London";
        uk.BaseOffsetMinutes = 0;
        uk.HasDst = true;
        uk.DstDeltaMinutes = 60;
        var bstStart = CoreIntrinsics.DefaultValue <TransitionRule >();
        bstStart.Month = 3;
        bstStart.Week = - 1;
        // last
        bstStart.Weekday = 0;
        // Sunday
        bstStart.Hour = 1;
        bstStart.Minute = 0;
        var bstEnd = CoreIntrinsics.DefaultValue <TransitionRule >();
        bstEnd.Month = 10;
        bstEnd.Week = - 1;
        // last
        bstEnd.Weekday = 0;
        bstEnd.Hour = 2;
        bstEnd.Minute = 0;
        uk.StartRule = bstStart;
        uk.EndRule = bstEnd;
        FVec.Push <TimeZoneRecord >(ref zones, uk);
        return zones;
    }
    private static int ResolveNthWeekday(int year, int month, TransitionRule rule) {
        var monthTicks = 0L;
        if (! DateTimeUtil.TryDateToTicks (year, month, 1, out monthTicks)) {
            return 1;
        }
        let firstDayOfWeek = DateTimeUtil.DayOfWeekFromTicks(monthTicks);
        // 0 = Sunday
        if (rule.Week >0)
        {
            var delta = rule.Weekday - firstDayOfWeek;
            if (delta <0)
            {
                delta += 7;
            }
            return 1 + delta + (rule.Week - 1) * 7;
        }
        // last week
        let daysInMonth = DateTimeUtil.DaysInMonth(year, month);
        let lastTicks = monthTicks + (long)(daysInMonth - 1) * DateTimeConstants.TicksPerDay;
        let lastDayOfWeek = DateTimeUtil.DayOfWeekFromTicks(lastTicks);
        var deltaLast = lastDayOfWeek - rule.Weekday;
        if (deltaLast <0)
        {
            deltaLast += 7;
        }
        return daysInMonth - deltaLast;
    }
    private static long TransitionTicks(int year, TransitionRule rule) {
        var day = ResolveNthWeekday(year, rule.Month, rule);
        var dateTicks = 0L;
        if (! DateTimeUtil.TryDateToTicks (year, rule.Month, day, out dateTicks)) {
            return 0L;
        }
        var timeTicks = 0L;
        if (! DateTimeUtil.TryTimeToTicks (rule.Hour, rule.Minute, 0, 0, out timeTicks)) {
            return dateTicks;
        }
        return dateTicks + timeTicks;
    }
    private static bool TryFindZone(string name, out TimeZoneRecord record) {
        let vec = BuiltInZones();
        let span = FVec.AsReadOnlySpan <TimeZoneRecord >(in vec);
        let len = span.Length;
        var idx = 0usize;
        var hasFirst = false;
        var first = CoreIntrinsics.DefaultValue <TimeZoneRecord >();
        while (idx <len)
        {
            let zone = span[idx];
            if (zone.Name == name)
            {
                record = zone;
                return true;
            }
            if (! hasFirst)
            {
                first = zone;
                hasFirst = true;
            }
            idx += 1;
        }
        if (hasFirst || len >0usize)
        {
            if (hasFirst)
            {
                record = first;
            }
            else
            {
                record = CoreIntrinsics.DefaultValue <TimeZoneRecord >();
                record.Name = "UTC";
                record.BaseOffsetMinutes = 0;
                record.HasDst = false;
                record.DstDeltaMinutes = 0;
                var zero = CoreIntrinsics.DefaultValue <TransitionRule >();
                zero.Month = 1;
                zero.Week = 1;
                zero.Weekday = 0;
                zero.Hour = 0;
                zero.Minute = 0;
                record.StartRule = zero;
                record.EndRule = zero;
            }
        }
        else
        {
            var fallback = CoreIntrinsics.DefaultValue <TimeZoneRecord >();
            fallback.Name = "UTC";
            fallback.BaseOffsetMinutes = 0;
            fallback.HasDst = false;
            fallback.DstDeltaMinutes = 0;
            var zero = CoreIntrinsics.DefaultValue <TransitionRule >();
            zero.Month = 1;
            zero.Week = 1;
            zero.Weekday = 0;
            zero.Hour = 0;
            zero.Minute = 0;
            fallback.StartRule = zero;
            fallback.EndRule = zero;
            record = fallback;
        }
        return false;
    }
    private static ZoneOffset BuildOffset(TimeZoneRecord zone, DateTime value) {
        var output = CoreIntrinsics.DefaultValue <ZoneOffset >();
        output.Name = zone.Name;
        output.TotalOffsetMinutes = zone.BaseOffsetMinutes;
        output.IsDst = false;
        if (! zone.HasDst)
        {
            return output;
        }
        DateTimeUtil.GetDateParts(value.Ticks, out var year, out var month, out var day);
        let ticks = value.Ticks;
        let start = TransitionTicks(year, zone.StartRule);
        let end = TransitionTicks(year, zone.EndRule);
        let southern = start >end;
        var inDst = false;
        if (southern)
        {
            inDst = ! (ticks >= end && ticks <start);
        }
        else
        {
            inDst = ticks >= start && ticks <end;
        }
        if (inDst)
        {
            output.IsDst = true;
            output.TotalOffsetMinutes = zone.BaseOffsetMinutes + zone.DstDeltaMinutes;
        }
        return output;
    }
    public static ZoneOffset ResolveOffset(string zoneName, DateTime value) {
        if (! TryFindZone (zoneName, out var record)) {
            TryFindZone("UTC", out record);
        }
        return BuildOffset(record, value);
    }
    public static uint Version => 1;
    public static bool InstallFromData(string data) {
        // Data format: name;baseOffset;dstDelta;startMonth;startWeek;startWeekday;startHour;startMinute;endMonth;endWeek;endWeekday;endHour;endMinute
        // Lines separated by '\n'. dstDelta=0 disables DST.
        var fresh = FVecIntrinsics.chic_rt_vec_new((usize) __sizeof <TimeZoneRecord >(), (usize) __alignof <TimeZoneRecord >(),
        (isize) __drop_glue_of <TimeZoneRecord >());
        let bytes = Std.Span.ReadOnlySpan.FromString(data);
        var index = 0usize;
        while (index <bytes.Length)
        {
            // read line
            var lineStart = index;
            while (index <bytes.Length && bytes[index] != ASCII_NEWLINE)
            {
                index += 1;
            }
            let line = bytes.Slice(lineStart, index - lineStart);
            if (index <bytes.Length && bytes[index] == ASCII_NEWLINE)
            {
                index += 1;
            }
            if (line.IsEmpty)
            {
                continue;
            }
            if (! TryParseRecord (line, out var rec)) {
                return false;
            }
            FVec.Push <TimeZoneRecord >(ref fresh, rec);
        }
        return true;
    }
    private static bool TryParseRecord(ReadOnlySpan <byte >line, out TimeZoneRecord record) {
        var recInit = CoreIntrinsics.DefaultValue <TimeZoneRecord >();
        recInit.Name = "";
        recInit.BaseOffsetMinutes = 0;
        recInit.HasDst = false;
        recInit.DstDeltaMinutes = 0;
        var initRule = CoreIntrinsics.DefaultValue <TransitionRule >();
        initRule.Month = 1;
        initRule.Week = 1;
        initRule.Weekday = 0;
        initRule.Hour = 0;
        initRule.Minute = 0;
        recInit.StartRule = initRule;
        recInit.EndRule = initRule;
        record = recInit;
        var cursor = 0usize;
        // parse name
        var sep = 0usize;
        while (sep <line.Length && line[sep] != ASCII_SEMICOLON)
        {
            sep += 1;
        }
        if (sep == 0 || sep == line.Length)
        {
            return false;
        }
        let name = Utf8String.FromSpan(line.Slice(0, sep));
        cursor = sep + 1;
        var baseOffset = 0;
        var dstDelta = 0;
        var startMonth = 0;
        var startWeek = 0;
        var startWeekday = 0;
        var startHour = 0;
        var startMinute = 0;
        var endMonth = 0;
        var endWeek = 0;
        var endWeekday = 0;
        var endHour = 0;
        var endMinute = 0;
        var fieldIndex = 0;
        for (var i = 0; i <12; i += 1) {
            var value = 0;
            var negative = false;
            if (cursor >= line.Length)
            {
                return false;
            }
            if (line[cursor] == ASCII_DASH)
            {
                negative = true;
                cursor += 1;
            }
            if (cursor >= line.Length)
            {
                return false;
            }
            while (cursor <line.Length && line[cursor] != ASCII_SEMICOLON)
            {
                let b = line[cursor];
                if (b <ASCII_ZERO || b >ASCII_NINE)
                {
                    return false;
                }
                value = value * 10 + (int)(b - ASCII_ZERO);
                cursor += 1;
            }
            let parsed = negative ?- value : value;
            if (fieldIndex == 0)
            {
                baseOffset = parsed;
            }
            else if (fieldIndex == 1)
            {
                dstDelta = parsed;
            }
            else if (fieldIndex == 2)
            {
                startMonth = parsed;
            }
            else if (fieldIndex == 3)
            {
                startWeek = parsed;
            }
            else if (fieldIndex == 4)
            {
                startWeekday = parsed;
            }
            else if (fieldIndex == 5)
            {
                startHour = parsed;
            }
            else if (fieldIndex == 6)
            {
                startMinute = parsed;
            }
            else if (fieldIndex == 7)
            {
                endMonth = parsed;
            }
            else if (fieldIndex == 8)
            {
                endWeek = parsed;
            }
            else if (fieldIndex == 9)
            {
                endWeekday = parsed;
            }
            else if (fieldIndex == 10)
            {
                endHour = parsed;
            }
            else if (fieldIndex == 11)
            {
                endMinute = parsed;
            }
            fieldIndex += 1;
            cursor += 1;
        }
        record.Name = name;
        record.BaseOffsetMinutes = baseOffset;
        record.DstDeltaMinutes = dstDelta;
        record.HasDst = record.DstDeltaMinutes != 0;
        var start = CoreIntrinsics.DefaultValue <TransitionRule >();
        start.Month = startMonth;
        start.Week = startWeek;
        start.Weekday = startWeekday;
        start.Hour = startHour;
        start.Minute = startMinute;
        var end = CoreIntrinsics.DefaultValue <TransitionRule >();
        end.Month = endMonth;
        end.Week = endWeek;
        end.Weekday = endWeekday;
        end.Hour = endHour;
        end.Minute = endMinute;
        record.StartRule = start;
        record.EndRule = end;
        return true;
    }
}
