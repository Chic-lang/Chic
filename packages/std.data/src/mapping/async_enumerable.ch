namespace Std.Data.Mapping;
import Std.Async;
public delegate T RowFactory <T >(Row row);
public delegate Task <T >RowFactoryAsync <T >(Row row);
/// <summary>Minimal async enumerable placeholder.</summary>
public struct AsyncEnumerable <T >
{
    public static AsyncEnumerable <T >Empty() {
        return new AsyncEnumerable <T >();
    }
}
