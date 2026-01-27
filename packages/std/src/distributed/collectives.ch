namespace Std.Distributed;
import Std.Testing;
public enum CollectiveKind
{
    Allreduce, Broadcast, Pshard,
}
public struct CollectiveRecord
{
    public ulong Seq;
    public CollectiveKind Kind;
    public uint Participants;
    public ulong Bytes;
    public ulong LatencyNs;
}
public class CollectiveLog
{
    private CollectiveRecord[] _records;
    private string[] _diagnostics;
    private usize _count;
    private usize _diagCount;
    public init() {
        _records = new CollectiveRecord[4];
        _diagnostics = new string[4];
        _count = 0usize;
        _diagCount = 0usize;
    }
    public void Allreduce(uint participants, ulong bytes) {
        EnsureCapacity();
        let seq = _count + 1usize;
        _records[_count].Seq = NumericUnchecked.ToUInt64(seq);
        _records[_count].Kind = CollectiveKind.Allreduce;
        _records[_count].Participants = participants;
        _records[_count].Bytes = bytes;
        _records[_count].LatencyNs = DeterministicLatency(participants, bytes);
        _count = _count + 1usize;
        PushDiagnostic("collective stub: allreduce participants=" + participants.ToString() + " bytes=" + bytes.ToString() + " seq=" + seq.ToString());
    }
    public CollectiveRecord[] Records() {
        var copy = new CollectiveRecord[_count];
        var idx = 0usize;
        while (idx <_count)
        {
            copy[idx] = _records[idx];
            idx = idx + 1usize;
        }
        return copy;
    }
    public string[] Diagnostics() {
        var copy = new string[_diagCount];
        var idx = 0usize;
        while (idx <_diagCount)
        {
            copy[idx] = _diagnostics[idx];
            idx = idx + 1usize;
        }
        return copy;
    }
    private ulong DeterministicLatency(uint participants, ulong bytes) {
        return NumericUnchecked.ToUInt64(participants) * bytes;
    }
    private void EnsureCapacity() {
        if (_records == null)
        {
            _records = new CollectiveRecord[4];
            _count = 0usize;
            return;
        }
        if (_count <NumericUnchecked.ToUSize (_records.Length))
        {
            return;
        }
        let newLen = _records.Length == 0 ?4usize : NumericUnchecked.ToUSize(_records.Length * 2);
        var grown = new CollectiveRecord[newLen];
        var idx = 0usize;
        while (idx <_records.Length)
        {
            grown[idx] = _records[idx];
            idx = idx + 1usize;
        }
        _records = grown;
    }
    private void PushDiagnostic(string diag) {
        if (_diagnostics == null)
        {
            _diagnostics = new string[4];
            _diagCount = 0usize;
        }
        if (_diagCount >= NumericUnchecked.ToUSize (_diagnostics.Length))
        {
            let newLen = _diagnostics.Length == 0 ?4usize : NumericUnchecked.ToUSize(_diagnostics.Length * 2);
            var grown = new string[newLen];
            var idx = 0usize;
            while (idx <_diagnostics.Length)
            {
                grown[idx] = _diagnostics[idx];
                idx = idx + 1usize;
            }
            _diagnostics = grown;
        }
        _diagnostics[_diagCount] = diag;
        _diagCount = _diagCount + 1usize;
    }
}
testcase Given_collectives_log_records_length_When_executed_Then_collectives_log_records_length()
{
    var log = new CollectiveLog();
    log.Allreduce(2u, 32ul);
    let records = log.Records();
    Assert.That(records.Length).IsEqualTo(1usize);
}
testcase Given_collectives_log_records_participants_When_executed_Then_collectives_log_records_participants()
{
    var log = new CollectiveLog();
    log.Allreduce(2u, 32ul);
    let records = log.Records();
    Assert.That(records[0usize].Participants).IsEqualTo(2u);
}
testcase Given_collectives_log_diagnostics_length_When_executed_Then_collectives_log_diagnostics_length()
{
    var log = new CollectiveLog();
    log.Allreduce(2u, 32ul);
    let diags = log.Diagnostics();
    Assert.That(diags.Length).IsEqualTo(1usize);
}
