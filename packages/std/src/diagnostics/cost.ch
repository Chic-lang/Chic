namespace Std.Diagnostics;
import Std.Testing;
public struct BudgetResult
{
    public bool Exceeded;
    public double DeltaUs;
}
/// <summary>Utility helpers for enforcing declared cost budgets.</summary>
public static class Cost
{
    public static BudgetResult Enforce(double measuredUs) {
        var result = new BudgetResult();
        result.Exceeded = false;
        result.DeltaUs = 0.0;
        return result;
    }
    public static BudgetResult Enforce(double measuredUs, ulong budgetUs) {
        var result = new BudgetResult();
        let delta = measuredUs - (double) budgetUs;
        result.Exceeded = delta >0.0;
        result.DeltaUs = delta;
        return result;
    }
}
testcase Given_cost_enforcement_handles_budget_When_executed_Then_cost_enforcement_handles_budget()
{
    let result = Cost.Enforce(15.0, 10ul);
    Assert.That(result.Exceeded).IsTrue();
    Assert.That(result.DeltaUs).IsEqualTo(5.0);
}
testcase Given_cost_enforcement_allows_missing_budget_When_executed_Then_cost_enforcement_allows_missing_budget()
{
    let result = Cost.Enforce(5.0);
    Assert.That(result.Exceeded).IsFalse();
    Assert.That(result.DeltaUs).IsEqualTo(0.0);
}
