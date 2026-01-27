namespace Exec.StdData;

public struct FakeResultSet
{
    public string[] Columns;
    public object?[][] Rows;

    public init(string[] columns, object?[][] rows)
    {
        Columns = columns;
        Rows = rows;
    }
}
