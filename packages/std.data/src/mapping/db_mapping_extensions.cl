namespace Std.Data.Mapping;
import Std.Async;
import Std.Data;
/// <summary>Minimal Dapper-like extension stubs.</summary>
public static class DbMappingExtensions
{
    public static Task <AsyncEnumerable <T >> QueryAsync <T >(this DbConnection connection, string sql) {
        return TaskRuntime.FromResult(AsyncEnumerable <T >.Empty());
    }
    public static Task <int >ExecuteAsync(this DbConnection connection, string sql) {
        return TaskRuntime.FromResult(0);
    }
}
