namespace Std.Runtime.Native
{
    public static class PendingExceptionRuntime
    {
        @threadlocal private static bool _has_pending;
        @threadlocal private static i64 _pending_payload;
        @threadlocal private static i64 _pending_type_id;
        @extern("C") private static extern void abort();
        @extern("C") @export("chic_rt_throw") public static void chic_rt_throw(i64 payload, i64 typeId) {
            _has_pending = true;
            _pending_payload = payload;
            _pending_type_id = typeId;
        }
        @extern("C") @export("chic_rt_has_pending_exception") public static int chic_rt_has_pending_exception() {
            return _has_pending ?1 : 0;
        }
        @extern("C") @export("chic_rt_peek_pending_exception") public unsafe static int chic_rt_peek_pending_exception(* mut i64 payload,
        * mut i64 typeId) {
            if (!_has_pending)
            {
                return 0;
            }
            if (payload != null)
            {
                * payload = _pending_payload;
            }
            if (typeId != null)
            {
                * typeId = _pending_type_id;
            }
            return 1;
        }
        @extern("C") @export("chic_rt_clear_pending_exception") public static void chic_rt_clear_pending_exception() {
            _has_pending = false;
            _pending_payload = 0;
            _pending_type_id = 0;
        }
        @extern("C") @export("chic_rt_take_pending_exception") public unsafe static int chic_rt_take_pending_exception(* mut i64 payload,
        * mut i64 typeId) {
            if (!_has_pending)
            {
                return 0;
            }
            let p = _pending_payload;
            let t = _pending_type_id;
            chic_rt_clear_pending_exception();
            if (payload != null)
            {
                * payload = p;
            }
            if (typeId != null)
            {
                * typeId = t;
            }
            return 1;
        }
        @extern("C") @export("chic_rt_abort_unhandled_exception") public static void chic_rt_abort_unhandled_exception() {
            abort();
        }
    }
    // Panic/abort shims so core runtime calls resolve without a Rust shim.
    public static class PanicRuntime
    {
        @extern("C") private static extern void abort();
        @extern("C") @export("chic_rt_panic") public static int chic_rt_panic(int code) {
            return Halt(code);
        }
        @extern("C") @export("chic_rt_abort") public static int chic_rt_abort(int code) {
            return Halt(code);
        }
        private static int Halt(int code) {
            abort();
            return code;
        }
    }
}
