namespace Std.Data;
import Std.Core;
import Std.Testing;
/// <summary>Creates provider-specific database objects.</summary>
public abstract class DbProviderFactory
{
    /// <summary>Creates a new connection instance.</summary>
    public virtual DbConnection CreateConnection() {
        throw new DbException("DbProviderFactory.CreateConnection not implemented");
        return CoreIntrinsics.DefaultValue <DbConnection >();
    }
    /// <summary>Creates a new command instance.</summary>
    public virtual DbCommand CreateCommand() {
        throw new DbException("DbProviderFactory.CreateCommand not implemented");
        return CoreIntrinsics.DefaultValue <DbCommand >();
    }
    /// <summary>Creates a new parameter instance.</summary>
    public virtual DbParameter CreateParameter() {
        throw new DbException("DbProviderFactory.CreateParameter not implemented");
        return CoreIntrinsics.DefaultValue <DbParameter >();
    }
}

private sealed class DbProviderFactoryTestAdapter : DbProviderFactory
{
}

testcase Given_db_provider_factory_create_connection_throws_When_executed_Then_db_provider_factory_create_connection_throws()
{
    var factory = new DbProviderFactoryTestAdapter();
    Assert.Throws<DbException>(() => {
        let _ = factory.CreateConnection();
    });
}

testcase Given_db_provider_factory_create_command_throws_When_executed_Then_db_provider_factory_create_command_throws()
{
    var factory = new DbProviderFactoryTestAdapter();
    Assert.Throws<DbException>(() => {
        let _ = factory.CreateCommand();
    });
}

testcase Given_db_provider_factory_create_parameter_throws_When_executed_Then_db_provider_factory_create_parameter_throws()
{
    var factory = new DbProviderFactoryTestAdapter();
    Assert.Throws<DbException>(() => {
        let _ = factory.CreateParameter();
    });
}
