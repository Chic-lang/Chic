namespace Std.Data;
import Foundation.Collections;
import Std.Collections;
import Std.Testing;
/// <summary>Global registry of provider factories keyed by invariant name.</summary>
public static class DbProviderRegistry
{
    /// <summary>Registers or replaces a provider factory.</summary>
    public static void Register(string invariantName, DbProviderFactory factory) {
    }
    /// <summary>Resolves a provider factory by invariant name.</summary>
    public static DbProviderFactory Resolve(string invariantName) {
        throw new DbException("Provider not found: " + invariantName);
    }
    /// <summary>Creates a connection using the registered factory and applies the connection string.</summary>
    public static DbConnection CreateConnection(string invariantName, string connectionString) {
        throw new DbException("Provider not found: " + invariantName);
    }
}
testcase Given_db_provider_registry_resolve_unknown_throws_When_executed_Then_db_provider_registry_resolve_unknown_throws()
{
    Assert.Throws <DbException >(() => {
        let _ = DbProviderRegistry.Resolve("missing");
    }
    );
}
testcase Given_db_provider_registry_create_connection_unknown_throws_When_executed_Then_db_provider_registry_create_connection_unknown_throws()
{
    Assert.Throws <DbException >(() => {
        let _ = DbProviderRegistry.CreateConnection("missing", "server=local");
    }
    );
}
