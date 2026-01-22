namespace Std.Runtime.NoStd.Tests;
import Std.Runtime.NoStd;
import Std.Testing;

testcase Given_panic_handlers_return_with_test_hook_When_executed_Then_panic_handlers_return_with_test_hook()
{
    PanicHandlers.TestEnabled = true;
    PanicHandlers.TestSpinCount = 1u;
    Assert.That(PanicHandlers.Panic(7)).IsEqualTo(7);
    PanicHandlers.TestEnabled = false;
    PanicHandlers.TestSpinCount = 0u;
}

testcase Given_panic_handlers_abort_returns_value_When_executed_Then_panic_handlers_abort_returns_value()
{
    PanicHandlers.TestEnabled = true;
    PanicHandlers.TestSpinCount = 1u;
    Assert.That(PanicHandlers.Abort(9)).IsEqualTo(9);
    PanicHandlers.TestEnabled = false;
    PanicHandlers.TestSpinCount = 0u;
}

@group("native") testcase Given_pending_exception_roundtrip_When_executed_Then_pending_exception_roundtrip()
{
    PendingExceptionRuntime.ClearPendingException();
    unsafe {
        let peek = PendingExceptionRuntime.PeekPendingException(null, null);
        Assert.That(peek).IsEqualTo(0);
    }
}

@group("native") testcase Given_pending_exception_throw_sets_flag_When_executed_Then_pending_exception_throw_sets_flag()
{
    PendingExceptionRuntime.ClearPendingException();
    PendingExceptionRuntime.ThrowPending(5L, 6L);
    unsafe {
        let peek = PendingExceptionRuntime.PeekPendingException(null, null);
        Assert.That(peek).IsEqualTo(1);
    }
}

@group("native") testcase Given_pending_exception_peek_with_nulls_When_executed_Then_pending_exception_peek_with_nulls()
{
    PendingExceptionRuntime.ClearPendingException();
    PendingExceptionRuntime.ThrowPending(5L, 6L);
    unsafe {
        let peek = PendingExceptionRuntime.PeekPendingException(null, null);
        Assert.That(peek).IsEqualTo(1);
    }
}

@group("native") testcase Given_pending_exception_peek_with_nulls_preserves_flag_When_executed_Then_pending_exception_peek_with_nulls_preserves_flag()
{
    PendingExceptionRuntime.ClearPendingException();
    PendingExceptionRuntime.ThrowPending(5L, 6L);
    unsafe {
        let _ = PendingExceptionRuntime.PeekPendingException(null, null);
        let peek = PendingExceptionRuntime.PeekPendingException(null, null);
        Assert.That(peek).IsEqualTo(1);
    }
}

@group("native") testcase Given_pending_exception_peek_without_pending_returns_zero_When_executed_Then_pending_exception_peek_without_pending_returns_zero()
{
    PendingExceptionRuntime.ClearPendingException();
    unsafe {
        let peek = PendingExceptionRuntime.PeekPendingException(null, null);
        Assert.That(peek).IsEqualTo(0);
    }
}

@group("native") testcase Given_pending_exception_peek_without_pending_flag_is_zero_When_executed_Then_pending_exception_peek_without_pending_flag_is_zero()
{
    PendingExceptionRuntime.ClearPendingException();
    unsafe {
        let _ = PendingExceptionRuntime.PeekPendingException(null, null);
        let peek = PendingExceptionRuntime.PeekPendingException(null, null);
        Assert.That(peek).IsEqualTo(0);
    }
}

@group("native") testcase Given_pending_exception_take_with_nulls_returns_one_When_executed_Then_pending_exception_take_with_nulls_returns_one()
{
    PendingExceptionRuntime.ClearPendingException();
    PendingExceptionRuntime.ThrowPending(5L, 6L);
    unsafe {
        let take = PendingExceptionRuntime.TakePendingException(null, null);
        Assert.That(take).IsEqualTo(1);
    }
}

@group("native") testcase Given_pending_exception_take_with_nulls_clears_flag_When_executed_Then_pending_exception_take_with_nulls_clears_flag()
{
    PendingExceptionRuntime.ClearPendingException();
    PendingExceptionRuntime.ThrowPending(5L, 6L);
    unsafe {
        let _ = PendingExceptionRuntime.TakePendingException(null, null);
        let peek = PendingExceptionRuntime.PeekPendingException(null, null);
        Assert.That(peek).IsEqualTo(0);
    }
}

@group("native") testcase Given_pending_exception_take_without_pending_returns_zero_When_executed_Then_pending_exception_take_without_pending_returns_zero()
{
    PendingExceptionRuntime.ClearPendingException();
    unsafe {
        let take = PendingExceptionRuntime.TakePendingException(null, null);
        Assert.That(take).IsEqualTo(0);
    }
}

