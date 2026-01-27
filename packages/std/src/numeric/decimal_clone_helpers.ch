namespace Std.Numeric.Decimal;
import Std.Memory;
internal static class CloneHelpers
{
    public static bool TryClone <T >(in T value, out T result) {
        Std.Memory.GlobalAllocator.InitializeDefault(out result);
        let decimalId = __type_id_of <decimal >();
        let typeId = __type_id_of <T >();
        if (typeId != decimalId)
        {
            return false;
        }
        var slot = Std.Memory.MaybeUninit <T >.Uninit();
        slot.Write(value);
        result = slot.AssumeInit();
        return true;
    }
}
