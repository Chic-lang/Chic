import Std.Collections;
import Std.Span;

namespace ReadOnlySpanExec
{
    public int Main()
    {
        var data = Vec.New<int>();
        var mut = Span<int>.FromVec(ref data);
        mut.Fill(0, 4, in 42);
        var readonlyView = mut.ToReadOnly();
        if (readonlyView.Len != 4)
        {
            return 91;
        }
        return 0;
    }
}
