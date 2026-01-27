namespace Std.Sync;
import Std.Core;
import Std.Numeric;
import Std.Runtime;
public static class ArcRuntime
{
    public unsafe static ref T AsRef <T >(ref __StdSyncArcHandle handle) {
        let ptr = RuntimeIntrinsics.chic_rt_arc_get(& handle);
        if (__StdSyncPointerHelpers.IsNullConst (ptr))
        {
            throw new Std.InvalidOperationException(Std.Runtime.StringRuntime.FromStr("Arc handle is null"));
        }
        var * mut @expose_address T typed = (* mut @expose_address T) ptr;
        return ref * typed;
    }
}
