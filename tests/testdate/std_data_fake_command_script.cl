namespace Exec.StdData;

public struct FakeCommandScript
{
    public string CommandText;
    public int RowsAffected;
    public object? ScalarResult;
    public FakeResultSet ResultSet;
    public bool HasResultSet;

    public static FakeCommandScript NonQuery(string text, int rows)
    {
        var script = new FakeCommandScript();
        script.CommandText = text;
        script.RowsAffected = rows;
        script.HasResultSet = false;
        return script;
    }

    public static FakeCommandScript Scalar(string text, object? value)
    {
        var script = new FakeCommandScript();
        script.CommandText = text;
        script.ScalarResult = value;
        script.HasResultSet = false;
        return script;
    }

    public static FakeCommandScript Reader(string text, FakeResultSet resultSet)
    {
        var script = new FakeCommandScript();
        script.CommandText = text;
        script.ResultSet = resultSet;
        script.HasResultSet = true;
        return script;
    }
}
