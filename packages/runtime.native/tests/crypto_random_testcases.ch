namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;
private static void EnableDeterministicCryptoRandom() {
    CryptoRandom.TestUseFakeIo(true);
    CryptoRandom.TestSetFakeByte(17u8);
}
private static void ResetCryptoRandomTestState() {
    CryptoRandom.TestForceOpenFailure(false);
    CryptoRandom.TestForceReadFailure(false);
    CryptoRandom.TestSetReadLimit(0usize);
    CryptoRandom.TestUseFakeIo(false);
}
testcase Given_crypto_random_zero_len_returns_true_When_executed_Then_crypto_random_zero_len_returns_true()
{
    unsafe {
        let empty = CryptoRandom.chic_rt_random_fill((* mut @expose_address byte) NativePtr.NullMut(), 0usize);
        Assert.That(empty).IsTrue();
    }
}
testcase Given_crypto_random_null_buffer_returns_false_When_executed_Then_crypto_random_null_buffer_returns_false()
{
    unsafe {
        let nullBuffer = CryptoRandom.chic_rt_random_fill((* mut @expose_address byte) NativePtr.NullMut(), 4usize);
        Assert.That(nullBuffer).IsFalse();
    }
}
testcase Given_crypto_random_fill_small_buffer_alloc_ok_When_executed_Then_crypto_random_fill_small_buffer_alloc_ok()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(16usize, 1usize);
        EnableDeterministicCryptoRandom();
        let _ = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 16usize);
        ResetCryptoRandomTestState();
        Assert.That(NativePtr.IsNull(buffer.Pointer)).IsFalse();
        MemoryRuntime.chic_rt_free(buffer);
    }
}
testcase Given_crypto_random_fill_small_buffer_filled_When_executed_Then_crypto_random_fill_small_buffer_filled()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(16usize, 1usize);
        EnableDeterministicCryptoRandom();
        let filled = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 16usize);
        ResetCryptoRandomTestState();
        Assert.That(filled).IsTrue();
        MemoryRuntime.chic_rt_free(buffer);
    }
}
testcase Given_crypto_random_fill_large_buffer_alloc_ok_When_executed_Then_crypto_random_fill_large_buffer_alloc_ok()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(128usize, 1usize);
        EnableDeterministicCryptoRandom();
        let _ = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 128usize);
        ResetCryptoRandomTestState();
        Assert.That(NativePtr.IsNull(buffer.Pointer)).IsFalse();
        MemoryRuntime.chic_rt_free(buffer);
    }
}
testcase Given_crypto_random_fill_large_buffer_filled_When_executed_Then_crypto_random_fill_large_buffer_filled()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(128usize, 1usize);
        EnableDeterministicCryptoRandom();
        let filled = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 128usize);
        ResetCryptoRandomTestState();
        Assert.That(filled).IsTrue();
        MemoryRuntime.chic_rt_free(buffer);
    }
}
testcase Given_crypto_random_forced_failures_buffer_ok_When_executed_Then_crypto_random_forced_failures_buffer_ok()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(8usize, 1usize);
        let bufferOk = !NativePtr.IsNull(buffer.Pointer);
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(bufferOk).IsTrue();
    }
}
testcase Given_crypto_random_forced_failures_open_failure_When_executed_Then_crypto_random_forced_failures_open_failure()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(8usize, 1usize);
        CryptoRandom.TestForceOpenFailure(true);
        let openFail = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 8usize);
        CryptoRandom.TestForceOpenFailure(false);
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(openFail).IsFalse();
    }
}
testcase Given_crypto_random_forced_failures_read_failure_When_executed_Then_crypto_random_forced_failures_read_failure()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(8usize, 1usize);
        EnableDeterministicCryptoRandom();
        CryptoRandom.TestForceReadFailure(true);
        let readFail = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 8usize);
        ResetCryptoRandomTestState();
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(readFail).IsFalse();
    }
}
testcase Given_crypto_random_partial_reads_buffer_ok_When_executed_Then_crypto_random_partial_reads_buffer_ok()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(4usize, 1usize);
        let bufferOk = !NativePtr.IsNull(buffer.Pointer);
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(bufferOk).IsTrue();
    }
}
testcase Given_crypto_random_partial_reads_filled_When_executed_Then_crypto_random_partial_reads_filled()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(4usize, 1usize);
        EnableDeterministicCryptoRandom();
        CryptoRandom.TestSetReadLimit(1usize);
        let filled = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 4usize);
        ResetCryptoRandomTestState();
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(filled).IsTrue();
    }
}
testcase Given_crypto_random_fake_io_paths_buffer_ok_When_executed_Then_crypto_random_fake_io_paths_buffer_ok()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(6usize, 1usize);
        let bufferOk = !NativePtr.IsNull(buffer.Pointer);
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(bufferOk).IsTrue();
    }
}
testcase Given_crypto_random_fake_io_paths_filled_When_executed_Then_crypto_random_fake_io_paths_filled()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(6usize, 1usize);
        EnableDeterministicCryptoRandom();
        CryptoRandom.TestSetReadLimit(2usize);
        let filled = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 6usize);
        ResetCryptoRandomTestState();
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(filled).IsTrue();
    }
}
testcase Given_crypto_random_fake_io_paths_first_byte_When_executed_Then_crypto_random_fake_io_paths_first_byte()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(6usize, 1usize);
        EnableDeterministicCryptoRandom();
        CryptoRandom.TestSetReadLimit(2usize);
        let _ = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 6usize);
        let first = * buffer.Pointer;
        ResetCryptoRandomTestState();
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(first).IsEqualTo(17u8);
    }
}
testcase Given_crypto_random_fake_io_paths_last_byte_When_executed_Then_crypto_random_fake_io_paths_last_byte()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(6usize, 1usize);
        EnableDeterministicCryptoRandom();
        CryptoRandom.TestSetReadLimit(2usize);
        let _ = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 6usize);
        let last = * NativePtr.OffsetMut(buffer.Pointer, 5isize);
        ResetCryptoRandomTestState();
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(last).IsEqualTo(17u8);
    }
}
testcase Given_crypto_random_fake_io_full_sweep_buffer_ok_When_executed_Then_crypto_random_fake_io_full_sweep_buffer_ok()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(5usize, 1usize);
        let bufferOk = !NativePtr.IsNull(buffer.Pointer);
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(bufferOk).IsTrue();
    }
}
testcase Given_crypto_random_fake_io_full_sweep_filled_When_executed_Then_crypto_random_fake_io_full_sweep_filled()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(5usize, 1usize);
        EnableDeterministicCryptoRandom();
        CryptoRandom.TestSetFakeByte(3u8);
        CryptoRandom.TestSetReadLimit(1usize);
        let filled = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 5usize);
        ResetCryptoRandomTestState();
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(filled).IsTrue();
    }
}
testcase Given_crypto_random_fake_io_full_sweep_read_failure_When_executed_Then_crypto_random_fake_io_full_sweep_read_failure()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(5usize, 1usize);
        EnableDeterministicCryptoRandom();
        CryptoRandom.TestSetFakeByte(3u8);
        CryptoRandom.TestSetReadLimit(1usize);
        let _ = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 5usize);
        CryptoRandom.TestForceReadFailure(true);
        let failed = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 5usize);
        ResetCryptoRandomTestState();
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(failed).IsFalse();
    }
}
testcase Given_crypto_random_coverage_sweep_When_executed_Then_crypto_random_coverage_sweep()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(9usize, 1usize);
        var ok = !NativePtr.IsNull(buffer.Pointer);
        EnableDeterministicCryptoRandom();
        CryptoRandom.TestSetFakeByte(9u8);
        CryptoRandom.TestSetReadLimit(2usize);
        let filled = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 9usize);
        ok = ok && filled;
        CryptoRandom.TestForceReadFailure(true);
        let failedRead = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 9usize);
        ok = ok && !failedRead;
        CryptoRandom.TestForceOpenFailure(true);
        let failedOpen = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 9usize);
        ok = ok && !failedOpen;
        let zeroLen = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 0usize);
        ok = ok && zeroLen;
        ok = ok && CryptoRandom.TestCoverageSweep();
        ResetCryptoRandomTestState();
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(ok).IsTrue();
    }
}
