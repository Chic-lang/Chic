namespace Std.Diagnostics;
import Std;
import Std.Core;
import Std.Globalization;
import Std.Sync;
import Std.Runtime;
import Std.Strings;
import Std.Span;
import Std.Platform.IO;
import Std.Collections;
import Foundation.Collections;
import FVec = Foundation.Collections.Vec;
import FVecIntrinsics = Foundation.Collections.VecIntrinsics;
import VecIntrinsics = Std.Collections.VecIntrinsics;
import VecError = Std.Collections.VecError;
public enum TraceLevel
{
    Off = 0, Error = 1, Warning = 2, Info = 3, Verbose = 4,
}
public class AssertFailedException : Exception
{
    public string ?DetailMessage;
    public init() : super("Assertion failed") {
    }
    public init(str message) : super(StringRuntime.FromStr(message)) {
    }
    public init(string message) : super(message) {
    }
    public init(string message, string ?detail) : super(message) {
        DetailMessage = detail;
    }
}
internal struct SwitchOverride
{
    public string Name;
    public string Value;
}
internal static class __SwitchSettings
{
    private static bool _initialized;
    private static Lock _lock;
    private static VecPtr _overrides;
    private static void EnsureInit() {
        if (_initialized)
        {
            return;
        }
        _lock = new Lock();
        _overrides = Std.Collections.VecIntrinsics.Create <SwitchOverride >();
        _initialized = true;
    }
    public static void Set(string name, string value) {
        EnsureInit();
        var guard = _lock.Enter();
        var overrides = _overrides;
        var span = FVec.AsSpan <SwitchOverride >(ref overrides);
        var idx = 0usize;
        while (idx <span.Length)
        {
            if (span[idx].Name == name)
            {
                span[idx].Value = value;
                guard.Release();
                return;
            }
            idx += 1usize;
        }
        let status = VecUtil.Push <SwitchOverride >(ref overrides, new SwitchOverride {
            Name = name, Value = value
        }
        );
        if (status != VecError.Success)
        {
            // Best-effort; ignore failures to avoid destabilising diagnostics.
        }
        _overrides = overrides;
        guard.Release();
    }
    public static bool TryGet(string name, out string value) {
        EnsureInit();
        var guard = _lock.Enter();
        var overrides = _overrides;
        var span = FVec.AsReadOnlySpan <SwitchOverride >(in overrides);
        var idx = 0usize;
        while (idx <span.Length)
        {
            if (span[idx].Name == name)
            {
                value = span[idx].Value;
                guard.Release();
                return true;
            }
            idx += 1usize;
        }
        guard.Release();
        value = StringRuntime.Create();
        return false;
    }
}
public static class Switches
{
    public static void SetOverride(string name, string value) {
        __SwitchSettings.Set(name, value);
    }
}
public class BooleanSwitch
{
    public string Name;
    public string Description;
    private bool _enabled;
    public init(string name, string description, bool defaultValue = false) {
        Name = name;
        Description = description;
        _enabled = ResolveInitial(name, defaultValue);
    }
    public bool Enabled {
        get {
            return _enabled;
        }
        set {
            _enabled = value;
        }
    }
    private static bool ResolveInitial(string name, bool fallback) {
        if (__SwitchSettings.TryGet (name, out var overrideValue)) {
            return ParseBool(overrideValue, fallback);
        }
        let env = Std.Environment.GetEnvironmentVariable(name);
        if (env != null && env != "")
        {
            return ParseBool(env, fallback);
        }
        return fallback;
    }
    private static bool ParseBool(string value, bool fallback) {
        let text = value.ToString();
        if (text == "1" || NumericCultureInfo.EqualsIgnoreAsciiCase (text, "true") || NumericCultureInfo.EqualsIgnoreAsciiCase (text,
        "yes") || NumericCultureInfo.EqualsIgnoreAsciiCase (text, "on"))
        {
            return true;
        }
        if (text == "0" || NumericCultureInfo.EqualsIgnoreAsciiCase (text, "false") || NumericCultureInfo.EqualsIgnoreAsciiCase (text,
        "no") || NumericCultureInfo.EqualsIgnoreAsciiCase (text, "off"))
        {
            return false;
        }
        return fallback;
    }
}
public class TraceSwitch
{
    public string Name;
    public string Description;
    private TraceLevel _level;
    public init(string name, string description, TraceLevel defaultLevel = TraceLevel.Off) {
        Name = name;
        Description = description;
        _level = ResolveInitial(name, defaultLevel);
    }
    public TraceLevel Level {
        get {
            return _level;
        }
        set {
            _level = value;
        }
    }
    public bool TraceError => (int) _level >= (int) TraceLevel.Error;
    public bool TraceWarning => (int) _level >= (int) TraceLevel.Warning;
    public bool TraceInfo => (int) _level >= (int) TraceLevel.Info;
    public bool TraceVerbose => (int) _level >= (int) TraceLevel.Verbose;
    private static TraceLevel ResolveInitial(string name, TraceLevel fallback) {
        if (__SwitchSettings.TryGet (name, out var overrideValue)) {
            return ParseLevel(overrideValue, fallback);
        }
        let env = Std.Environment.GetEnvironmentVariable(name);
        if (env != null && env != "")
        {
            return ParseLevel(env, fallback);
        }
        return fallback;
    }
    private static TraceLevel ParseLevel(string value, TraceLevel fallback) {
        let text = value.ToString();
        if (NumericCultureInfo.EqualsIgnoreAsciiCase (text, "off"))
        {
            return TraceLevel.Off;
        }
        if (NumericCultureInfo.EqualsIgnoreAsciiCase (text, "error"))
        {
            return TraceLevel.Error;
        }
        if (NumericCultureInfo.EqualsIgnoreAsciiCase (text, "warn") || NumericCultureInfo.EqualsIgnoreAsciiCase (text, "warning"))
        {
            return TraceLevel.Warning;
        }
        if (NumericCultureInfo.EqualsIgnoreAsciiCase (text, "info") || NumericCultureInfo.EqualsIgnoreAsciiCase (text, "information"))
        {
            return TraceLevel.Info;
        }
        if (NumericCultureInfo.EqualsIgnoreAsciiCase (text, "verbose") || NumericCultureInfo.EqualsIgnoreAsciiCase (text,
        "trace"))
        {
            return TraceLevel.Verbose;
        }
        return fallback;
    }
}
public class TraceListener
{
    public virtual void Write(string ?message) {
        // no-op in base implementation
    }
    public virtual void Write(str ?message) {
        if (message == null)
        {
            Write(null);
            return;
        }
        let text = StringRuntime.FromStr(message);
        Write(text);
    }
    public virtual void WriteLine() {
        Write(Environment.NewLine());
    }
    public virtual void WriteLine(string ?message) {
        Write(message);
        Write(Environment.NewLine());
    }
    public virtual void WriteLine(str ?message) {
        if (message == null)
        {
            Write(Environment.NewLine());
            return;
        }
        let text = StringRuntime.FromStr(message);
        WriteLine(text);
    }
    public virtual void Flush() {
    }
    public virtual void Close() {
    }
    public virtual void Fail(string ?message, string ?detail) {
        if (message != null)
        {
            Write(message);
        }
        if (detail != null && detail != "")
        {
            if (message != null && message != "")
            {
                Write(": ");
            }
            Write(detail);
        }
        WriteLine();
    }
}
public sealed class TraceListenerCollection
{
    private Lock _lock;
    private VecPtr _items;
    public init() {
        _lock = new Lock();
        _items = Std.Collections.VecIntrinsics.Create <TraceListener >();
    }
    public int Count {
        get {
            var guard = _lock.Enter();
            let len = FVec.Len(in _items);
            guard.Release();
            return(int) len;
        }
    }
    public TraceListener this[int index] {
        get {
            var guard = _lock.Enter();
            let span = FVec.AsReadOnlySpan <TraceListener >(in _items);
            if (index <0 || (usize) index >= span.Length)
            {
                guard.Release();
                throw new ArgumentOutOfRangeException("index");
            }
            let listener = span[(usize) index];
            guard.Release();
            return listener;
        }
    }
    public void Add(TraceListener listener) {
        var owner = this;
        if (listener == null)
        {
            throw new ArgumentNullException("listener");
        }
        var guard = owner._lock.Enter();
        let status = VecUtil.Push <TraceListener >(ref owner._items, listener);
        if (status != VecError.Success)
        {
            // Ignore failure to avoid breaking diagnostics in low-memory cases.
        }
        guard.Release();
    }
    public bool Remove(TraceListener listener) {
        var owner = this;
        if (listener == null)
        {
            return false;
        }
        var guard = owner._lock.Enter();
        let span = FVec.AsReadOnlySpan <TraceListener >(in owner._items);
        var idx = 0usize;
        var removed = false;
        while (idx <span.Length)
        {
            if (span[idx] == listener)
            {
                var slot = Std.Memory.MaybeUninit <TraceListener >.Uninit();
                let handle = slot.AsValueMutPtr();
                let status = FVecIntrinsics.chic_rt_vec_remove(ref owner._items, idx, in handle);
                if (status == VecError.Success)
                {
                    slot.dispose();
                    removed = true;
                }
                break;
            }
            idx += 1usize;
        }
        guard.Release();
        return removed;
    }
    public void Clear() {
        var owner = this;
        var guard = owner._lock.Enter();
        FVecIntrinsics.chic_rt_vec_clear(ref owner._items);
        guard.Release();
    }
    internal VecPtr Snapshot() {
        var guard = _lock.Enter();
        var clone = CoreIntrinsics.DefaultValue <VecPtr >();
        FVecIntrinsics.chic_rt_vec_clone(ref clone, in _items);
        guard.Release();
        return clone;
    }
    public void dispose(ref this) {
        var guard = _lock.Enter();
        FVecIntrinsics.chic_rt_vec_drop(ref _items);
        guard.Release();
    }
}
public sealed class DefaultTraceListener : TraceListener
{
    private bool _useError;
    private bool _closed;
    public init(bool useError = true) {
        _useError = useError;
    }
    public override void Write(string ?message) {
        if (_closed)
        {
            return;
        }
        if (_useError)
        {
            Console.Error.Write(message);
        }
        else
        {
            Console.Write(message);
        }
    }
    public override void WriteLine(string ?message) {
        if (_closed)
        {
            return;
        }
        if (_useError)
        {
            Console.Error.WriteLine(message);
        }
        else
        {
            Console.WriteLine(message);
        }
    }
    public override void Flush() {
        if (_closed)
        {
            return;
        }
        if (_useError)
        {
            Console.Error.Flush();
        }
        else
        {
            Console.Out.Flush();
        }
    }
    public override void Close() {
        _closed = true;
    }
}
public sealed class ConsoleTraceListener : TraceListener
{
    private bool _error;
    public init() : self(false) {
    }
    public init(bool useErrorStream) {
        _error = useErrorStream;
    }
    public override void Write(string ?message) {
        if (_error)
        {
            Console.Error.Write(message);
        }
        else
        {
            Console.Write(message);
        }
    }
    public override void WriteLine(string ?message) {
        if (_error)
        {
            Console.Error.WriteLine(message);
        }
        else
        {
            Console.WriteLine(message);
        }
    }
    public override void Flush() {
        if (_error)
        {
            Console.Error.Flush();
        }
        else
        {
            Console.Out.Flush();
        }
    }
}
public sealed class FileTraceListener : TraceListener
{
    private File _file;
    private bool _closed;
    public init(string path) : self(path, true) {
    }
    public init(string path, bool append) {
        _file = File.OpenWrite(path, append, out var status);
        if (status != IoError.Success)
        {
            _closed = true;
        }
    }
    public override void Write(string ?message) {
        if (_closed || !_file.IsValid)
        {
            return;
        }
        if (message == null)
        {
            return;
        }
        let utf8 = message.AsUtf8Span();
        _file.Write(utf8);
    }
    public override void WriteLine(string ?message) {
        Write(message);
        Write(Environment.NewLine());
    }
    public override void Flush() {
        if (_closed || !_file.IsValid)
        {
            return;
        }
        _file.Flush();
    }
    public override void Close() {
        if (_closed)
        {
            return;
        }
        _closed = true;
        var closeStatus = IoError.Success;
        _file.Close(out closeStatus);
    }
    public void dispose(ref this) {
        Close();
    }
}
internal static class __TraceCore
{
    private static bool _initialized;
    private static Lock _lock;
    private static TraceListenerCollection _listeners;
    private static bool _autoFlush;
    private static int _indentLevel;
    private static int _indentSize;
    private static string _indentString;
    private static bool _atLineStart;
    private static void EnsureInit() {
        if (_initialized)
        {
            return;
        }
        _lock = new Lock();
        _listeners = new TraceListenerCollection();
        _listeners.Add(new DefaultTraceListener(true));
        _autoFlush = false;
        _indentLevel = 0;
        _indentSize = 4;
        _indentString = StringRuntime.Create();
        _atLineStart = true;
        _initialized = true;
    }
    internal static TraceListenerCollection Listeners {
        get {
            EnsureInit();
            return _listeners;
        }
    }
    internal static bool AutoFlush {
        get {
            EnsureInit();
            var guard = _lock.Enter();
            let value = _autoFlush;
            guard.Release();
            return value;
        }
        set {
            EnsureInit();
            var guard = _lock.Enter();
            _autoFlush = value;
            guard.Release();
        }
    }
    internal static int IndentLevel {
        get {
            EnsureInit();
            var guard = _lock.Enter();
            let value = _indentLevel;
            guard.Release();
            return value;
        }
        set {
            EnsureInit();
            var guard = _lock.Enter();
            _indentLevel = value <0 ?0 : value;
            RefreshIndent();
            guard.Release();
        }
    }
    internal static int IndentSize {
        get {
            EnsureInit();
            var guard = _lock.Enter();
            let value = _indentSize;
            guard.Release();
            return value;
        }
        set {
            EnsureInit();
            var guard = _lock.Enter();
            _indentSize = value <0 ?0 : value;
            RefreshIndent();
            guard.Release();
        }
    }
    internal static void Indent() {
        EnsureInit();
        var guard = _lock.Enter();
        _indentLevel = _indentLevel + 1;
        RefreshIndent();
        guard.Release();
    }
    internal static void Unindent() {
        EnsureInit();
        var guard = _lock.Enter();
        if (_indentLevel >0)
        {
            _indentLevel = _indentLevel - 1;
        }
        RefreshIndent();
        guard.Release();
    }
    internal static void Flush() {
        EnsureInit();
        var snapshot = _listeners.Snapshot();
        var span = FVec.AsReadOnlySpan <TraceListener >(in snapshot);
        var idx = 0usize;
        while (idx <span.Length)
        {
            span[idx].Flush();
            idx += 1usize;
        }
        FVecIntrinsics.chic_rt_vec_drop(ref snapshot);
    }
    internal static void Close() {
        EnsureInit();
        var snapshot = _listeners.Snapshot();
        var span = FVec.AsReadOnlySpan <TraceListener >(in snapshot);
        var idx = 0usize;
        while (idx <span.Length)
        {
            span[idx].Close();
            idx += 1usize;
        }
        FVecIntrinsics.chic_rt_vec_drop(ref snapshot);
    }
    internal static void Write(str ?message, str ?category, bool newline) {
        if (message == null)
        {
            if (category == null)
            {
                Write(null, null, newline);
                return;
            }
            let cat = StringRuntime.FromStr(category);
            Write(null, cat, newline);
            return;
        }
        let msg = StringRuntime.FromStr(message);
        if (category == null)
        {
            Write(msg, null, newline);
            return;
        }
        let cat = StringRuntime.FromStr(category);
        Write(msg, cat, newline);
    }
    internal static void Write(string ?message, string ?category, bool newline) {
        EnsureInit();
        var guard = _lock.Enter();
        let formatted = FormatCategory(message, category);
        ApplyIndent(formatted, newline, out var rendered, out var finalLineStart);
        _atLineStart = finalLineStart;
        let autoflush = _autoFlush;
        var snapshot = _listeners.Snapshot();
        guard.Release();
        var span = FVec.AsReadOnlySpan <TraceListener >(in snapshot);
        var idx = 0usize;
        while (idx <span.Length)
        {
            if (newline)
            {
                span[idx].WriteLine(rendered);
            }
            else
            {
                span[idx].Write(rendered);
            }
            if (autoflush)
            {
                span[idx].Flush();
            }
            idx += 1usize;
        }
        FVecIntrinsics.chic_rt_vec_drop(ref snapshot);
    }
    internal static string Format(string format, object[] args) {
        return __ConsoleCore.Format(format, args);
    }
    internal static void ReportFail(string ?message, string ?detail) {
        var baseMessage = "Assertion failed";
        var msgText = StringRuntime.Create();
        if (message == null)
        {
            msgText = StringRuntime.Create();
        }
        else
        {
            msgText = message.ToString();
        }
        if (msgText != "")
        {
            baseMessage = baseMessage + ": " + msgText;
        }
        var detailText = StringRuntime.Create();
        if (detail == null)
        {
            detailText = StringRuntime.Create();
        }
        else
        {
            detailText = detail.ToString();
        }
        if (detailText != "")
        {
            baseMessage = baseMessage + " (" + detailText + ")";
        }
        var stack = StackPlaceholder();
        if (stack != "")
        {
            baseMessage = baseMessage + Environment.NewLine() + stack;
        }
        Write(baseMessage, null, true);
        let ex = new AssertFailedException(baseMessage, detail);
        throw ex;
    }
    private static string StackPlaceholder() {
        return "stack trace unavailable on this target/build";
    }
    private static void RefreshIndent() {
        var count = _indentLevel * _indentSize;
        if (count <= 0)
        {
            _indentString = StringRuntime.Create();
            return;
        }
        var buffer = FVec.WithCapacity <char >((usize) count);
        var idx = 0usize;
        while (idx < (usize) count)
        {
            FVec.Push <char >(ref buffer, ' ');
            idx += 1usize;
        }
        let span = FVec.AsReadOnlySpan <char >(in buffer);
        _indentString = span.ToString();
        FVecIntrinsics.chic_rt_vec_drop(ref buffer);
    }
    private static void ApplyIndent(string message, bool newline, out string rendered, out bool finalLineStart) {
        let text = message;
        var indent = _indentString;
        var builder = StringRuntime.Create();
        var atLineStart = _atLineStart;
        var idx = 0usize;
        while (idx <text.Length)
        {
            let ch = text[idx];
            if (atLineStart && ch != '\n' && ch != '\r')
            {
                builder = builder + indent;
                atLineStart = false;
            }
            builder = builder + ch.ToString();
            if (ch == '\n')
            {
                atLineStart = true;
            }
            idx += 1usize;
        }
        if (newline)
        {
            atLineStart = true;
        }
        rendered = builder;
        finalLineStart = atLineStart;
    }
    private static string FormatCategory(string ?message, string ?category) {
        var content = StringRuntime.Create();
        if (message == null)
        {
            content = StringRuntime.Create();
        }
        else
        {
            content = message.ToString();
        }
        if (category == null || category == "")
        {
            return content;
        }
        let cat = category.ToString();
        return "[" + cat + "] " + content;
    }
}
public static class Debug
{
    public static bool AutoFlush {
        get => __TraceCore.AutoFlush;
        set => __TraceCore.AutoFlush = value;
    }
    public static int IndentLevel {
        get => __TraceCore.IndentLevel;
        set => __TraceCore.IndentLevel = value;
    }
    public static int IndentSize {
        get => __TraceCore.IndentSize;
        set => __TraceCore.IndentSize = value;
    }
    public static TraceListenerCollection Listeners => __TraceCore.Listeners;
    @conditional("DEBUG") public static void Indent() {
        __TraceCore.Indent();
    }
    @conditional("DEBUG") public static void Unindent() {
        __TraceCore.Unindent();
    }
    @conditional("DEBUG") public static void Write(string ?message) {
        __TraceCore.Write(message, null, false);
    }
    @conditional("DEBUG") public static void Write(str ?message) {
        __TraceCore.Write(message, null, false);
    }
    @conditional("DEBUG") public static void Write(string ?message, string ?category) {
        __TraceCore.Write(message, category, false);
    }
    @conditional("DEBUG") public static void Write(object ?value) {
        if (value == null)
        {
            __TraceCore.Write(null, null, false);
            return;
        }
        __TraceCore.Write(value.ToString(), null, false);
    }
    @conditional("DEBUG") public static void Write(object ?value, string ?category) {
        if (value == null)
        {
            __TraceCore.Write(null, category, false);
            return;
        }
        __TraceCore.Write(value.ToString(), category, false);
    }
    @conditional("DEBUG") public static void WriteIf(bool condition, string ?message) {
        if (!condition)
        {
            return;
        }
        __TraceCore.Write(message, null, false);
    }
    @conditional("DEBUG") public static void WriteIf(bool condition, string ?message, string ?category) {
        if (!condition)
        {
            return;
        }
        __TraceCore.Write(message, category, false);
    }
    @conditional("DEBUG") public static void WriteIf(bool condition, object ?value) {
        if (!condition)
        {
            return;
        }
        if (value == null)
        {
            __TraceCore.Write(null, null, false);
            return;
        }
        __TraceCore.Write(value.ToString(), null, false);
    }
    @conditional("DEBUG") public static void WriteIf(bool condition, object ?value, string ?category) {
        if (!condition)
        {
            return;
        }
        if (value == null)
        {
            __TraceCore.Write(null, category, false);
            return;
        }
        __TraceCore.Write(value.ToString(), category, false);
    }
    @conditional("DEBUG") public static void WriteLine(string ?message) {
        __TraceCore.Write(message, null, true);
    }
    @conditional("DEBUG") public static void WriteLine(str ?message) {
        __TraceCore.Write(message, null, true);
    }
    @conditional("DEBUG") public static void WriteLine(string ?message, string ?category) {
        __TraceCore.Write(message, category, true);
    }
    @conditional("DEBUG") public static void WriteLine(object ?value) {
        if (value == null)
        {
            __TraceCore.Write(null, null, true);
            return;
        }
        __TraceCore.Write(value.ToString(), null, true);
    }
    @conditional("DEBUG") public static void WriteLine(object ?value, string ?category) {
        if (value == null)
        {
            __TraceCore.Write(null, category, true);
            return;
        }
        __TraceCore.Write(value.ToString(), category, true);
    }
    @conditional("DEBUG") public static void WriteLineIf(bool condition, string ?message) {
        if (!condition)
        {
            return;
        }
        __TraceCore.Write(message, null, true);
    }
    @conditional("DEBUG") public static void WriteLineIf(bool condition, string ?message, string ?category) {
        if (!condition)
        {
            return;
        }
        __TraceCore.Write(message, category, true);
    }
    @conditional("DEBUG") public static void WriteLineIf(bool condition, object ?value) {
        if (!condition)
        {
            return;
        }
        if (value == null)
        {
            __TraceCore.Write(null, null, true);
            return;
        }
        __TraceCore.Write(value.ToString(), null, true);
    }
    @conditional("DEBUG") public static void WriteLineIf(bool condition, object ?value, string ?category) {
        if (!condition)
        {
            return;
        }
        if (value == null)
        {
            __TraceCore.Write(null, category, true);
            return;
        }
        __TraceCore.Write(value.ToString(), category, true);
    }
    @conditional("DEBUG") public static void Print(string ?message) {
        __TraceCore.Write(message, null, true);
    }
    @conditional("DEBUG") public static void Print(str ?message) {
        __TraceCore.Write(message, null, true);
    }
    @conditional("DEBUG") public static void PrintFormat(string format, object[] args) {
        __TraceCore.Write(__TraceCore.Format(format, args), null, true);
    }
    @conditional("DEBUG") public static void WriteLineFormat(string format, object[] args) {
        __TraceCore.Write(__TraceCore.Format(format, args), null, true);
    }
    @conditional("DEBUG") public static void Flush() {
        __TraceCore.Flush();
    }
    @conditional("DEBUG") public static void Close() {
        __TraceCore.Close();
    }
    @conditional("DEBUG") public static void Fail(string ?message) {
        __TraceCore.ReportFail(message, null);
    }
    @conditional("DEBUG") public static void Fail(string ?message, string ?detailMessage) {
        __TraceCore.ReportFail(message, detailMessage);
    }
    @conditional("DEBUG") public static void Assert(bool condition) {
        if (!condition)
        {
            __TraceCore.ReportFail(null, null);
        }
    }
    @conditional("DEBUG") public static void Assert(bool condition, string ?message) {
        if (!condition)
        {
            __TraceCore.ReportFail(message, null);
        }
    }
    @conditional("DEBUG") public static void Assert(bool condition, string ?message, string ?detailMessage) {
        if (!condition)
        {
            __TraceCore.ReportFail(message, detailMessage);
        }
    }
}
public static class Trace
{
    public static bool AutoFlush {
        get => __TraceCore.AutoFlush;
        set => __TraceCore.AutoFlush = value;
    }
    public static int IndentLevel {
        get => __TraceCore.IndentLevel;
        set => __TraceCore.IndentLevel = value;
    }
    public static int IndentSize {
        get => __TraceCore.IndentSize;
        set => __TraceCore.IndentSize = value;
    }
    public static TraceListenerCollection Listeners => __TraceCore.Listeners;
    @conditional("TRACE") public static void Indent() {
        __TraceCore.Indent();
    }
    @conditional("TRACE") public static void Unindent() {
        __TraceCore.Unindent();
    }
    @conditional("TRACE") public static void Write(string ?message) {
        __TraceCore.Write(message, null, false);
    }
    @conditional("TRACE") public static void Write(str ?message) {
        __TraceCore.Write(message, null, false);
    }
    @conditional("TRACE") public static void Write(string ?message, string ?category) {
        __TraceCore.Write(message, category, false);
    }
    @conditional("TRACE") public static void Write(object ?value) {
        if (value == null)
        {
            __TraceCore.Write(null, null, false);
            return;
        }
        __TraceCore.Write(value.ToString(), null, false);
    }
    @conditional("TRACE") public static void Write(object ?value, string ?category) {
        if (value == null)
        {
            __TraceCore.Write(null, category, false);
            return;
        }
        __TraceCore.Write(value.ToString(), category, false);
    }
    @conditional("TRACE") public static void WriteIf(bool condition, string ?message) {
        if (!condition)
        {
            return;
        }
        __TraceCore.Write(message, null, false);
    }
    @conditional("TRACE") public static void WriteIf(bool condition, string ?message, string ?category) {
        if (!condition)
        {
            return;
        }
        __TraceCore.Write(message, category, false);
    }
    @conditional("TRACE") public static void WriteIf(bool condition, object ?value) {
        if (!condition)
        {
            return;
        }
        if (value == null)
        {
            __TraceCore.Write(null, null, false);
            return;
        }
        __TraceCore.Write(value.ToString(), null, false);
    }
    @conditional("TRACE") public static void WriteIf(bool condition, object ?value, string ?category) {
        if (!condition)
        {
            return;
        }
        if (value == null)
        {
            __TraceCore.Write(null, category, false);
            return;
        }
        __TraceCore.Write(value.ToString(), category, false);
    }
    @conditional("TRACE") public static void WriteLine(string ?message) {
        __TraceCore.Write(message, null, true);
    }
    @conditional("TRACE") public static void WriteLine(str ?message) {
        __TraceCore.Write(message, null, true);
    }
    @conditional("TRACE") public static void WriteLine(string ?message, string ?category) {
        __TraceCore.Write(message, category, true);
    }
    @conditional("TRACE") public static void WriteLine(object ?value) {
        if (value == null)
        {
            __TraceCore.Write(null, null, true);
            return;
        }
        __TraceCore.Write(value.ToString(), null, true);
    }
    @conditional("TRACE") public static void WriteLine(object ?value, string ?category) {
        if (value == null)
        {
            __TraceCore.Write(null, category, true);
            return;
        }
        __TraceCore.Write(value.ToString(), category, true);
    }
    @conditional("TRACE") public static void WriteLineIf(bool condition, string ?message) {
        if (!condition)
        {
            return;
        }
        __TraceCore.Write(message, null, true);
    }
    @conditional("TRACE") public static void WriteLineIf(bool condition, string ?message, string ?category) {
        if (!condition)
        {
            return;
        }
        __TraceCore.Write(message, category, true);
    }
    @conditional("TRACE") public static void WriteLineIf(bool condition, object ?value) {
        if (!condition)
        {
            return;
        }
        if (value == null)
        {
            __TraceCore.Write(null, null, true);
            return;
        }
        __TraceCore.Write(value.ToString(), null, true);
    }
    @conditional("TRACE") public static void WriteLineIf(bool condition, object ?value, string ?category) {
        if (!condition)
        {
            return;
        }
        if (value == null)
        {
            __TraceCore.Write(null, category, true);
            return;
        }
        __TraceCore.Write(value.ToString(), category, true);
    }
    @conditional("TRACE") public static void Print(string ?message) {
        __TraceCore.Write(message, null, true);
    }
    @conditional("TRACE") public static void Print(str ?message) {
        __TraceCore.Write(message, null, true);
    }
    @conditional("TRACE") public static void PrintFormat(string format, object[] args) {
        __TraceCore.Write(__TraceCore.Format(format, args), null, true);
    }
    @conditional("TRACE") public static void WriteLineFormat(string format, object[] args) {
        __TraceCore.Write(__TraceCore.Format(format, args), null, true);
    }
    @conditional("TRACE") public static void Flush() {
        __TraceCore.Flush();
    }
    @conditional("TRACE") public static void Close() {
        __TraceCore.Close();
    }
    @conditional("TRACE") public static void Fail(string ?message) {
        __TraceCore.ReportFail(message, null);
    }
    @conditional("TRACE") public static void Fail(string ?message, string ?detailMessage) {
        __TraceCore.ReportFail(message, detailMessage);
    }
}
