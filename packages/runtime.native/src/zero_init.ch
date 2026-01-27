namespace Std.Runtime.Native;
public static class ZeroInit
{
    @extern("C") private unsafe static extern void memset(* mut @expose_address byte dest, byte value, usize len);
    @export("chic_rt_zero_init") public unsafe static void chic_rt_zero_init(* mut @expose_address byte dest, usize len) {
        if (dest == null || len == 0usize)
        {
            return;
        }
        memset(dest, 0u8, len);
    }
}
