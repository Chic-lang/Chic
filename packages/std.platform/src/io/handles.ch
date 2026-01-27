namespace Std.Platform.IO;
import Std.Collections;
import Std.Runtime.Collections;
import Std.Runtime.Native;
import Std.Span;
import Std.Strings;
import Std.Core;
internal struct StdinHandle
{
    internal int Fd;
    internal bool Terminal;
    internal static StdinHandle System() {
        var handle = CoreIntrinsics.DefaultValue <StdinHandle >();
        handle.Fd = Platform.FdStdin;
        handle.Terminal = Platform.IsTerminal(Platform.FdStdin);
        return handle;
    }
    public IoError Read(Span <byte >destination, out usize readCount) {
        var localRead = 0usize;
        let status = Platform.ReadInto(Fd, destination, out localRead);
        readCount = localRead;
        return status;
    }
    public IoError ReadExact(Span <byte >destination) {
        var total = 0usize;
        while (total <destination.Length)
        {
            var readCount = 0usize;
            var status = Platform.ReadInto(Fd, destination.Slice(total, destination.Length - total), out readCount);
            if (status == IoError.Success)
            {
                total += readCount;
                continue;
            }
            if (status == IoError.Eof)
            {
                return IoError.UnexpectedEof;
            }
            return status;
        }
        return IoError.Success;
    }
    public IoError ReadLine(ref ChicString destination) {
        var buffer = VecIntrinsics.Create <byte >();
        var slot = Span <byte >.StackAlloc(1);
        var sawAny = false;
        while (true)
        {
            var readCount = 0usize;
            var status = Platform.ReadInto(Fd, slot, out readCount);
            if (status == IoError.Eof || readCount == 0)
            {
                if (!sawAny)
                {
                    VecIntrinsics.chic_rt_vec_drop(ref buffer);
                    return IoError.Eof;
                }
                break;
            }
            if (status != IoError.Success)
            {
                VecIntrinsics.chic_rt_vec_drop(ref buffer);
                return status;
            }
            sawAny = true;
            Vec.Push <byte >(ref buffer, slot[0]);
            if (slot[0] == 10)
            {
                break;
            }
        }
        var result = CoreIntrinsics.DefaultValue <ChicString >();
        {
            let span = Vec.AsReadOnlySpan <byte >(in buffer);
            var copy = Span <byte >.StackAlloc(span.Length);
            copy.Slice(0, span.Length).CopyFrom(span);
            let slice = IoTyped.ToRuntimeStr(copy.AsReadOnly());
            result = StringIntrinsics.chic_rt_string_from_slice(slice);
        }
        VecIntrinsics.chic_rt_vec_drop(ref buffer);
        destination = result;
        return IoError.Success;
    }
}
internal struct StdoutHandle
{
    internal int Fd;
    internal bool LineBuffered;
    internal bool FlushOnNewline;
    internal bool NormalizeNewlines;
    internal bool Terminal;
    internal static StdoutHandle System() {
        var handle = CoreIntrinsics.DefaultValue <StdoutHandle >();
        handle.Fd = Platform.FdStdout;
        handle.Terminal = Platform.IsTerminal(Platform.FdStdout);
        handle.LineBuffered = handle.Terminal;
        handle.FlushOnNewline = handle.Terminal;
        handle.NormalizeNewlines = false;
        return handle;
    }
    public IoError WriteString(string value) {
        var text = value;
        let slice = SpanIntrinsics.chic_rt_string_as_slice(text);
        let bytes = IoTyped.FromStringSlice(slice);
        return WriteBytes(bytes, false);
    }
    public IoError WriteLineString(string value) {
        var text = value;
        let slice = SpanIntrinsics.chic_rt_string_as_slice(text);
        let bytes = IoTyped.FromStringSlice(slice);
        return WriteBytes(bytes, true);
    }
    public IoError WriteLine() {
        return WriteBytes(ReadOnlySpan <byte >.Empty, true);
    }
    public IoError WriteLineBytes(ReadOnlySpan <byte >buffer) {
        return WriteBytes(buffer, true);
    }
    public IoError WriteBytes(ReadOnlySpan <byte >buffer, bool appendNewline) {
        return NormalizeAndWrite(buffer, appendNewline);
    }
    public IoError Flush() {
        return Platform.FlushFd(Fd);
    }
    public void SetLineBuffered(bool enabled) {
        LineBuffered = enabled;
    }
    public void SetFlushOnNewline(bool enabled) {
        FlushOnNewline = enabled;
    }
    public void SetNormalizeNewlines(bool enabled) {
        NormalizeNewlines = enabled;
    }
    public bool IsTerminal() {
        return Terminal;
    }
    private IoError NormalizeAndWrite(ReadOnlySpan <byte >buffer, bool appendNewline) {
        var newlinePending = appendNewline;
        var scratch = Span <byte >.StackAlloc(256);
        var lineBreakSeen = newlinePending;
        var remaining = buffer;
        while (remaining.Length >0 || newlinePending)
        {
            var produced = 0usize;
            while (produced <scratch.Length && (remaining.Length >0 || newlinePending))
            {
                var current = remaining.Length >0 ?remaining[0] : NumericUnchecked.ToByte(10);
                if (remaining.Length >0)
                {
                    remaining = remaining.Slice(1, remaining.Length - 1);
                }
                else
                {
                    newlinePending = false;
                }
                if (NormalizeNewlines && current == 10)
                {
                    if (produced + 2 >scratch.Length)
                    {
                        break;
                    }
                    scratch[produced] = 13;
                    scratch[produced + 1] = 10;
                    produced += 2;
                    lineBreakSeen = true;
                    continue;
                }
                if (current == 10)
                {
                    lineBreakSeen = true;
                }
                scratch[produced] = current;
                produced += 1;
            }
            if (produced == 0)
            {
                continue;
            }
            let chunk = scratch.AsReadOnly().Slice(0, produced);
            var status = Platform.WriteAll(Fd, chunk);
            if (status != IoError.Success)
            {
                return status;
            }
        }
        if (FlushOnNewline && lineBreakSeen)
        {
            return IoError.Success;
        }
        if (!LineBuffered)
        {
            return IoError.Success;
        }
        return IoError.Success;
    }
}
internal struct StderrHandle
{
    internal StdoutHandle Inner;
    internal static StderrHandle System() {
        var handle = CoreIntrinsics.DefaultValue <StderrHandle >();
        handle.Inner = StdoutHandle.System();
        handle.Inner.Fd = Platform.FdStderr;
        handle.Inner.LineBuffered = true;
        handle.Inner.FlushOnNewline = true;
        handle.Inner.NormalizeNewlines = false;
        handle.Inner.Terminal = Platform.IsTerminal(Platform.FdStderr);
        return handle;
    }
    public IoError WriteString(string value) {
        return Inner.WriteString(value);
    }
    public IoError WriteLineString(string value) {
        return Inner.WriteLineString(value);
    }
    public IoError WriteLine() {
        return Inner.WriteLine();
    }
    public IoError WriteBytes(ReadOnlySpan <byte >buffer) {
        return Inner.WriteBytes(buffer, false);
    }
    public IoError WriteLineBytes(ReadOnlySpan <byte >buffer) {
        return Inner.WriteBytes(buffer, true);
    }
    public IoError Flush() {
        return Inner.Flush();
    }
    public void SetNormalizeNewlines(bool enabled) {
        Inner.SetNormalizeNewlines(enabled);
    }
    public bool IsTerminal() {
        return Inner.IsTerminal();
    }
}
