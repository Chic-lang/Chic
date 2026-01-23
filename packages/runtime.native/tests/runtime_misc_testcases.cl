namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;
testcase Given_crypto_random_handles_empty_buffer_When_executed_Then_crypto_random_handles_empty_buffer()
{
    let ok = CryptoRandom.chic_rt_random_fill(NativePtr.NullMut(), 0usize);
    Assert.That(ok).IsTrue();
}
testcase Given_crypto_random_rejects_null_buffer_When_executed_Then_crypto_random_rejects_null_buffer()
{
    let fail = CryptoRandom.chic_rt_random_fill(NativePtr.NullMut(), 4usize);
    Assert.That(fail).IsFalse();
}
testcase Given_crypto_random_fills_buffer_alloc_ok_When_executed_Then_crypto_random_fills_buffer_alloc_ok()
{
    unsafe {
        var buffer = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 16usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(16usize, 1usize, out buffer);
        let _ = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 16usize);
        Assert.That((int) status).IsEqualTo((int) NativeAllocationError.Success);
        NativeAlloc.Free(buffer);
    }
}
testcase Given_crypto_random_fills_buffer_ok_When_executed_Then_crypto_random_fills_buffer_ok()
{
    unsafe {
        var buffer = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 16usize, Alignment = 1usize
        }
        ;
        let _ = NativeAlloc.AllocZeroed(16usize, 1usize, out buffer);
        let ok = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 16usize);
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(buffer);
    }
}
testcase Given_debug_mark_does_not_throw_When_executed_Then_debug_mark_does_not_throw()
{
    DebugMark.chic_rt_debug_mark(7u64, 1u64, 2u64, 3u64);
    Assert.That(true).IsTrue();
}
testcase Given_float_ops_remainder_f32_matches_expected_When_executed_Then_float_ops_remainder_f32_matches_expected()
{
    let f32rem = chic_rt_f32_rem(5.5f, 2f);
    let f64rem = chic_rt_f64_rem(9.0, 4.0);
    Assert.That(f32rem == 1.5f).IsTrue();
}
testcase Given_float_ops_remainder_f64_matches_expected_When_executed_Then_float_ops_remainder_f64_matches_expected()
{
    let _ = chic_rt_f32_rem(5.5f, 2f);
    let f64rem = chic_rt_f64_rem(9.0, 4.0);
    Assert.That(f64rem == 1.0).IsTrue();
}
testcase Given_socket_shims_accept_negative_on_invalid_When_executed_Then_socket_shims_accept_negative_on_invalid()
{
    unsafe {
        let addr = (* mut SocketShims.SockAddrIn6) NativePtr.NullMut();
        let len_ptr = (* mut int) NativePtr.NullMut();
        let accept = SocketShims.Accept6(- 1, addr, len_ptr);
        Assert.That(accept <= 0).IsTrue();
    }
}
testcase Given_socket_shims_bind_negative_on_invalid_When_executed_Then_socket_shims_bind_negative_on_invalid()
{
    unsafe {
        let addr = (* mut SocketShims.SockAddrIn6) NativePtr.NullMut();
        let bind = SocketShims.Bind6(- 1, addr, 0);
        Assert.That(bind <= 0).IsTrue();
    }
}
testcase Given_socket_shims_connect_negative_on_invalid_When_executed_Then_socket_shims_connect_negative_on_invalid()
{
    unsafe {
        let addr = (* mut SocketShims.SockAddrIn6) NativePtr.NullMut();
        let connect = SocketShims.Connect6(- 1, addr, 0);
        Assert.That(connect <= 0).IsTrue();
    }
}
testcase Given_socket_shims_receive_negative_on_invalid_When_executed_Then_socket_shims_receive_negative_on_invalid()
{
    unsafe {
        let addr = (* mut SocketShims.SockAddrIn6) NativePtr.NullMut();
        let len_ptr = (* mut int) NativePtr.NullMut();
        let recv = SocketShims.ReceiveFrom6(- 1, NativePtr.NullMut(), 0usize, 0, addr, len_ptr);
        Assert.That(recv <= 0).IsTrue();
    }
}
testcase Given_socket_shims_send_negative_on_invalid_When_executed_Then_socket_shims_send_negative_on_invalid()
{
    unsafe {
        let send = SocketShims.SendTo6(- 1, NativePtr.NullConst(), 0usize, 0, (* const SocketShims.SockAddrIn6) NativePtr.NullConst(),
        0);
        Assert.That(send <= 0).IsTrue();
    }
}
testcase Given_test_executor_stub_returns_zero_When_executed_Then_test_executor_stub_returns_zero()
{
    unsafe {
        let code = chic_rt_test_executor_run_all();
        Assert.That(code).IsEqualTo(0);
    }
}
