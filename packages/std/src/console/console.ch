namespace Std;
import Std.Numeric;
import Std.Runtime;
import Std.Strings;
import Std.Span;
import Std.Core;
import Foundation.Collections;
import FVec = Foundation.Collections.Vec;
import FVecIntrinsics = Foundation.Collections.VecIntrinsics;
public enum ConsoleColor
{
    Black = 0, DarkBlue = 1, DarkGreen = 2, DarkCyan = 3, DarkRed = 4, DarkMagenta = 5, DarkYellow = 6, Gray = 7, DarkGray = 8, Blue = 9, Green = 10, Cyan = 11, Red = 12, Magenta = 13, Yellow = 14, White = 15,
}
public struct ConsoleKeyInfo
{
    public char KeyChar;
    public bool Alt;
    public bool Shift;
    public bool Control;
    public init(char keyChar, bool shift, bool alt, bool control) {
        KeyChar = keyChar;
        Shift = shift;
        Alt = alt;
        Control = control;
    }
}
public class TextReader
{
    public virtual bool IsTerminal {
        get {
            return false;
        }
    }
    public virtual int Read() {
        return - 1;
    }
    public virtual string ?ReadLine() {
        return null;
    }
}
public class TextWriter
{
    public virtual string NewLine {
        get {
            return Console.NewLine;
        }
    }
    public virtual bool IsTerminal {
        get {
            return false;
        }
    }
    public virtual bool IsRedirected {
        get {
            return true;
        }
    }
    public virtual void Write(string ?value) {
    }
    public virtual void Write(str ?value) {
        if (value == null)
        {
            return;
        }
        let text = StringRuntime.FromStr(value);
        Write(text);
    }
    public virtual void Write(char value) {
        Write(value.ToString());
    }
    public virtual void WriteLine() {
        Write(NewLine);
    }
    public virtual void WriteLine(string ?value) {
        Write(value);
        Write(NewLine);
    }
    public virtual void WriteLine(str ?value) {
        Write(value);
        Write(NewLine);
    }
    public virtual void Flush() {
    }
}
public sealed class StringWriter : TextWriter
{
    private VecPtr _buffer;
    public init() {
        // Explicitly call the Vec constructor to avoid name-resolution ambiguity now that HashMap exists.
        _buffer = Foundation.Collections.Vec.WithCapacity <byte >(64);
    }
    public override bool IsRedirected {
        get {
            return true;
        }
    }
    public override bool IsTerminal {
        get {
            return false;
        }
    }
    public override void Write(string ?value) {
        if (value == null)
        {
            return;
        }
        var owner = this;
        let utf8 = value.AsUtf8Span();
        var idx = 0usize;
        while (idx <utf8.Length)
        {
            FVec.Push <byte >(ref owner._buffer, utf8[idx]);
            idx += 1;
        }
    }
    public override void WriteLine() {
        Write(Console.NewLine);
    }
    public override void WriteLine(string ?value) {
        Write(value);
        Write(Console.NewLine);
    }
    public string ToString() {
        let span = Foundation.Collections.Vec.AsReadOnlySpan <byte >(in _buffer);
        return Utf8String.FromSpan(span);
    }
    public void dispose(ref this) {
        FVecIntrinsics.chic_rt_vec_drop(ref _buffer);
    }
}
public sealed class StringReader : TextReader
{
    private ReadOnlySpan <byte >_utf8;
    private usize _index;
    public init(string value) {
        if (value == null)
        {
            value = StringRuntime.Create();
        }
        _utf8 = value.AsUtf8Span();
        _index = 0usize;
    }
    public override int Read() {
        if (_index >= _utf8.Length)
        {
            return - 1;
        }
        let current = _utf8[_index];
        _index += 1;
        return NumericUnchecked.ToInt32(current);
    }
    public override string ?ReadLine() {
        if (_index >= _utf8.Length)
        {
            return null;
        }
        let start = _index;
        while (_index <_utf8.Length)
        {
            let current = _utf8[_index];
            if (current == NumericUnchecked.ToByte (10) || current == NumericUnchecked.ToByte (13))
            {
                break;
            }
            _index += 1;
        }
        let length = _index - start;
        let line = Utf8String.FromSpan(_utf8.Slice(start, length));
        if (_index <_utf8.Length)
        {
            if (_utf8[_index] == NumericUnchecked.ToByte (13) && _index + 1 <_utf8.Length && _utf8[_index + 1] == NumericUnchecked.ToByte (10))
            {
                _index += 2;
            }
            else
            {
                _index += 1;
            }
        }
        return line;
    }
}
public static class Console
{
    public static TextReader In {
        get {
            return __ConsoleCore.InReader();
        }
    }
    public static TextWriter Out {
        get {
            return __ConsoleCore.OutWriter();
        }
    }
    public static TextWriter Error {
        get {
            return __ConsoleCore.ErrorWriter();
        }
    }
    public static void SetIn(TextReader reader) {
        __ConsoleCore.SetIn(reader);
    }
    public static void SetOut(TextWriter writer) {
        __ConsoleCore.SetOut(writer);
    }
    public static void SetError(TextWriter writer) {
        __ConsoleCore.SetError(writer);
    }
    public static string NewLine {
        get {
            return __ConsoleCore.GetNewLine();
        }
        set {
            __ConsoleCore.SetNewLine(value);
        }
    }
    public static bool AutoFlush {
        get {
            return __ConsoleCore.GetAutoFlush();
        }
        set {
            __ConsoleCore.SetAutoFlush(value);
        }
    }
    public static void Write(string ?value) {
        __ConsoleCore.Write(value, ConsoleStream.StandardOut);
    }
    public static void Write(str ?value) {
        if (value == null)
        {
            __ConsoleCore.Write(StringRuntime.Create(), ConsoleStream.StandardOut);
            return;
        }
        __ConsoleCore.Write(StringRuntime.FromStr(value), ConsoleStream.StandardOut);
    }
    public static void Write(char value) {
        __ConsoleCore.Write(value.ToString(), ConsoleStream.StandardOut);
    }
    public static void Write(bool value) {
        __ConsoleCore.Write(value.ToString(), ConsoleStream.StandardOut);
    }
    public static void Write(int value) {
        __ConsoleCore.Write(value.ToString(), ConsoleStream.StandardOut);
    }
    public static void Write(uint value) {
        __ConsoleCore.Write(value.ToString(), ConsoleStream.StandardOut);
    }
    public static void Write(long value) {
        __ConsoleCore.Write(value.ToString(), ConsoleStream.StandardOut);
    }
    public static void Write(ulong value) {
        __ConsoleCore.Write(value.ToString(), ConsoleStream.StandardOut);
    }
    public static void Write(float value) {
        __ConsoleCore.Write(value.ToString(), ConsoleStream.StandardOut);
    }
    public static void Write(double value) {
        __ConsoleCore.Write(value.ToString(), ConsoleStream.StandardOut);
    }
    public static void Write(decimal value) {
        __ConsoleCore.Write(value.ToString(), ConsoleStream.StandardOut);
    }
    public static void Write(object ?value) {
        if (value == null)
        {
            __ConsoleCore.Write(StringRuntime.Create(), ConsoleStream.StandardOut);
            return;
        }
        __ConsoleCore.Write(value.ToString(), ConsoleStream.StandardOut);
    }
    public static void Write(string format, object[] args) {
        __ConsoleCore.Write(__ConsoleCore.Format(format, args), ConsoleStream.StandardOut);
    }
    public static void WriteLine() {
        __ConsoleCore.WriteLine(ConsoleStream.StandardOut);
    }
    public static void WriteLine(string ?value) {
        __ConsoleCore.WriteLine(value, ConsoleStream.StandardOut);
    }
    public static void WriteLine(str ?value) {
        if (value == null)
        {
            __ConsoleCore.WriteLine(StringRuntime.Create(), ConsoleStream.StandardOut);
            return;
        }
        __ConsoleCore.WriteLine(StringRuntime.FromStr(value), ConsoleStream.StandardOut);
    }
    public static void WriteLine(object ?value) {
        if (value == null)
        {
            __ConsoleCore.WriteLine(StringRuntime.Create(), ConsoleStream.StandardOut);
            return;
        }
        __ConsoleCore.WriteLine(value.ToString(), ConsoleStream.StandardOut);
    }
    public static void WriteLine(string format, object[] args) {
        __ConsoleCore.WriteLine(__ConsoleCore.Format(format, args), ConsoleStream.StandardOut);
    }
    public static int Read() {
        return __ConsoleCore.Read();
    }
    public static string ?ReadLine() {
        return __ConsoleCore.ReadLine();
    }
    public static bool KeyAvailable {
        get {
            return __ConsoleCore.KeyAvailable();
        }
    }
    public static ConsoleKeyInfo ReadKey() {
        return __ConsoleCore.ReadKey(false);
    }
    public static ConsoleKeyInfo ReadKey(bool intercept) {
        return __ConsoleCore.ReadKey(intercept);
    }
    public static bool IsInputRedirected {
        get {
            return __ConsoleCore.IsInputRedirected();
        }
    }
    public static bool IsOutputRedirected {
        get {
            return __ConsoleCore.IsOutputRedirected();
        }
    }
    public static bool IsErrorRedirected {
        get {
            return __ConsoleCore.IsErrorRedirected();
        }
    }
    public static ConsoleColor ForegroundColor {
        get {
            return __ConsoleCore.GetForegroundColor();
        }
        set {
            __ConsoleCore.SetForegroundColor(value);
        }
    }
    public static ConsoleColor BackgroundColor {
        get {
            return __ConsoleCore.GetBackgroundColor();
        }
        set {
            __ConsoleCore.SetBackgroundColor(value);
        }
    }
    public static void ResetColor() {
        __ConsoleCore.ResetColor();
    }
    public static void Clear() {
        __ConsoleCore.Clear();
    }
    public static int CursorLeft {
        get {
            return __ConsoleCore.GetCursorLeft();
        }
        set {
            __ConsoleCore.SetCursorPosition(value, __ConsoleCore.GetCursorTop());
        }
    }
    public static int CursorTop {
        get {
            return __ConsoleCore.GetCursorTop();
        }
        set {
            __ConsoleCore.SetCursorPosition(__ConsoleCore.GetCursorLeft(), value);
        }
    }
    public static void SetCursorPosition(int left, int top) {
        __ConsoleCore.SetCursorPosition(left, top);
    }
    public static bool CursorVisible {
        get {
            return __ConsoleCore.GetCursorVisible();
        }
        set {
            __ConsoleCore.SetCursorVisible(value);
        }
    }
    public static int BufferWidth {
        get {
            return __ConsoleCore.GetBufferWidth();
        }
        set {
            __ConsoleCore.SetBufferWidth(value);
        }
    }
    public static int BufferHeight {
        get {
            return __ConsoleCore.GetBufferHeight();
        }
        set {
            __ConsoleCore.SetBufferHeight(value);
        }
    }
    public static int WindowWidth {
        get {
            return __ConsoleCore.GetWindowWidth();
        }
        set {
            __ConsoleCore.SetWindowWidth(value);
        }
    }
    public static int WindowHeight {
        get {
            return __ConsoleCore.GetWindowHeight();
        }
        set {
            __ConsoleCore.SetWindowHeight(value);
        }
    }
    public static int LargestWindowWidth {
        get {
            return __ConsoleCore.GetLargestWindowWidth();
        }
    }
    public static int LargestWindowHeight {
        get {
            return __ConsoleCore.GetLargestWindowHeight();
        }
    }
}
