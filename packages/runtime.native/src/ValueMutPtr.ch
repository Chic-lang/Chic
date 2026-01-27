namespace Std.Runtime.Native;
@repr(c) public struct ValueMutPtr
{
    public * mut @expose_address byte Pointer;
    public usize Size;
    public usize Alignment;
}
