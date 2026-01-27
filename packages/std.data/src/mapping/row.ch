namespace Std.Data.Mapping;
import Std.Core;
import Std.Testing;
/// <summary>Lightweight row placeholder.</summary>
public struct Row
{
    public T Get <T >(int index) {
        return CoreIntrinsics.DefaultValue <T >();
    }
}

testcase Given_row_get_default_value_When_executed_Then_row_get_default_value()
{
    let row = new Row();
    Assert.That(row.Get<int>(0)).IsEqualTo(0);
}
