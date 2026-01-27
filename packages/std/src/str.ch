namespace Std;
import Std.Runtime.InteropServices;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "str", kind = "str", aliases = ["str", "Str", "Std.Str",
"System.Str"], c_type = "struct chic_str") public readonly struct Str : Clone, Copy
{
    public Self Clone() => this;
}
