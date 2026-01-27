namespace Std.IO;
import Std;
import Std.Core;
import Std.Platform.IO;
import Std.Span;
import Std.Testing;
testcase Given_file_stream_invalid_handle_throws_When_executed_Then_file_stream_invalid_handle_throws()
{
    var handle = CoreIntrinsics.DefaultValue <File >();
    Assert.Throws <ArgumentException >(() => {
        let _ = new FileStream(handle, FileAccess.Read);
    }
    );
}
testcase Given_file_stream_roundtrip_or_open_failure_When_executed_Then_file_stream_roundtrip_or_open_failure()
{
    let path = ".chic_test_filestream.bin";
    var ok = false;
    try {
        var stream = new FileStream(path, FileMode.Create, FileAccess.ReadWrite);
        var data = new byte[3];
        data[0] = 11u8;
        data[1] = 22u8;
        data[2] = 33u8;
        stream.Write(ReadOnlySpan <byte >.FromArray(in data));
        stream.Flush();
        stream.Position = 0;
        var buffer = Span <byte >.StackAlloc(3usize);
        let read = stream.Read(buffer);
        let endPos = stream.Seek(0, SeekOrigin.End);
        let dataOk = stream.CanRead && stream.CanWrite && stream.Length == 3 && read == 3 && buffer[0usize] == 11u8 && buffer[2usize] == 33u8 && endPos == 3;
        var setLengthThrows = false;
        try {
            stream.SetLength(1);
        }
        catch(NotSupportedException _) {
            setLengthThrows = true;
        }
        stream.Dispose();
        ok = dataOk && setLengthThrows;
    }
    catch(IOException ex) {
        ok = ex.Message == "Failed to open file";
    }
    Assert.That(ok).IsTrue();
}
