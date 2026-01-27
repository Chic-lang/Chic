namespace Std.Diagnostics;
import Std.Core;
import Std.Numeric;
import Std.Random;
import Std.Testing;
public struct PerfMetric
{
    public ulong TraceId;
    public string MirId;
    public string Label;
    public double CpuUs;
    public ulong ?BudgetCpuUs;
    public ulong ?BudgetGpuUs;
    public ulong ?BudgetMemBytes;
}
public struct PerfRun
{
    public string Profile;
    public PerfMetric[] Metrics;
    public RunLog RunLog;
}
public struct PerfSnapshot
{
    public string Version;
    public string Target;
    public PerfRun[] Runs;
}
/// <summary>Collects trace metrics and run logs for profiling.</summary>
public class TraceCollector
{
    private string _profile;
    private string _target;
    private PerfMetric[] _metrics;
    private usize _count;
    public init(string profile, string target) {
        _profile = profile;
        _target = target;
        _metrics = new PerfMetric[4];
        _count = 0usize;
    }
    public void Record(ulong traceId, string label, double cpuUs) {
        EnsureCapacity();
        _metrics[_count].TraceId = traceId;
        _metrics[_count].MirId = label;
        _metrics[_count].Label = label;
        _metrics[_count].CpuUs = cpuUs;
        _metrics[_count].BudgetCpuUs = null;
        _metrics[_count].BudgetGpuUs = null;
        _metrics[_count].BudgetMemBytes = null;
        _count = _count + 1usize;
    }
    public PerfSnapshot Snapshot() {
        var metricsCopy = new PerfMetric[_count];
        var idx = 0usize;
        while (idx <_count)
        {
            metricsCopy[idx] = _metrics[idx];
            idx = idx + 1usize;
        }
        var run = CoreIntrinsics.DefaultValue <PerfRun >();
        run.Profile = _profile;
        run.Metrics = metricsCopy;
        run.RunLog = RunLogRecorder.Snapshot();
        var snapshot = CoreIntrinsics.DefaultValue <PerfSnapshot >();
        snapshot.Version = "0.1";
        snapshot.Target = _target;
        snapshot.Runs = new PerfRun[1];
        snapshot.Runs[0] = run;
        return snapshot;
    }
    private void EnsureCapacity() {
        if (_metrics == null)
        {
            _metrics = new PerfMetric[4];
            _count = 0usize;
            return;
        }
        if (_count <NumericUnchecked.ToUSize (_metrics.Length))
        {
            return;
        }
        let newLen = _metrics.Length == 0 ?4usize : NumericUnchecked.ToUSize(_metrics.Length * 2);
        var grown = new PerfMetric[newLen];
        var idx = 0usize;
        while (idx <_metrics.Length)
        {
            grown[idx] = _metrics[idx];
            idx = idx + 1usize;
        }
        _metrics = grown;
    }
}
testcase Given_trace_collector_snapshot_has_single_run_When_executed_Then_trace_collector_snapshot_has_single_run()
{
    var collector = new TraceCollector("debug", "host");
    collector.Record(1ul, "foo", 10.0);
    let snapshot = collector.Snapshot();
    Assert.That(snapshot.Runs.Length).IsEqualTo(1usize);
}
testcase Given_trace_collector_snapshot_has_single_metric_When_executed_Then_trace_collector_snapshot_has_single_metric()
{
    var collector = new TraceCollector("debug", "host");
    collector.Record(1ul, "foo", 10.0);
    let snapshot = collector.Snapshot();
    Assert.That(snapshot.Runs[0usize].Metrics.Length).IsEqualTo(1usize);
}
testcase Given_trace_collector_snapshot_has_runlog_version_When_executed_Then_trace_collector_snapshot_has_runlog_version()
{
    var collector = new TraceCollector("debug", "host");
    collector.Record(1ul, "foo", 10.0);
    let snapshot = collector.Snapshot();
    Assert.That(snapshot.Runs[0usize].RunLog.Version).IsEqualTo(RunLogRecorder.RunLogVersion);
}
