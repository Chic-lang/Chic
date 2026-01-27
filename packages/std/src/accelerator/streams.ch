namespace Std.Accelerator;
import Std.Core;
import Std.Numeric;
import Std.Testing;
/// <summary>Marker for host memory space.</summary>
public struct Host
{
}
/// <summary>Marker for pinned host memory space.</summary>
public struct PinnedHost
{
}
/// <summary>Marker for GPU memory space (device-specific).</summary>
public struct Gpu
{
    public uint Id;
}
/// <summary>Marker for unified memory space.</summary>
public struct Unified
{
}
/// <summary>Stream capability bound to a specific memory space.</summary>
public struct Stream <M >
{
    public uint Id;
    public uint Device;
}
/// <summary>Event token representing completion on a stream.</summary>
public struct Event <M >
{
    public uint Id;
    public uint Stream;
    public uint Device;
}
/// <summary>Deterministic stream handle used by the native accelerator runtime.</summary>
public struct StreamHandle
{
    public uint Id;
    public uint DeviceId;
}
/// <summary>Deterministic event handle tied to a particular stream/device.</summary>
public struct EventHandle
{
    public uint Id;
    public uint Stream;
    public uint DeviceId;
}
/// <summary>Records accelerator operations for profiling and diagnostics.</summary>
public class NativeStreamRecorder
{
    private string[] _ops;
    private usize _count;
    public init() {
        _ops = new string[4];
        _count = 0usize;
    }
    public EventHandle EnqueueKernel(StreamHandle stream, string kernel) {
        let eventHandle = NextEvent(stream);
        PushOp("enqueue_kernel stream=" + stream.Id.ToString() + " kernel=" + kernel + " event=" + eventHandle.Id.ToString());
        return eventHandle;
    }
    public EventHandle EnqueueCopy(StreamHandle stream, usize bytes) {
        let eventHandle = NextEvent(stream);
        PushOp("enqueue_copy stream=" + stream.Id.ToString() + " bytes=" + bytes.ToString() + " event=" + eventHandle.Id.ToString());
        return eventHandle;
    }
    public EventHandle RecordEvent(StreamHandle stream) {
        let eventHandle = NextEvent(stream);
        PushOp("record_event stream=" + stream.Id.ToString() + " event=" + eventHandle.Id.ToString());
        return eventHandle;
    }
    public void WaitEvent(StreamHandle stream, EventHandle eventHandle) {
        if (stream.Id != eventHandle.Stream)
        {
            throw new Std.InvalidOperationException("event does not belong to stream");
        }
        PushOp("wait_event stream=" + stream.Id.ToString() + " event=" + eventHandle.Id.ToString());
    }
    public string[] Ordered() {
        var copy = new string[_count];
        var idx = 0usize;
        while (idx <_count)
        {
            copy[idx] = _ops[idx];
            idx = idx + 1usize;
        }
        return copy;
    }
    private EventHandle NextEvent(StreamHandle stream) {
        let id = NumericUnchecked.ToUInt32(_count + 1usize);
        var handle = new EventHandle();
        handle.Id = id;
        handle.Stream = stream.Id;
        handle.DeviceId = stream.DeviceId;
        return handle;
    }
    private void PushOp(string op) {
        EnsureCapacity();
        _ops[_count] = op;
        _count = _count + 1usize;
    }
    private void EnsureCapacity() {
        if (_ops == null)
        {
            _ops = new string[4];
            _count = 0usize;
            return;
        }
        if (_count <NumericUnchecked.ToUSize (_ops.Length))
        {
            return;
        }
        let newLen = _ops.Length == 0 ?4usize : NumericUnchecked.ToUSize(_ops.Length * 2);
        var grown = new string[newLen];
        var idx = 0usize;
        while (idx <_ops.Length)
        {
            grown[idx] = _ops[idx];
            idx = idx + 1usize;
        }
        _ops = grown;
    }
}
/// <summary>Stream factory that assigns deterministic identifiers.</summary>
public class StreamFactory
{
    private uint _nextStream;
    private uint _deviceId;
    public init(uint deviceId) {
        _nextStream = 0u;
        _deviceId = deviceId;
    }
    public StreamHandle Create() {
        let id = _nextStream;
        _nextStream = _nextStream + 1u;
        var handle = new StreamHandle();
        handle.Id = id;
        handle.DeviceId = _deviceId;
        return handle;
    }
}
public static class Streams
{
    public static Stream <M >NewStream <M >(uint device) {
        var stream = new Stream <M >();
        stream.Id = 0;
        stream.Device = device;
        return stream;
    }
}
testcase Given_native_stream_recorder_orders_ops_length_When_executed_Then_native_stream_recorder_orders_ops_length()
{
    var recorder = new NativeStreamRecorder();
    var factory = new StreamFactory(0u);
    let stream = factory.Create();
    let eventHandle = recorder.RecordEvent(stream);
    recorder.WaitEvent(stream, eventHandle);
    let ordered = recorder.Ordered();
    Assert.That(ordered.Length).IsEqualTo(2usize);
}
testcase Given_native_stream_recorder_orders_ops_record_event_first_When_executed_Then_native_stream_recorder_orders_ops_record_event_first()
{
    var recorder = new NativeStreamRecorder();
    var factory = new StreamFactory(0u);
    let stream = factory.Create();
    let eventHandle = recorder.RecordEvent(stream);
    recorder.WaitEvent(stream, eventHandle);
    let ordered = recorder.Ordered();
    Assert.That(ordered[0usize].Contains("record_event")).IsTrue();
}
testcase Given_native_stream_recorder_orders_ops_wait_event_second_When_executed_Then_native_stream_recorder_orders_ops_wait_event_second()
{
    var recorder = new NativeStreamRecorder();
    var factory = new StreamFactory(0u);
    let stream = factory.Create();
    let eventHandle = recorder.RecordEvent(stream);
    recorder.WaitEvent(stream, eventHandle);
    let ordered = recorder.Ordered();
    Assert.That(ordered[1usize].Contains("wait_event")).IsTrue();
}
testcase Given_native_stream_recorder_flags_wrong_stream_When_executed_Then_native_stream_recorder_flags_wrong_stream()
{
    var recorder = new NativeStreamRecorder();
    var factory = new StreamFactory(0u);
    let streamA = factory.Create();
    let streamB = factory.Create();
    let eventHandle = recorder.RecordEvent(streamA);
    Assert.Throws <Std.InvalidOperationException >(() => {
        recorder.WaitEvent(streamB, eventHandle);
    }
    );
}
