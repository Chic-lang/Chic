namespace Exec.StdData;

import Std.Data;

public class FakeProviderFactory : DbProviderFactory
{
    private FakeCommandScript[] _scripts;
    private string _dataSource;
    private string _database;
    private string _serverVersion;

    public init(FakeCommandScript[] scripts, string dataSource, string database, string serverVersion)
    {
        _scripts = scripts;
        _dataSource = dataSource;
        _database = database;
        _serverVersion = serverVersion;
    }

    public override DbConnection CreateConnection()
    {
        return new FakeConnection(CloneScripts(), _dataSource, _database, _serverVersion);
    }

    public override DbCommand CreateCommand()
    {
        var connection = (FakeConnection)CreateConnection();
        return new FakeCommand(connection, CloneScripts());
    }

    public override DbParameter CreateParameter()
    {
        return new FakeParameter();
    }

    private FakeCommandScript[] CloneScripts()
    {
        var copy = new FakeCommandScript[_scripts.Length];
        var idx = 0;
        while (idx < _scripts.Length)
        {
            copy[idx] = _scripts[idx];
            idx += 1;
        }
        return copy;
    }
}
