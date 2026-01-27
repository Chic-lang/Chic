namespace Exec.StdData;

import Std.Data;

public class FakeParameter : DbParameter
{
    private string _name;
    private DbType _dbType;
    private ParameterDirection _direction;
    private bool _isNullable;
    private int _size;
    private object? _value;

    public init()
    {
        _name = "";
        _dbType = DbType.String;
        _direction = ParameterDirection.Input;
        _isNullable = true;
        _size = 0;
        _value = null;
    }

    public override string ParameterName
    {
        get { return _name; }
        set { _name = value; }
    }

    public override DbType DbType
    {
        get { return _dbType; }
        set { _dbType = value; }
    }

    public override ParameterDirection Direction
    {
        get { return _direction; }
        set { _direction = value; }
    }

    public override bool IsNullable
    {
        get { return _isNullable; }
        set { _isNullable = value; }
    }

    public override int Size
    {
        get { return _size; }
        set { _size = value; }
    }

    public override object? Value
    {
        get { return _value; }
        set { _value = value; }
    }
}
