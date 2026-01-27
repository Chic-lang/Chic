namespace Std.Async;
import Std.Core;
import Std.Testing;
internal struct ScopeTask
{
    public string Name;
    public bool Completed;
    public bool Canceled;
}
/// <summary>Structured concurrency scope tracker. Tasks must complete or cancel before exit.</summary>
public class ScopeTracker
{
    private ScopeTask[] _tasks;
    private usize _count;
    public init() {
        _tasks = new ScopeTask[4];
        _count = 0usize;
    }
    public usize Count {
        get {
            return _count;
        }
    }
    public usize ElementSize {
        get {
            return __sizeof <ScopeTask >();
        }
    }
    public void RegisterTask(string name) {
        EnsureCapacity();
        _tasks[_count].Name = name;
        _tasks[_count].Completed = false;
        _tasks[_count].Canceled = false;
        _count = _count + 1usize;
    }
    public void MarkCompleted(string name) {
        var idx = 0usize;
        while (idx <_count)
        {
            if (_tasks[idx].Name == name)
            {
                _tasks[idx].Completed = true;
                return;
            }
            idx = idx + 1usize;
        }
    }
    public void CancelAll() {
        var idx = 0usize;
        while (idx <_count)
        {
            _tasks[idx].Canceled = true;
            idx = idx + 1usize;
        }
    }
    public bool TryFinalizeScope(out string message) {
        message = "";
        var incomplete = new string[NumericUnchecked.ToInt32(_count)];
        var missing = 0usize;
        var idx = 0usize;
        while (idx <_count)
        {
            if (! (_tasks[idx].Completed || _tasks[idx].Canceled))
            {
                incomplete[NumericUnchecked.ToInt32(missing)] = _tasks[idx].Name;
                missing = missing + 1usize;
            }
            idx = idx + 1usize;
        }
        if (missing == 0usize)
        {
            return true;
        }
        message = "structured scope exited with incomplete tasks: ";
        var i = 0usize;
        while (i <missing)
        {
            message = message + incomplete[NumericUnchecked.ToInt32(i)];
            if (i + 1usize <missing)
            {
                message = message + ", ";
            }
            i = i + 1usize;
        }
        return false;
    }
    private void EnsureCapacity() {
        if (_tasks == null)
        {
            _tasks = new ScopeTask[4];
            _count = 0usize;
            return;
        }
        if (_count <NumericUnchecked.ToUSize (_tasks.Length))
        {
            return;
        }
        let newLen = _tasks.Length == 0 ?4usize : NumericUnchecked.ToUSize(_tasks.Length * 2);
        var grown = new ScopeTask[newLen];
        var idx = 0usize;
        while (idx <_tasks.Length)
        {
            grown[idx] = _tasks[idx];
            idx = idx + 1usize;
        }
        _tasks = grown;
    }
}
testcase Given_scope_tracker_detects_incomplete_tasks_When_executed_Then_scope_tracker_detects_incomplete_tasks()
{
    Assert.That(__sizeof <ScopeTask >()).IsEqualTo(48usize);
    var tracker = new ScopeTracker();
    Assert.That(tracker.ElementSize).IsEqualTo(__sizeof <ScopeTask >());
    tracker.RegisterTask("a");
    tracker.RegisterTask("b");
    tracker.MarkCompleted("a");
    Assert.That(tracker.Count).IsEqualTo(2usize);
    var message = "";
    var ok = tracker.TryFinalizeScope(out message);
    Assert.That(ok).IsFalse();
}
testcase Given_scope_tracker_allows_completed_tasks_When_executed_Then_scope_tracker_allows_completed_tasks()
{
    var tracker = new ScopeTracker();
    tracker.RegisterTask("a");
    tracker.MarkCompleted("a");
    var message = "";
    var ok = tracker.TryFinalizeScope(out message);
    Assert.That(ok).IsTrue();
    Assert.That(message).IsEqualTo("");
}
