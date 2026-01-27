namespace Std.Runtime.Native;
@repr(c) public struct RegionHandle
{
    public ulong Pointer;
    public ulong Profile;
    public ulong Generation;
}
