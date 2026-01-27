namespace Std;
import Std.Testing;
static class TypeTestHelpers
{
    public static Type IntType() {
        return Type.Of <int >();
    }
    public static Type LongType() {
        return Type.Of <long >();
    }
}
testcase Given_type_of_returns_id_When_executed_Then_type_of_returns_id()
{
    let typeId = Type.Of <int >().Id;
    Assert.That(typeId).IsNotEqualTo(0ul);
}
testcase Given_type_equality_operator_true_for_same_type_When_executed_Then_type_equality_operator_true_for_same_type()
{
    let left = TypeTestHelpers.IntType();
    let right = TypeTestHelpers.IntType();
    Assert.That(left == right).IsTrue();
}
testcase Given_type_inequality_operator_false_for_same_type_When_executed_Then_type_inequality_operator_false_for_same_type()
{
    let left = TypeTestHelpers.IntType();
    let right = TypeTestHelpers.IntType();
    Assert.That(left != right).IsFalse();
}
testcase Given_type_equality_operator_false_for_other_type_When_executed_Then_type_equality_operator_false_for_other_type()
{
    let left = TypeTestHelpers.IntType();
    let other = TypeTestHelpers.LongType();
    Assert.That(left == other).IsFalse();
}
testcase Given_type_inequality_operator_true_for_other_type_When_executed_Then_type_inequality_operator_true_for_other_type()
{
    let left = TypeTestHelpers.IntType();
    let other = TypeTestHelpers.LongType();
    Assert.That(left != other).IsTrue();
}
testcase Given_type_equals_true_for_same_type_When_executed_Then_type_equals_true_for_same_type()
{
    let left = TypeTestHelpers.IntType();
    let right = TypeTestHelpers.IntType();
    Assert.That(left.Equals(right)).IsTrue();
}
testcase Given_type_equals_false_for_other_type_When_executed_Then_type_equals_false_for_other_type()
{
    let left = TypeTestHelpers.IntType();
    let other = TypeTestHelpers.LongType();
    Assert.That(left.Equals(other)).IsFalse();
}
testcase Given_type_get_hashcode_is_stable_When_executed_Then_hashcode_repeats()
{
    let ty = TypeTestHelpers.IntType();
    let h1 = ty.GetHashCode();
    let h2 = ty.GetHashCode();
    Assert.That(h1 == h2).IsTrue();
}
