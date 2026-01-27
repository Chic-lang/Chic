namespace Std.IO;
import Std.Async;
import Std.Platform.IO;
import Std.Span;
import Std.Numeric;
/// <summary>Specifies how the operating system should open a file.</summary>
public enum FileMode
{
    /// <summary>Create a new file; throw if it already exists.</summary>
    CreateNew = 1,
    /// <summary>Create a new file or overwrite an existing file.</summary>
    Create = 2,
    /// <summary>Open an existing file; throw if it does not exist.</summary>
    Open = 3,
    /// <summary>Open a file if it exists; otherwise create a new one.</summary>
    OpenOrCreate = 4,
    /// <summary>Open an existing file and truncate it.</summary>
    Truncate = 5,
    /// <summary>Open or create a file and seek to the end for appending.</summary>
    Append = 6,
}
/// <summary>Specifies file read and write access.</summary>
public enum FileAccess
{
    /// <summary>Read-only access.</summary>
    Read = 1,
    /// <summary>Write-only access.</summary>
    Write = 2,
    /// <summary>Read and write access.</summary>
    ReadWrite = 3,
}
/// <summary>Specifies how the file can be shared with other handles.</summary>
public enum FileShare
{
    /// <summary>Prevents other processes from opening the file.</summary>
    None = 0,
    /// <summary>Permits subsequent openings for reading.</summary>
    Read = 1,
    /// <summary>Permits subsequent openings for writing.</summary>
    Write = 2,
    /// <summary>Permits subsequent openings for reading or writing.</summary>
    ReadWrite = 3,
}
/// <summary>File-backed stream over libc FILE* APIs.</summary>
public sealed class FileStream : Stream
{
    private File _file;
    private bool _canRead;
    private bool _canWrite;
    private bool _ownsHandle;
    /// <summary>Initializes a new file stream for the given path.</summary>
    /// <param name="path">Path to the file.</param>
    /// <param name="mode">File mode to apply.</param>
    /// <param name="access">Requested access.</param>
    /// <param name="share">Sharing semantics.</param>
    /// <exception cref="Std.IOException">Thrown when the file cannot be opened.</exception>
    public init(string path, FileMode mode, FileAccess access, FileShare share = FileShare.None) {
        var err = IoError.Unknown;
        let openMode = ResolveMode(mode, access);
        _file = File.Open(path, openMode, out err);
        if (err != IoError.Success)
        {
            throw new Std.IOException("Failed to open file");
        }
        _canRead = access == FileAccess.Read || access == FileAccess.ReadWrite;
        _canWrite = access == FileAccess.Write || access == FileAccess.ReadWrite || mode == FileMode.Append;
        _ownsHandle = true;
        if (mode == FileMode.Append)
        {
            _file.Seek(0, 2, out err);
        }
    }
    /// <summary>Initializes a new file stream from an existing handle.</summary>
    /// <param name="handle">Existing file handle.</param>
    /// <param name="access">Requested access.</param>
    /// <param name="ownsHandle">Whether the stream should close the handle when disposed.</param>
    /// <exception cref="Std.ArgumentException">Thrown when the handle is invalid.</exception>
    public init(File handle, FileAccess access, bool ownsHandle = true) {
        if (!handle.IsValid)
        {
            throw new Std.ArgumentException("Invalid file handle");
        }
        _file = handle;
        _canRead = access == FileAccess.Read || access == FileAccess.ReadWrite;
        _canWrite = access == FileAccess.Write || access == FileAccess.ReadWrite;
        _ownsHandle = ownsHandle;
    }
    /// <inheritdoc />
    public override bool CanRead => _canRead;
    /// <inheritdoc />
    public override bool CanWrite => _canWrite;
    /// <inheritdoc />
    public override bool CanSeek => true;
    /// <inheritdoc />
    public override long Length {
        get {
            this.ThrowIfDisposed();
            if (!_file.Tell (out var pos, out var status)) {
                throw new Std.IOException("Unable to read position");
            }
            // Seek to end to compute length, then restore.
            _file.Seek(0, 2, out status);
            if (!_file.Tell (out var endPos, out status)) {
                throw new Std.IOException("Unable to compute length");
            }
            let _restore = _file.Seek(pos, 0, out status);
            return NumericUnchecked.ToInt64(endPos);
        }
    }
    public override long Position {
        get {
            this.ThrowIfDisposed();
            if (_file.Tell (out var pos, out var status)) {
                return NumericUnchecked.ToInt64(pos);
            }
            throw new Std.IOException("Unable to read position");
        }
        set {
            this.ThrowIfDisposed();
            if (value <0)
            {
                throw new Std.ArgumentOutOfRangeException("Position");
            }
            if (!_file.Seek (NumericUnchecked.ToISize (value), 0, out var status)) {
                throw new Std.IOException("Unable to seek");
            }
        }
    }
    /// <inheritdoc />
    public override int Read(Span <byte >buffer) {
        this.ThrowIfDisposed();
        if (!_canRead)
        {
            throw new Std.NotSupportedException("FileStream not readable");
        }
        if (buffer.Length == 0)
        {
            return 0;
        }
        let ok = _file.Read(buffer, out var read, out var err);
        if (!ok && err != IoError.Eof)
        {
            throw new Std.IOException("File read failed");
        }
        return NumericUnchecked.ToInt32(read);
    }
    /// <inheritdoc />
    public override void Write(ReadOnlySpan <byte >buffer) {
        this.ThrowIfDisposed();
        if (!_canWrite)
        {
            throw new Std.NotSupportedException("FileStream not writable");
        }
        let status = _file.Write(buffer);
        if (status != IoError.Success)
        {
            throw new Std.IOException("File write failed");
        }
    }
    /// <inheritdoc />
    public override void Flush() {
        this.ThrowIfDisposed();
        _file.Flush();
    }
    /// <inheritdoc />
    public override long Seek(long offset, SeekOrigin origin) {
        this.ThrowIfDisposed();
        var originCode = origin == SeekOrigin.Begin ?0 : origin == SeekOrigin.Current ?1 : 2;
        if (!_file.Seek (NumericUnchecked.ToISize (offset), originCode, out var status)) {
            throw new Std.IOException("Seek failed");
        }
        if (!_file.Tell (out var pos, out status)) {
            throw new Std.IOException("Tell failed");
        }
        return NumericUnchecked.ToInt64(pos);
    }
    /// <inheritdoc />
    public override void SetLength(long value) {
        // Not currently supported over FILE*, raise.
        throw new Std.NotSupportedException("SetLength not supported on FileStream");
    }
    /// <inheritdoc />
    public override Task FlushAsync(CancellationToken ct) {
        this.ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Flush canceled");
        }
        _file.Flush();
        return TaskRuntime.CompletedTask();
    }
    /// <inheritdoc />
    protected override void Dispose(bool disposing) {
        if (IsDisposed)
        {
            return;
        }
        if (disposing && _ownsHandle && _file.IsValid)
        {
            _file.Close(out var closeStatus);
        }
        base.Dispose(disposing);
    }
    private static string ResolveMode(FileMode mode, FileAccess access) {
        switch (mode)
        {
            case FileMode.Create:
            case FileMode.CreateNew:
            case FileMode.Truncate:
                return access == FileAccess.Read ?"w+b" : "w+b";
            case FileMode.Open:
                if (access == FileAccess.Read)
                {
                    return "rb";
                }
                if (access == FileAccess.Write)
                {
                    return "r+b";
                }
                return "r+b";
            case FileMode.OpenOrCreate:
                return access == FileAccess.Read ?"a+b" : "a+b";
            case FileMode.Append:
                return "ab";
            default :
                return "rb";
            }
        }
        }