@group("native") testcase Given_pending_exception_take_without_pending_flag_is_zero_When_executed_Then_pending_exception_take_without_pending_flag_is_zero()
{
    PendingExceptionRuntime.ClearPendingException();
    unsafe {
        let _ = PendingExceptionRuntime.TakePendingException(null, null);
        let peek = PendingExceptionRuntime.PeekPendingException(null, null);
        Assert.That(peek).IsEqualTo(0);
    }
}

@group("native") testcase Given_pending_exception_peek_returns_one_When_executed_Then_pending_exception_peek_returns_one()
{
    PendingExceptionRuntime.ClearPendingException();
    PendingExceptionRuntime.ThrowPending(5L, 6L);
    unsafe {
        let payloadPtr = (* mut @expose_address long) 0x2000usize;
        let typeIdPtr = (* mut @expose_address long) 0x2008usize;
        * payloadPtr = 0L;
        * typeIdPtr = 0L;
        let peek = PendingExceptionRuntime.PeekPendingException(payloadPtr, typeIdPtr);
        Assert.That(peek).IsEqualTo(1);
    }
}

@group("native") testcase Given_pending_exception_peek_writes_payload_When_executed_Then_pending_exception_peek_writes_payload()
{
    PendingExceptionRuntime.ClearPendingException();
    PendingExceptionRuntime.ThrowPending(5L, 6L);
    unsafe {
        let payloadPtr = (* mut @expose_address long) 0x2000usize;
        let typeIdPtr = (* mut @expose_address long) 0x2008usize;
        * payloadPtr = 0L;
        * typeIdPtr = 0L;
        let _ = PendingExceptionRuntime.PeekPendingException(payloadPtr, typeIdPtr);
        Assert.That(* payloadPtr).IsEqualTo(5L);
    }
}

@group("native") testcase Given_pending_exception_peek_writes_type_id_When_executed_Then_pending_exception_peek_writes_type_id()
{
    PendingExceptionRuntime.ClearPendingException();
    PendingExceptionRuntime.ThrowPending(5L, 6L);
    unsafe {
        let payloadPtr = (* mut @expose_address long) 0x2000usize;
        let typeIdPtr = (* mut @expose_address long) 0x2008usize;
        * payloadPtr = 0L;
        * typeIdPtr = 0L;
        let _ = PendingExceptionRuntime.PeekPendingException(payloadPtr, typeIdPtr);
        Assert.That(* typeIdPtr).IsEqualTo(6L);
    }
}

@group("native") testcase Given_pending_exception_take_returns_one_When_executed_Then_pending_exception_take_returns_one()
{
    PendingExceptionRuntime.ClearPendingException();
    PendingExceptionRuntime.ThrowPending(7L, 8L);
    unsafe {
        let payloadPtr = (* mut @expose_address long) 0x2000usize;
        let typeIdPtr = (* mut @expose_address long) 0x2008usize;
        * payloadPtr = 0L;
        * typeIdPtr = 0L;
        let take = PendingExceptionRuntime.TakePendingException(payloadPtr, typeIdPtr);
        Assert.That(take).IsEqualTo(1);
    }
}

@group("native") testcase Given_pending_exception_take_writes_payload_When_executed_Then_pending_exception_take_writes_payload()
{
    PendingExceptionRuntime.ClearPendingException();
    PendingExceptionRuntime.ThrowPending(7L, 8L);
    unsafe {
        let payloadPtr = (* mut @expose_address long) 0x2000usize;
        let typeIdPtr = (* mut @expose_address long) 0x2008usize;
        * payloadPtr = 0L;
        * typeIdPtr = 0L;
        let _ = PendingExceptionRuntime.TakePendingException(payloadPtr, typeIdPtr);
        Assert.That(* payloadPtr).IsEqualTo(7L);
    }
}

@group("native") testcase Given_pending_exception_take_writes_type_id_When_executed_Then_pending_exception_take_writes_type_id()
{
    PendingExceptionRuntime.ClearPendingException();
    PendingExceptionRuntime.ThrowPending(7L, 8L);
    unsafe {
        let payloadPtr = (* mut @expose_address long) 0x2000usize;
        let typeIdPtr = (* mut @expose_address long) 0x2008usize;
        * payloadPtr = 0L;
        * typeIdPtr = 0L;
        let _ = PendingExceptionRuntime.TakePendingException(payloadPtr, typeIdPtr);
        Assert.That(* typeIdPtr).IsEqualTo(8L);
    }
}

@group("native") testcase Given_pending_exception_take_clears_flag_When_executed_Then_pending_exception_take_clears_flag()
{
    PendingExceptionRuntime.ClearPendingException();
    PendingExceptionRuntime.ThrowPending(7L, 8L);
    unsafe {
        let _ = PendingExceptionRuntime.TakePendingException(null, null);
        let peek = PendingExceptionRuntime.PeekPendingException(null, null);
        Assert.That(peek).IsEqualTo(0);
    }
}
