namespace Std.Data;
import Std.Core;
import Std.Testing;
/// <summary>Represents a mutable collection of <see cref="DbParameter"/> instances.</summary>
public abstract class DbParameterCollection
{
    /// <summary>Gets the number of parameters contained in the collection.</summary>
    public abstract int Count {
        get;
    }
    /// <summary>Adds a parameter to the collection.</summary>
    public virtual void Add(DbParameter parameter) {
        throw new DbException("DbParameterCollection.Add not implemented");
    }
    /// <summary>Removes a parameter from the collection if present.</summary>
    public virtual bool Remove(DbParameter parameter) {
        throw new DbException("DbParameterCollection.Remove not implemented");
        return false;
    }
    /// <summary>Removes all parameters from the collection.</summary>
    public virtual void Clear() {
        throw new DbException("DbParameterCollection.Clear not implemented");
    }
    /// <summary>Gets the parameter at the given ordinal.</summary>
    public virtual DbParameter GetByIndex(int index) {
        throw new DbException("DbParameterCollection.GetByIndex not implemented");
        return CoreIntrinsics.DefaultValue <DbParameter >();
    }
    /// <summary>Gets the parameter with the given name.</summary>
    public virtual DbParameter GetByName(string name) {
        throw new DbException("DbParameterCollection.GetByName not implemented");
        return CoreIntrinsics.DefaultValue <DbParameter >();
    }
}
private sealed class DbParameterCollectionTestAdapter : DbParameterCollection
{
    public override int Count => 0;
}
testcase Given_db_parameter_collection_add_throws_When_executed_Then_db_parameter_collection_add_throws()
{
    var collection = new DbParameterCollectionTestAdapter();
    Assert.Throws <DbException >(() => {
        collection.Add(CoreIntrinsics.DefaultValue <DbParameter >());
    }
    );
}
testcase Given_db_parameter_collection_remove_throws_When_executed_Then_db_parameter_collection_remove_throws()
{
    var collection = new DbParameterCollectionTestAdapter();
    Assert.Throws <DbException >(() => {
        let _ = collection.Remove(CoreIntrinsics.DefaultValue <DbParameter >());
    }
    );
}
testcase Given_db_parameter_collection_clear_throws_When_executed_Then_db_parameter_collection_clear_throws()
{
    var collection = new DbParameterCollectionTestAdapter();
    Assert.Throws <DbException >(() => {
        collection.Clear();
    }
    );
}
testcase Given_db_parameter_collection_get_by_index_throws_When_executed_Then_db_parameter_collection_get_by_index_throws()
{
    var collection = new DbParameterCollectionTestAdapter();
    Assert.Throws <DbException >(() => {
        let _ = collection.GetByIndex(0);
    }
    );
}
testcase Given_db_parameter_collection_get_by_name_throws_When_executed_Then_db_parameter_collection_get_by_name_throws()
{
    var collection = new DbParameterCollectionTestAdapter();
    Assert.Throws <DbException >(() => {
        let _ = collection.GetByName("param");
    }
    );
}
