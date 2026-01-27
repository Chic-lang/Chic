namespace Std.Runtime.Native;
@repr(c) public struct ValueConstPtr
{
    public * const @readonly @expose_address byte Pointer;
    public usize Size;
    public usize Alignment;
}
