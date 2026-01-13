namespace Std.Runtime.Native;
@repr(c) public struct RegionHandle
{
    public * mut @expose_address byte Pointer;
}
