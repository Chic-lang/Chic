namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;
testcase Given_pending_exception_roundtrip_When_executed_Then_pending_exception_roundtrip()
{
    PendingExceptionRuntime.chic_rt_clear_pending_exception();
    let empty = PendingExceptionRuntime.chic_rt_has_pending_exception();
    PendingExceptionRuntime.chic_rt_throw(123L, 456L);
    let hasPending = PendingExceptionRuntime.chic_rt_has_pending_exception();
    var payload = 0L;
    var typeId = 0L;
    unsafe {
        let peek = PendingExceptionRuntime.chic_rt_peek_pending_exception(& payload, & typeId);
        let peekPayload = payload;
        let peekType = typeId;
        let take = PendingExceptionRuntime.chic_rt_take_pending_exception(& payload, & typeId);
        let ok = empty == 0 && hasPending == 1 && peek == 1 && peekPayload == 123L && peekType == 456L && take == 1 && payload == 123L && typeId == 456L && PendingExceptionRuntime.chic_rt_has_pending_exception() == 0;
        Assert.That(ok).IsTrue();
    }
}
testcase Given_pending_exception_null_and_empty_paths_When_executed_Then_pending_exception_null_and_empty_paths()
{
    PendingExceptionRuntime.chic_rt_clear_pending_exception();
    unsafe {
        let peekEmpty = PendingExceptionRuntime.chic_rt_peek_pending_exception((* mut i64) NativePtr.NullMut(), (* mut i64) NativePtr.NullMut());
        let takeEmpty = PendingExceptionRuntime.chic_rt_take_pending_exception((* mut i64) NativePtr.NullMut(), (* mut i64) NativePtr.NullMut());
        PendingExceptionRuntime.chic_rt_throw(1L, 2L);
        let peekNull = PendingExceptionRuntime.chic_rt_peek_pending_exception((* mut i64) NativePtr.NullMut(), (* mut i64) NativePtr.NullMut());
        let ok = peekEmpty == 0 && takeEmpty == 0 && peekNull == 1;
        Assert.That(ok).IsTrue();
        PendingExceptionRuntime.chic_rt_clear_pending_exception();
    }
}
testcase Given_pending_exception_null_payload_When_peeked_Then_type_id_written()
{
    PendingExceptionRuntime.chic_rt_clear_pending_exception();
    PendingExceptionRuntime.chic_rt_throw(11L, 22L);
    var typeId = 0L;
    unsafe {
        let peek = PendingExceptionRuntime.chic_rt_peek_pending_exception((* mut i64) NativePtr.NullMut(), & typeId);
        let ok = peek == 1 && typeId == 22L;
        Assert.That(ok).IsTrue();
    }
    PendingExceptionRuntime.chic_rt_clear_pending_exception();
}
testcase Given_pending_exception_null_type_When_peeked_Then_payload_written()
{
    PendingExceptionRuntime.chic_rt_clear_pending_exception();
    PendingExceptionRuntime.chic_rt_throw(33L, 44L);
    var payload = 0L;
    unsafe {
        let peek = PendingExceptionRuntime.chic_rt_peek_pending_exception(& payload, (* mut i64) NativePtr.NullMut());
        let ok = peek == 1 && payload == 33L;
        Assert.That(ok).IsTrue();
    }
    PendingExceptionRuntime.chic_rt_clear_pending_exception();
}
