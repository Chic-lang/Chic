namespace Std.Runtime.Native;
// Minimal OS-backed cryptographic random provider used by Std.Security.Cryptography.RandomNumberGenerator.
internal static class CryptoRandom
{
    private static bool _test_fail_open = false;
    private static bool _test_fail_read = false;
    private static usize _test_read_limit = 0usize;
    private static bool _test_use_fake_io = false;
    private static byte _test_fake_byte = 0u8;
    public static void TestForceOpenFailure(bool value) {
        _test_fail_open = value;
    }
    public static void TestForceReadFailure(bool value) {
        _test_fail_read = value;
    }
    public static void TestSetReadLimit(usize value) {
        _test_read_limit = value;
    }
    public static void TestUseFakeIo(bool value) {
        _test_use_fake_io = value;
    }
    public static void TestSetFakeByte(byte value) {
        _test_fake_byte = value;
    }
    public unsafe static bool TestCoverageSweep() {
        _test_fail_open = false;
        _test_fail_read = false;
        _test_read_limit = 0usize;
        _test_use_fake_io = false;
        var ok = true;
        var buffer = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 8usize, Alignment = 1usize
        }
        ;
        let alloc = NativeAlloc.AllocZeroed(8usize, 1usize, out buffer);
        ok = ok && alloc == NativeAllocationError.Success;
        // Path and mode literals: "/dev/urandom", "r" (use inline bytes so we pass a real C string pointer).
        var path = new InlineBytes64 {
            b00 = 47u8, b01 = 100u8, b02 = 101u8, b03 = 118u8, b04 = 47u8, b05 = 117u8, b06 = 114u8, b07 = 97u8, b08 = 110u8, b09 = 100u8, b10 = 111u8, b11 = 109u8, b12 = 0u8,
        }
        ;
        var mode = new InlineBytes64 {
            b00 = 114u8, b01 = 0u8,
        }
        ;
        _test_use_fake_io = true;
        let file = OpenRandomFile(NativePtr.AsConstPtr(& path.b00), NativePtr.AsConstPtr(& mode.b00));
        ok = ok && !NativePtr.IsNull(file);
        let read = ReadRandom(buffer.Pointer, 1usize, 4usize, file);
        ok = ok && read == 4usize;
        let closeStatus = CloseRandom(file);
        ok = ok && closeStatus == 0;
        _test_use_fake_io = false;
        _test_fail_open = true;
        let failedFile = OpenRandomFile(NativePtr.AsConstPtr(& path.b00), NativePtr.AsConstPtr(& mode.b00));
        ok = ok && NativePtr.IsNull(failedFile);
        _test_use_fake_io = true;
        _test_read_limit = 1usize;
        let filled = chic_rt_random_fill(buffer.Pointer, 4usize);
        ok = ok && filled;
        _test_fail_read = true;
        let failed = chic_rt_random_fill(buffer.Pointer, 4usize);
        ok = ok && !failed;
        _test_read_limit = 0usize;
        _test_use_fake_io = false;
        if (!NativePtr.IsNull (buffer.Pointer))
        {
            NativeAlloc.Free(buffer);
        }
        return ok;
    }
    @extern("C") private static extern * mut @expose_address byte fopen(* const @readonly @expose_address byte path, * const @readonly @expose_address byte mode);
    @extern("C") private static extern usize fread(* mut @expose_address byte ptr, usize size, usize count, * mut @expose_address byte stream);
    @extern("C") private static extern int fclose(* mut @expose_address byte stream);
    private unsafe static * mut @expose_address byte OpenRandomFile(* const @readonly @expose_address byte path, * const @readonly @expose_address byte mode) {
        if (_test_fail_open)
        {
            _test_fail_open = false;
            return NativePtr.NullMut();
        }
        if (_test_use_fake_io)
        {
            return NativePtr.FromIsize(1);
        }
        return fopen(path, mode);
    }
    private unsafe static usize ReadRandom(* mut @expose_address byte ptr, usize size, usize count, * mut @expose_address byte stream) {
        let _ = stream;
        if (_test_use_fake_io)
        {
            var total = size * count;
            if (_test_read_limit >0usize && total >_test_read_limit)
            {
                total = _test_read_limit;
            }
            var idx = 0usize;
            while (idx <total)
            {
                let cursor = NativePtr.OffsetMut(ptr, (isize) idx);
                * cursor = _test_fake_byte;
                idx = idx + 1usize;
            }
            return total;
        }
        return fread(ptr, size, count, stream);
    }
    private unsafe static int CloseRandom(* mut @expose_address byte stream) {
        if (_test_use_fake_io)
        {
            return 0;
        }
        return fclose(stream);
    }
    @extern("C") @export("chic_rt_random_fill") public unsafe static bool chic_rt_random_fill(* mut @expose_address byte buffer,
    usize length) {
        if (length == 0usize)
        {
            return true;
        }
        if (NativePtr.IsNull (buffer))
        {
            return false;
        }
        // Path and mode literals: "/dev/urandom", "r" (use inline bytes so we pass a real C string pointer).
        var path = new InlineBytes64 {
            b00 = 47u8, b01 = 100u8, b02 = 101u8, b03 = 118u8, b04 = 47u8, b05 = 117u8, b06 = 114u8, b07 = 97u8, b08 = 110u8, b09 = 100u8, b10 = 111u8, b11 = 109u8, b12 = 0u8,
        }
        ;
        var mode = new InlineBytes64 {
            b00 = 114u8, b01 = 0u8,
        }
        ;
        let file = OpenRandomFile(NativePtr.AsConstPtr(& path.b00), NativePtr.AsConstPtr(& mode.b00));
        if (NativePtr.IsNull (file))
        {
            return false;
        }
        var remaining = length;
        var offset = 0isize;
        while (remaining >0usize)
        {
            var read = ReadRandom(NativePtr.OffsetMut(buffer, offset), 1usize, remaining, file);
            if (_test_fail_read)
            {
                _test_fail_read = false;
                read = 0usize;
            }
            if (_test_read_limit >0usize && read >_test_read_limit)
            {
                read = _test_read_limit;
            }
            if (read == 0usize)
            {
                let _ = CloseRandom(file);
                return false;
            }
            remaining = remaining - read;
            offset = offset + AsIsize(read);
        }
        let _ = CloseRandom(file);
        return true;
    }
    private static isize AsIsize(usize value) {
        return(isize) value;
    }
}
