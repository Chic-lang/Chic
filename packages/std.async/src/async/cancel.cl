namespace Std.Async;
import Std.Core;
import Std.Testing;
/// <summary>Shared cancellation state.</summary>
internal class CancelState
{
    public bool Canceled;
    public bool HasDeadline;
    public ulong DeadlineNs;
    public ulong BudgetRemaining;
}
/// <summary>Budgeted cancellation token source with deterministic propagation.</summary>
public class CancelSource
{
    internal CancelState State;
    public init(ulong budgetUnits = 0ul, ulong deadlineNs = 0ul) {
        State = new CancelState();
        State.Canceled = false;
        State.HasDeadline = deadlineNs != 0ul;
        State.DeadlineNs = deadlineNs;
        State.BudgetRemaining = budgetUnits == 0ul ?ulong.MaxValue : budgetUnits;
    }
    public static CancelSource Create(ulong budgetUnits = 0ul, ulong deadlineNs = 0ul) {
        return new CancelSource(budgetUnits, deadlineNs);
    }
    public CancelToken Token() {
        return new CancelToken(State);
    }
    public void Cancel() {
        State.Canceled = true;
    }
    public void ForceDeadline() {
        State.Canceled = true;
    }
}
public class CancelToken
{
    internal CancelState State;
    internal init(CancelState state) {
        State = state;
    }
    public ulong EchoAmount(ulong amount) {
        return amount;
    }
    public bool IsCanceled {
        get {
            return State.Canceled;
        }
    }
    /// <summary>Consume a deterministic budget. Returns true if cancellation is now set.</summary>
    public bool ConsumeBudget(ulong amount) {
        if (State.Canceled)
        {
            return true;
        }
        if (amount == 0ul)
        {
            return State.Canceled;
        }
        if (State.BudgetRemaining <= amount)
        {
            State.BudgetRemaining = 0ul;
            State.Canceled = true;
            return true;
        }
        State.BudgetRemaining = State.BudgetRemaining - amount;
        return false;
    }
    /// <summary>Check deadline against a monotonic timestamp.</summary>
    public bool CheckDeadline(ulong nowNs) {
        if (State.Canceled)
        {
            return true;
        }
        if (State.HasDeadline && nowNs >= State.DeadlineNs)
        {
            State.Canceled = true;
            return true;
        }
        return State.Canceled;
    }
}
public static class Cancellation
{
    public static CancelSource NewCancelSource(ulong budgetUnits, ulong deadlineNs) {
        return CancelSource.Create(budgetUnits, deadlineNs);
    }
    public static CancelToken Token() {
        var source = new CancelSource();
        return source.Token();
    }
}
testcase Given_cancel_token_starts_not_canceled_When_executed_Then_cancel_token_starts_not_canceled()
{
    var source = new CancelSource(2ul, 0ul);
    var token = source.Token();
    Assert.That(token.IsCanceled).IsFalse();
}
testcase Given_cancel_token_consume_budget_first_false_When_executed_Then_cancel_token_consume_budget_first_false()
{
    var source = new CancelSource(2ul, 0ul);
    var token = source.Token();
    let first = token.ConsumeBudget(1ul);
    Assert.That(first).IsFalse();
}
testcase Given_cancel_token_consume_budget_second_true_When_executed_Then_cancel_token_consume_budget_second_true()
{
    var source = new CancelSource(2ul, 0ul);
    var token = source.Token();
    let _ = token.ConsumeBudget(1ul);
    let second = token.ConsumeBudget(2ul);
    Assert.That(second).IsTrue();
}
testcase Given_cancel_token_consume_budget_sets_canceled_When_executed_Then_cancel_token_consume_budget_sets_canceled()
{
    var source = new CancelSource(2ul, 0ul);
    var token = source.Token();
    let _ = token.ConsumeBudget(1ul);
    let _ = token.ConsumeBudget(2ul);
    Assert.That(token.IsCanceled).IsTrue();
}
testcase Given_cancel_token_deadline_before_is_false_When_executed_Then_cancel_token_deadline_before_is_false()
{
    var source = new CancelSource(0ul, 100ul);
    var token = source.Token();
    let before = token.CheckDeadline(50ul);
    Assert.That(before).IsFalse();
}
testcase Given_cancel_token_deadline_after_is_true_When_executed_Then_cancel_token_deadline_after_is_true()
{
    var source = new CancelSource(0ul, 100ul);
    var token = source.Token();
    let after = token.CheckDeadline(150ul);
    Assert.That(after).IsTrue();
}

testcase Given_cancel_source_sets_budget_remaining_When_executed_Then_budget_remaining_matches()
{
    var source = new CancelSource(2ul, 0ul);
    Assert.That(source.State.BudgetRemaining).IsEqualTo(2ul);
}

testcase Given_ulong_comparison_le_When_executed_Then_le_behaves()
{
    Assert.That(2ul <= 1ul).IsFalse();
    Assert.That(1ul <= 2ul).IsTrue();
}

testcase Given_ulong_comparison_ge_When_executed_Then_ge_behaves()
{
    Assert.That(50ul >= 100ul).IsFalse();
    Assert.That(150ul >= 100ul).IsTrue();
}

testcase Given_ulong_param_passing_When_executed_Then_ulong_params_roundtrip()
{
    var source = new CancelSource(2ul, 0ul);
    var token = source.Token();
    Assert.That(token.EchoAmount(1ul)).IsEqualTo(1ul);
    Assert.That(token.EchoAmount(150ul)).IsEqualTo(150ul);
}
