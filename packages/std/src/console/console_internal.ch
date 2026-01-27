namespace Std;
import Std.Core;
import Std.Sync;
import Std.Platform.IO;
import Std.Strings;
import Std.Collections;
import Std.Numeric;
import Std.Runtime;
import Foundation.Collections;
import FVec = Foundation.Collections.Vec;
import FVecIntrinsics = Foundation.Collections.VecIntrinsics;
internal enum ConsoleStream
{
    StandardOut, StandardError,
}
internal static class __ConsoleCore
{
    private static bool _initialized;
    private static TextReader _in;
    private static TextWriter _out;
    private static TextWriter _error;
    private static string _newline;
    private static bool _autoFlush;
    private static Lock _lock;
    private static TerminalCapabilities _caps;
    private static ConsoleColor _foreground;
    private static ConsoleColor _background;
    private static bool _cursorVisible;
    private static int _cursorLeft;
    private static int _cursorTop;
    private static void EnsureInit() {
        if (_initialized)
        {
            return;
        }
        _lock = new Lock();
        _in = new ConsoleInReader();
        _out = new ConsoleStreamWriter(false);
        _error = new ConsoleStreamWriter(true);
        _newline = Std.Environment.NewLine();
        if (_newline == null || _newline == "")
        {
            _newline = "\n";
        }
        _autoFlush = false;
        _caps = TerminalCapabilities.Detect();
        _foreground = ConsoleColor.Gray;
        _background = ConsoleColor.Black;
        _cursorVisible = true;
        _cursorLeft = 0;
        _cursorTop = 0;
        _initialized = true;
    }
    internal static TextReader InReader() {
        EnsureInit();
        return _in;
    }
    internal static TextWriter OutWriter() {
        EnsureInit();
        return _out;
    }
    internal static TextWriter ErrorWriter() {
        EnsureInit();
        return _error;
    }
    internal static void SetIn(TextReader reader) {
        if (reader == null)
        {
            throw new ArgumentNullException("reader");
        }
        EnsureInit();
        var guard = _lock.Enter();
        _in = reader;
        guard.Release();
    }
    internal static void SetOut(TextWriter writer) {
        if (writer == null)
        {
            throw new ArgumentNullException("writer");
        }
        EnsureInit();
        var guard = _lock.Enter();
        _out = writer;
        guard.Release();
    }
    internal static void SetError(TextWriter writer) {
        if (writer == null)
        {
            throw new ArgumentNullException("writer");
        }
        EnsureInit();
        var guard = _lock.Enter();
        _error = writer;
        guard.Release();
    }
    internal static string GetNewLine() {
        EnsureInit();
        return _newline;
    }
    internal static void SetNewLine(string value) {
        EnsureInit();
        if (value == null)
        {
            throw new ArgumentNullException("value");
        }
        var guard = _lock.Enter();
        _newline = value;
        guard.Release();
    }
    internal static bool GetAutoFlush() {
        EnsureInit();
        return _autoFlush;
    }
    internal static void SetAutoFlush(bool value) {
        EnsureInit();
        var guard = _lock.Enter();
        _autoFlush = value;
        guard.Release();
    }
    internal static void Write(string ?value, ConsoleStream target) {
        EnsureInit();
        var guard = _lock.Enter();
        var writer = ResolveWriter(target);
        writer.Write(value);
        FlushIfNeeded(writer);
        guard.Release();
    }
    internal static void WriteLine(string ?value, ConsoleStream target) {
        EnsureInit();
        var guard = _lock.Enter();
        var writer = ResolveWriter(target);
        writer.Write(value);
        writer.Write(_newline);
        FlushIfNeeded(writer);
        guard.Release();
    }
    internal static void WriteLine(ConsoleStream target) {
        EnsureInit();
        var guard = _lock.Enter();
        var writer = ResolveWriter(target);
        writer.Write(_newline);
        FlushIfNeeded(writer);
        guard.Release();
    }
    internal static void WriteUtf8(ReadOnlySpan <byte >bytes, ConsoleStream target) {
        EnsureInit();
        var asString = Utf8String.FromSpan(bytes);
        Write(asString, target);
    }
    internal static string ?ReadLine() {
        EnsureInit();
        return _in.ReadLine();
    }
    internal static int Read() {
        EnsureInit();
        return _in.Read();
    }
    internal static bool IsInputRedirected() {
        EnsureInit();
        if (! (_in is ConsoleInReader)) {
            return true;
        }
        return !_caps.InputIsTerminal;
    }
    internal static bool IsOutputRedirected() {
        EnsureInit();
        return !_out.IsTerminal;
    }
    internal static bool IsErrorRedirected() {
        EnsureInit();
        return !_error.IsTerminal;
    }
    internal static string Format(string format, object[] args) {
        return ConsoleFormatter.Format(format, args);
    }
    internal static bool KeyAvailable() {
        EnsureInit();
        if (!_caps.SupportsReadKey)
        {
            throw new NotSupportedException("Console key input is not supported on this target");
        }
        return false;
    }
    internal static ConsoleKeyInfo ReadKey(bool intercept) {
        EnsureInit();
        if (!_caps.SupportsReadKey)
        {
            throw new NotSupportedException("Console key input is not supported on this target");
        }
        return new ConsoleKeyInfo((char) 0, false, false, false);
    }
    internal static ConsoleColor GetForegroundColor() {
        EnsureInit();
        EnsureColorsSupported();
        return _foreground;
    }
    internal static void SetForegroundColor(ConsoleColor color) {
        EnsureInit();
        EnsureColorsSupported();
        _foreground = color;
        ApplyColors();
    }
    internal static ConsoleColor GetBackgroundColor() {
        EnsureInit();
        EnsureColorsSupported();
        return _background;
    }
    internal static void SetBackgroundColor(ConsoleColor color) {
        EnsureInit();
        EnsureColorsSupported();
        _background = color;
        ApplyColors();
    }
    internal static void ResetColor() {
        EnsureInit();
        if (!_caps.SupportsColor)
        {
            throw new NotSupportedException("Console colors are not supported on this target");
        }
        _foreground = ConsoleColor.Gray;
        _background = ConsoleColor.Black;
        var guard = _lock.Enter();
        let writer = ResolveWriter(ConsoleStream.StandardOut);
        writer.Write("\u001b[0m");
        FlushIfNeeded(writer);
        guard.Release();
    }
    internal static void Clear() {
        EnsureInit();
        if (!_caps.SupportsClear)
        {
            throw new NotSupportedException("Console clear is not supported on this target");
        }
        var guard = _lock.Enter();
        let writer = ResolveWriter(ConsoleStream.StandardOut);
        writer.Write("\u001b[2J\u001b[H");
        FlushIfNeeded(writer);
        guard.Release();
        _cursorLeft = 0;
        _cursorTop = 0;
    }
    internal static int GetCursorLeft() {
        EnsureInit();
        if (!_caps.SupportsCursor)
        {
            throw new NotSupportedException("Console cursor positioning is not supported");
        }
        return _cursorLeft;
    }
    internal static int GetCursorTop() {
        EnsureInit();
        if (!_caps.SupportsCursor)
        {
            throw new NotSupportedException("Console cursor positioning is not supported");
        }
        return _cursorTop;
    }
    internal static void SetCursorPosition(int left, int top) {
        EnsureInit();
        if (!_caps.SupportsCursor)
        {
            throw new NotSupportedException("Console cursor positioning is not supported");
        }
        if (left <0 || top <0)
        {
            throw new ArgumentOutOfRangeException("Cursor coordinates must be non-negative");
        }
        var guard = _lock.Enter();
        var writer = ResolveWriter(ConsoleStream.StandardOut);
        writer.Write(BuildCursorSequence(left, top));
        FlushIfNeeded(writer);
        guard.Release();
        _cursorLeft = left;
        _cursorTop = top;
    }
    internal static bool GetCursorVisible() {
        EnsureInit();
        if (!_caps.SupportsCursor)
        {
            throw new NotSupportedException("Console cursor visibility is not supported");
        }
        return _cursorVisible;
    }
    internal static void SetCursorVisible(bool visible) {
        EnsureInit();
        if (!_caps.SupportsCursor)
        {
            throw new NotSupportedException("Console cursor visibility is not supported");
        }
        var guard = _lock.Enter();
        var writer = ResolveWriter(ConsoleStream.StandardOut);
        if (visible)
        {
            writer.Write("\u001b[?25h");
        }
        else
        {
            writer.Write("\u001b[?25l");
        }
        FlushIfNeeded(writer);
        guard.Release();
        _cursorVisible = visible;
    }
    internal static int GetBufferWidth() {
        EnsureInit();
        if (!_caps.SupportsSizing)
        {
            throw new NotSupportedException("Console sizing is not supported on this target");
        }
        return 0;
    }
    internal static int GetBufferHeight() {
        EnsureInit();
        if (!_caps.SupportsSizing)
        {
            throw new NotSupportedException("Console sizing is not supported on this target");
        }
        return 0;
    }
    internal static void SetBufferWidth(int width) {
        EnsureInit();
        throw new NotSupportedException("Console buffer sizing is not supported on this target");
    }
    internal static void SetBufferHeight(int height) {
        EnsureInit();
        throw new NotSupportedException("Console buffer sizing is not supported on this target");
    }
    internal static int GetWindowWidth() {
        EnsureInit();
        if (!_caps.SupportsSizing)
        {
            throw new NotSupportedException("Console sizing is not supported on this target");
        }
        return 0;
    }
    internal static int GetWindowHeight() {
        EnsureInit();
        if (!_caps.SupportsSizing)
        {
            throw new NotSupportedException("Console sizing is not supported on this target");
        }
        return 0;
    }
    internal static void SetWindowWidth(int width) {
        EnsureInit();
        throw new NotSupportedException("Console sizing is not supported on this target");
    }
    internal static void SetWindowHeight(int height) {
        EnsureInit();
        throw new NotSupportedException("Console sizing is not supported on this target");
    }
    internal static int GetLargestWindowWidth() {
        EnsureInit();
        if (!_caps.SupportsSizing)
        {
            throw new NotSupportedException("Console sizing is not supported on this target");
        }
        return 0;
    }
    internal static int GetLargestWindowHeight() {
        EnsureInit();
        if (!_caps.SupportsSizing)
        {
            throw new NotSupportedException("Console sizing is not supported on this target");
        }
        return 0;
    }
    private static TextWriter ResolveWriter(ConsoleStream target) {
        if (target == ConsoleStream.StandardError)
        {
            return _error;
        }
        return _out;
    }
    private static void FlushIfNeeded(TextWriter writer) {
        if (_autoFlush)
        {
            writer.Flush();
        }
    }
    private static void EnsureColorsSupported() {
        if (!_caps.SupportsColor)
        {
            throw new NotSupportedException("Console colors are not supported on this target");
        }
        if (!_out.IsTerminal)
        {
            throw new NotSupportedException("Console colors require an attached terminal");
        }
    }
    private static void ApplyColors() {
        var guard = _lock.Enter();
        var writer = ResolveWriter(ConsoleStream.StandardOut);
        writer.Write(GetAnsiForeground(_foreground));
        writer.Write(GetAnsiBackground(_background));
        FlushIfNeeded(writer);
        guard.Release();
    }
    private static string GetAnsiForeground(ConsoleColor color) {
        switch (color)
        {
            case ConsoleColor.Black:
                return "\u001b[30m";
            case ConsoleColor.DarkBlue:
                return "\u001b[34m";
            case ConsoleColor.DarkGreen:
                return "\u001b[32m";
            case ConsoleColor.DarkCyan:
                return "\u001b[36m";
            case ConsoleColor.DarkRed:
                return "\u001b[31m";
            case ConsoleColor.DarkMagenta:
                return "\u001b[35m";
            case ConsoleColor.DarkYellow:
                return "\u001b[33m";
            case ConsoleColor.Gray:
                return "\u001b[37m";
            case ConsoleColor.DarkGray:
                return "\u001b[90m";
            case ConsoleColor.Blue:
                return "\u001b[94m";
            case ConsoleColor.Green:
                return "\u001b[92m";
            case ConsoleColor.Cyan:
                return "\u001b[96m";
            case ConsoleColor.Red:
                return "\u001b[91m";
            case ConsoleColor.Magenta:
                return "\u001b[95m";
            case ConsoleColor.Yellow:
                return "\u001b[93m";
            case ConsoleColor.White:
                return "\u001b[97m";
            default :
                return "\u001b[39m";
            }
        }
        private static string GetAnsiBackground(ConsoleColor color) {
            switch (color)
            {
                case ConsoleColor.Black:
                    return "\u001b[40m";
                case ConsoleColor.DarkBlue:
                    return "\u001b[44m";
                case ConsoleColor.DarkGreen:
                    return "\u001b[42m";
                case ConsoleColor.DarkCyan:
                    return "\u001b[46m";
                case ConsoleColor.DarkRed:
                    return "\u001b[41m";
                case ConsoleColor.DarkMagenta:
                    return "\u001b[45m";
                case ConsoleColor.DarkYellow:
                    return "\u001b[43m";
                case ConsoleColor.Gray:
                    return "\u001b[47m";
                case ConsoleColor.DarkGray:
                    return "\u001b[100m";
                case ConsoleColor.Blue:
                    return "\u001b[104m";
                case ConsoleColor.Green:
                    return "\u001b[102m";
                case ConsoleColor.Cyan:
                    return "\u001b[106m";
                case ConsoleColor.Red:
                    return "\u001b[101m";
                case ConsoleColor.Magenta:
                    return "\u001b[105m";
                case ConsoleColor.Yellow:
                    return "\u001b[103m";
                case ConsoleColor.White:
                    return "\u001b[107m";
                default :
                    return "\u001b[49m";
                }
            }
            private static string BuildCursorSequence(int left, int top) {
                var buffer = FVec.WithCapacity <byte >(16);
                FVec.Push <byte >(ref buffer, NumericUnchecked.ToByte(27));
                FVec.Push <byte >(ref buffer, NumericUnchecked.ToByte(91));
                // '['
                PushDecimal(ref buffer, top + 1);
                FVec.Push <byte >(ref buffer, NumericUnchecked.ToByte(59));
                // ';'
                PushDecimal(ref buffer, left + 1);
                FVec.Push <byte >(ref buffer, NumericUnchecked.ToByte(72));
                // 'H'
                let span = Foundation.Collections.Vec.AsReadOnlySpan <byte >(in buffer);
                let text = Utf8String.FromSpan(span);
                FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                return text;
            }
            private static void PushDecimal(ref VecPtr buffer, int value) {
                var tmp = Span <byte >.StackAlloc(16);
                var count = 0usize;
                var remaining = value;
                if (remaining == 0)
                {
                    FVec.Push <byte >(ref buffer, NumericUnchecked.ToByte(48));
                    // '0'
                    return;
                }
                while (remaining >0 && count <tmp.Length)
                {
                    let digit = remaining % 10;
                    tmp[count] = NumericUnchecked.ToByte(48 + digit);
                    count += 1;
                    remaining = remaining / 10;
                }
                while (count >0)
                {
                    count -= 1;
                    FVec.Push <byte >(ref buffer, tmp[count]);
                }
            }
        }
        internal static class ConsoleFormatter
        {
            public static string Format(string format, object[] args) {
                if (format == null)
                {
                    throw new ArgumentNullException("format");
                }
                if (args == null)
                {
                    throw new ArgumentNullException("args");
                }
                var utf8 = format.AsUtf8Span();
                var buffer = FVec.WithCapacity <byte >(utf8.Length + 8);
                var index = 0usize;
                while (index <utf8.Length)
                {
                    let current = utf8[index];
                    if (current == NumericUnchecked.ToByte (123))
                    {
                        if (index + 1 <utf8.Length && utf8[index + 1] == NumericUnchecked.ToByte (123))
                        {
                            FVec.Push <byte >(ref buffer, NumericUnchecked.ToByte(123));
                            index += 2;
                            continue;
                        }
                        index += 1;
                        if (index >= utf8.Length)
                        {
                            FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                            throw new FormatException("Incomplete format item");
                        }
                        var argIndex = 0;
                        var anyDigit = false;
                        while (index <utf8.Length)
                        {
                            let digit = utf8[index];
                            if (digit >= NumericUnchecked.ToByte (48) && digit <= NumericUnchecked.ToByte (57))
                            {
                                argIndex = argIndex * 10 + NumericUnchecked.ToInt32(digit - NumericUnchecked.ToByte(48));
                                index += 1;
                                anyDigit = true;
                                continue;
                            }
                            break;
                        }
                        if (!anyDigit)
                        {
                            FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                            throw new FormatException("Format item missing index");
                        }
                        while (index <utf8.Length && utf8[index] != NumericUnchecked.ToByte (125))
                        {
                            index += 1;
                        }
                        if (index >= utf8.Length || utf8[index] != NumericUnchecked.ToByte (125))
                        {
                            FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                            throw new FormatException("Unterminated format item");
                        }
                        index += 1;
                        let argsLength = NumericUnchecked.ToUSize(args.Length);
                        if (argIndex <0 || NumericUnchecked.ToUSize (argIndex) >= argsLength)
                        {
                            FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                            throw new FormatException("Format item index out of range");
                        }
                        let rendered = StringifyArg(args[argIndex]);
                        AppendUtf8(ref buffer, rendered);
                        continue;
                    }
                    if (current == NumericUnchecked.ToByte (125))
                    {
                        if (index + 1 <utf8.Length && utf8[index + 1] == NumericUnchecked.ToByte (125))
                        {
                            FVec.Push <byte >(ref buffer, NumericUnchecked.ToByte(125));
                            index += 2;
                            continue;
                        }
                        FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                        throw new FormatException("Single '}' encountered in format string");
                    }
                    FVec.Push <byte >(ref buffer, current);
                    index += 1;
                }
                let span = Foundation.Collections.Vec.AsReadOnlySpan <byte >(in buffer);
                let text = Utf8String.FromSpan(span);
                FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                return text;
            }
            private static void AppendUtf8(ref VecPtr buffer, string value) {
                if (value == null)
                {
                    return;
                }
                let span = value.AsUtf8Span();
                var idx = 0usize;
                while (idx <span.Length)
                {
                    FVec.Push <byte >(ref buffer, span[idx]);
                    idx += 1;
                }
            }
            private static string StringifyArg(object value) {
                if (value == null)
                {
                    return StringRuntime.Create();
                }
                return value.ToString();
            }
        }
        internal sealed class ConsoleStreamWriter : TextWriter
        {
            private bool _error;
            public init(bool isError) {
                _error = isError;
            }
            public override bool IsTerminal {
                get {
                    return _error ?Stderr.IsTerminal() : Stdout.IsTerminal();
                }
            }
            public override bool IsRedirected {
                get {
                    return !IsTerminal;
                }
            }
            public override void Write(string ?value) {
                var text = value;
                if (text == null)
                {
                    text = "";
                }
                if (_error)
                {
                    Stderr.Write(text);
                }
                else
                {
                    Stdout.Write(text);
                }
            }
            public override void Flush() {
                if (_error)
                {
                    Stderr.Flush();
                }
                else
                {
                    Stdout.Flush();
                }
            }
        }
        internal sealed class ConsoleInReader : TextReader
        {
            public override bool IsTerminal {
                get {
                    return Stdin.IsTerminal();
                }
            }
            public override int Read() {
                var slot = Span <byte >.StackAlloc(1);
                var readCount = 0usize;
                let status = Platform.ReadInto(Platform.FdStdin, slot, out readCount);
                if (status == IoError.Eof || readCount == 0)
                {
                    return - 1;
                }
                if (status != IoError.Success)
                {
                    return - 1;
                }
                let ch = NumericUnchecked.ToInt32(slot[0]);
                return ch;
            }
            public override string ?ReadLine() {
                var text = "";
                var status = IoError.Unknown;
                if (Stdin.TryReadLine (out text, out status)) {
                    return text;
                }
                if (status == IoError.Eof)
                {
                    return null;
                }
                return null;
            }
        }
