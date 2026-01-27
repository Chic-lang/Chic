namespace Std.Runtime.NoStd
{
    public static class PendingExceptionRuntime
    {
        private static bool _has_pending;
        private static i64 _pending_payload;
        private static i64 _pending_type_id;
        @export("chic_rt_throw") public static void chic_rt_throw(i64 payload, i64 typeId) {
            _has_pending = true;
            _pending_payload = payload;
            _pending_type_id = typeId;
        }
        @export("chic_rt_has_pending_exception") public static int chic_rt_has_pending_exception() {
            return _has_pending ?1 : 0;
        }
        @export("chic_rt_peek_pending_exception") public unsafe static int chic_rt_peek_pending_exception(* mut i64 payload,
        * mut i64 typeId) {
            if (! _has_pending)
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
        @export("chic_rt_clear_pending_exception") public static void chic_rt_clear_pending_exception() {
            _has_pending = false;
            _pending_payload = (i64) 0;
            _pending_type_id = (i64) 0;
        }
        @export("chic_rt_take_pending_exception") public unsafe static int chic_rt_take_pending_exception(* mut i64 payload,
        * mut i64 typeId) {
            if (! _has_pending)
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
    }
}
