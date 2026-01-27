namespace Std.Runtime;
import Std.Runtime.Collections;
public static class CloneRuntime
{
    @extern("C") private static extern void chic_rt_clone_invoke(isize glue, ValueConstPtr src, ValueMutPtr dest);
    public static void Invoke(isize glue, ValueConstPtr src, ValueMutPtr dest) {
        chic_rt_clone_invoke(glue, src, dest);
    }
}
