namespace Std.Data.Mapping;
import Std.Core;
import Std.Data;
import Std.Testing;
/// <summary>Placeholder parameter writer.</summary>
public static class ParameterWriter
{
    public static void Bind(DbCommand command, object ?args) {
    }
}

private sealed class ParameterWriterTestCommand : DbCommand
{
    public override DbParameterCollection Parameters => CoreIntrinsics.DefaultValue<DbParameterCollection>();
}

testcase Given_parameter_writer_bind_keeps_command_text_When_executed_Then_parameter_writer_bind_keeps_command_text()
{
    var command = new ParameterWriterTestCommand();
    command.CommandText = "SELECT 1";
    ParameterWriter.Bind(command, null);
    Assert.That(command.CommandText).IsEqualTo("SELECT 1");
}
