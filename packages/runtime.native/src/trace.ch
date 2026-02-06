namespace Std.Runtime.Native;
// Chic-native tracing buffer and JSON flush, replacing the legacy C shim.
@repr(c) internal struct TraceEvent
{
    public u64 TraceId;
    public * mut @expose_address byte LabelPtr;
    public usize LabelLen;
    public u64 StartNs;
    public u64 EndNs;
}
@repr(c) internal struct TraceTimespec
{
    public i64 tv_sec;
    public i64 tv_nsec;
}
public static class TraceRuntime
{
    @extern("C") private static extern int clock_gettime(int clk_id, * mut @expose_address TraceTimespec ts);
    @extern("C") private static extern int pthread_mutex_lock(* mut @expose_address byte mutex);
    @extern("C") private static extern int pthread_mutex_unlock(* mut @expose_address byte mutex);
    @extern("C") private static extern int pthread_mutex_init(* mut @expose_address byte mutex, * const @readonly @expose_address byte attr);
    @extern("C") private static extern * mut @expose_address byte malloc(usize size);
    @extern("C") private static extern * mut @expose_address byte calloc(usize count, usize size);
    @extern("C") private static extern * mut @expose_address byte realloc(* mut @expose_address byte ptr, usize size);
    @extern("C") private static extern void free(* mut @expose_address byte ptr);
    @extern("C") private static extern * mut @expose_address byte fopen(* const @readonly @expose_address byte path, * const @readonly @expose_address byte mode);
    @extern("C") private static extern int fprintf(* mut @expose_address byte file, * const @readonly @expose_address byte fmt,
    ... );
    @extern("C") private static extern int snprintf(* mut @expose_address byte buffer, usize size, * const @readonly @expose_address byte fmt,
    ... );
    @extern("C") private static extern int fputc(int ch, * mut @expose_address byte file);
    @extern("C") private static extern int fclose(* mut @expose_address byte stream);
    @extern("C") private static extern int fflush(* mut @expose_address byte stream);
    @extern("C") private static extern int fileno(* mut @expose_address byte stream);
    @extern("C") private static extern i64 ftell(* mut @expose_address byte stream);
    @extern("C") private static extern int ftruncate(int fd, i64 length);
    private const int CLOCK_MONOTONIC = 1;
    private const usize EVENT_SIZE = sizeof(TraceEvent);
    private const usize MUTEX_BYTES = 64usize;
    private static * mut @expose_address byte _events;
    private static usize _len = 0;
    private static usize _cap = 0;
    private static * mut @expose_address byte _mutex;
    private static bool _mutex_inited = false;
    private static int _test_fail_alloc_start = - 1;
    private static int _test_fail_alloc_count = 0;
    private static int _test_alloc_step = 0;
    private static bool _test_fail_open = false;
    public static void TestFailAllocAtStep(int step) {
        _test_fail_alloc_start = step;
        _test_fail_alloc_count = 1;
        _test_alloc_step = 0;
    }
    public static void TestFailAllocRange(int start, int count) {
        _test_fail_alloc_start = start;
        _test_fail_alloc_count = count <0 ?0 : count;
        _test_alloc_step = 0;
    }
    public static void TestDisableAllocFailures() {
        _test_fail_alloc_start = - 1;
        _test_fail_alloc_count = 0;
        _test_alloc_step = 0;
    }
    public static void TestForceOpenFailure(bool value) {
        _test_fail_open = value;
    }
    public unsafe static void TestResetState() {
        ClearEvents();
        if (!NativePtr.IsNull (_events))
        {
            free(_events);
        }
        _events = NativePtr.NullMut();
        _len = 0;
        _cap = 0;
        if (!NativePtr.IsNull (_mutex))
        {
            free(_mutex);
        }
        _mutex = NativePtr.NullMut();
        _mutex_inited = false;
        _test_alloc_step = 0;
        _test_fail_alloc_start = - 1;
        _test_fail_alloc_count = 0;
        _test_fail_open = false;
    }
    public unsafe static void TestAppendEscaped(* const @readonly @expose_address byte ptr, usize len) {
        AppendEscaped(ptr, len, NativePtr.NullMut());
    }
    public unsafe static void TestCoverageHelpers() {
        TestDisableAllocFailures();
        TestResetState();
        let _ = EnsureCapacity(0usize);
        let ok = EnsureCapacity(1usize);
        if (ok && !NativePtr.IsNull (_events))
        {
            var ev = EventAt(0usize);
            (* ev).TraceId = 1u64;
            (* ev).StartNs = 0u64;
            (* ev).EndNs = 0u64;
            (* ev).LabelLen = 4usize;
            (* ev).LabelPtr = TraceMalloc(4usize);
            if (!NativePtr.IsNull ( (* ev).LabelPtr))
            {
                let base = (* ev).LabelPtr;
                NativeAlloc.Set(new ValueMutPtr {
                    Pointer = base, Size = 1usize, Alignment = 1usize,
                }
                , 65u8, 1usize);
                NativeAlloc.Set(new ValueMutPtr {
                    Pointer = NativePtr.OffsetMut(base, 1isize), Size = 1usize, Alignment = 1usize,
                }
                , 34u8, 1usize);
                NativeAlloc.Set(new ValueMutPtr {
                    Pointer = NativePtr.OffsetMut(base, 2isize), Size = 1usize, Alignment = 1usize,
                }
                , 92u8, 1usize);
                NativeAlloc.Set(new ValueMutPtr {
                    Pointer = NativePtr.OffsetMut(base, 3isize), Size = 1usize, Alignment = 1usize,
                }
                , 66u8, 1usize);
            }
            _len = 1usize;
        }
        ClearEvents();
        var label = new StringInlineBytes64 {
            b00 = 92, b01 = 34, b02 = 65,
        }
        ;
        AppendEscaped(NativePtr.AsConstPtr(& label.b00), 3usize, NativePtr.NullMut());
        AppendEscaped(NativePtr.NullConst(), 0usize, NativePtr.NullMut());
    }
    public unsafe static bool TestCoverageSweep() {
        var ok = true;
        TestCoverageHelpers();
        TestResetState();
        TestFailAllocAtStep(0);
        let mutexFail = MutexPtr();
        ok = ok && NativePtr.IsNull(mutexFail);
        TestDisableAllocFailures();
        let mutexOk = MutexPtr();
        ok = ok && !NativePtr.IsNull(mutexOk);
        TestFailAllocAtStep(0);
        let capFail = EnsureCapacity(1usize);
        ok = ok && !capFail;
        TestDisableAllocFailures();
        let capOk = EnsureCapacity(1usize);
        ok = ok && capOk;
        TestFailAllocAtStep(0);
        let reallocFail = EnsureCapacity(_cap + 1usize);
        ok = ok && !reallocFail;
        TestDisableAllocFailures();
        let reallocOk = EnsureCapacity(_cap + 1usize);
        ok = ok && reallocOk;
        TestFailAllocAtStep(0);
        let mallocFail = TraceMalloc(4usize);
        ok = ok && NativePtr.IsNull(mallocFail);
        TestDisableAllocFailures();
        let mallocOk = TraceMalloc(4usize);
        ok = ok && !NativePtr.IsNull(mallocOk);
        if (!NativePtr.IsNull (mallocOk))
        {
            free(mallocOk);
        }
        TestFailAllocAtStep(0);
        let callocFail = TraceCalloc(1usize, 4usize);
        ok = ok && NativePtr.IsNull(callocFail);
        TestDisableAllocFailures();
        let callocOk = TraceCalloc(1usize, 4usize);
        ok = ok && !NativePtr.IsNull(callocOk);
        if (!NativePtr.IsNull (callocOk))
        {
            free(callocOk);
        }
        var escLabel = new StringInlineBytes64 {
            b00 = 34, b01 = 92, b02 = 65,
        }
        ;
        let escPtr = NativePtr.AsConstPtr(& escLabel.b00);
        AppendEscaped(escPtr, 3usize, NativePtr.NullMut());
        AppendEscaped(NativePtr.NullConst(), 0usize, NativePtr.NullMut());
        chic_rt_trace_enter(201u64, escPtr, 3u64);
        chic_rt_trace_exit(201u64);
        let emptyStatus = chic_rt_trace_flush(NativePtr.NullConst(), 0u64);
        ok = ok && emptyStatus == 0;
        var path = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101, b05 = 95, b06 = 99, b07 = 111, b08 = 118, b09 = 46, b10 = 106, b11 = 115, b12 = 111, b13 = 110,
        }
        ;
        TestFailAllocRange(0, 2);
        let pathFail = chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 14u64);
        ok = ok && pathFail == - 2;
        TestFailAllocRange(1, 2);
        let modeFail = chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 14u64);
        ok = ok && modeFail == - 3;
        TestDisableAllocFailures();
        TestForceOpenFailure(true);
        let openFail = chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 14u64);
        ok = ok && openFail == - 4;
        TestDisableAllocFailures();
        let flushOk = chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 14u64);
        ok = ok && flushOk == 0;
        TestResetState();
        return ok;
    }
    private static bool ShouldFailAlloc() {
        let step = _test_alloc_step;
        _test_alloc_step = _test_alloc_step + 1;
        return _test_fail_alloc_start >= 0 && step >= _test_fail_alloc_start && step <(_test_fail_alloc_start + _test_fail_alloc_count);
    }
    private unsafe static * mut @expose_address byte TraceMalloc(usize size) {
        if (ShouldFailAlloc ())
        {
            return NativePtr.NullMut();
        }
        return malloc(size);
    }
    private unsafe static * mut @expose_address byte TraceCalloc(usize count, usize size) {
        if (ShouldFailAlloc ())
        {
            return NativePtr.NullMut();
        }
        return calloc(count, size);
    }
    private unsafe static * mut @expose_address byte TraceRealloc(* mut @expose_address byte ptr, usize size) {
        if (ShouldFailAlloc ())
        {
            return NativePtr.NullMut();
        }
        return realloc(ptr, size);
    }
    private unsafe static * mut @expose_address byte TraceFopen(* const @readonly @expose_address byte path, * const @readonly @expose_address byte mode) {
        if (_test_fail_open)
        {
            _test_fail_open = false;
            return NativePtr.NullMut();
        }
        return fopen(path, mode);
    }
    private unsafe static * mut @expose_address byte MutexPtr() {
        if (NativePtr.IsNull (_mutex))
        {
            _mutex = TraceMalloc(MUTEX_BYTES);
            if (NativePtr.IsNull (_mutex))
            {
                return NativePtr.NullMut();
            }
            var i = 0usize;
            while (i <MUTEX_BYTES)
            {
                * NativePtr.OffsetMut(_mutex, (isize) i) = 0u8;
                i = i + 1usize;
            }
        }
        if (!_mutex_inited)
        {
            let _ = pthread_mutex_init(_mutex, NativePtr.NullConst());
            _mutex_inited = true;
        }
        return _mutex;
    }
    private unsafe static void InitMutex() {
        let _ = MutexPtr();
    }
    private static InlineBytes64 ZeroInline64() {
        return new InlineBytes64 {
            b00 = 0, b01 = 0, b02 = 0, b03 = 0, b04 = 0, b05 = 0, b06 = 0, b07 = 0, b08 = 0, b09 = 0, b10 = 0, b11 = 0, b12 = 0, b13 = 0, b14 = 0, b15 = 0, b16 = 0, b17 = 0, b18 = 0, b19 = 0, b20 = 0, b21 = 0, b22 = 0, b23 = 0, b24 = 0, b25 = 0, b26 = 0, b27 = 0, b28 = 0, b29 = 0, b30 = 0, b31 = 0, b32 = 0, b33 = 0, b34 = 0, b35 = 0, b36 = 0, b37 = 0, b38 = 0, b39 = 0, b40 = 0, b41 = 0, b42 = 0, b43 = 0, b44 = 0, b45 = 0, b46 = 0, b47 = 0, b48 = 0, b49 = 0, b50 = 0, b51 = 0, b52 = 0, b53 = 0, b54 = 0, b55 = 0, b56 = 0, b57 = 0, b58 = 0, b59 = 0, b60 = 0, b61 = 0, b62 = 0, b63 = 0,
        }
        ;
    }
    private unsafe static u64 NowNs() {
        var ts = new TraceTimespec {
            tv_sec = 0, tv_nsec = 0
        }
        ;
        let rc = clock_gettime(CLOCK_MONOTONIC, & ts);
        if (rc != 0)
        {
            return 0u64;
        }
        return((u64) ts.tv_sec) * 1_000_000_000u64 + (u64) ts.tv_nsec;
    }
    private unsafe static bool EnsureCapacity(usize needed) {
        if (needed <= _cap)
        {
            return true;
        }
        var newCap = _cap == 0usize ?32usize : _cap;
        while (newCap <needed)
        {
            newCap *= 2usize;
        }
        let bytes = newCap * EVENT_SIZE;
        var newPtr = NativePtr.NullMut();
        if (_cap == 0usize)
        {
            newPtr = TraceMalloc(bytes);
        }
        else
        {
            newPtr = TraceRealloc(_events, bytes);
        }
        if (NativePtr.IsNull (newPtr))
        {
            return false;
        }
        _events = newPtr;
        _cap = newCap;
        return true;
    }
    private unsafe static * mut TraceEvent EventAt(usize index) {
        return(* mut TraceEvent) NativePtr.OffsetMut(_events, (isize)(index * EVENT_SIZE));
    }
    private unsafe static void ClearEvents() {
        if (NativePtr.IsNull (_events))
        {
            _len = 0;
            return;
        }
        for (var i = 0usize; i <_len; i += 1usize) {
            var ev = EventAt(i);
            if (!NativePtr.IsNull ( (* ev).LabelPtr))
            {
                free((* ev).LabelPtr);
            }
            (* ev).LabelPtr = NativePtr.NullMut();
            (* ev).LabelLen = 0usize;
        }
        _len = 0;
    }
    private unsafe static void AppendEscaped(* const @readonly @expose_address byte ptr, usize len, * mut @expose_address byte file) {
        if (NativePtr.IsNullConst (ptr) || len == 0usize)
        {
            return;
        }
        if (NativePtr.IsNull (file))
        {
            return;
        }
        var offset = 0isize;
        while (offset < (isize) len)
        {
            let cursor = NativePtr.OffsetConst(ptr, offset);
            let ch = * cursor;
            if (ch == 34u8 || ch == 92u8)
            // " or \
            {
                let _ = fputc(92, file);
            }
            let _ = fputc((int) ch, file);
            offset = offset + 1isize;
        }
    }
    @extern("C") @export("chic_rt_trace_enter") public unsafe static void chic_rt_trace_enter(u64 trace_id, * const @readonly @expose_address byte label_ptr,
    u64 label_len) {
        InitMutex();
        let mutex = MutexPtr();
        if (NativePtr.IsNull (mutex))
        {
            return;
        }
        let _ = pthread_mutex_lock(mutex);
        if (EnsureCapacity (_len + 1usize) && !NativePtr.IsNull (_events))
        {
            var ev_ptr = EventAt(_len);
            _len = _len + 1usize;
            (* ev_ptr).TraceId = trace_id;
            (* ev_ptr).StartNs = NowNs();
            (* ev_ptr).EndNs = 0u64;
            (* ev_ptr).LabelPtr = NativePtr.NullMut();
            (* ev_ptr).LabelLen = (usize) label_len;
            if (label_len >0u64)
            {
                let alloc = TraceMalloc((usize) label_len);
                if (!NativePtr.IsNull (alloc))
                {
                    for (var i = 0usize; i <(usize) label_len; i += 1usize) {
                        var dst = NativePtr.OffsetMut(alloc, (isize) i);
                        let src = NativePtr.OffsetConst(label_ptr, (isize) i);
                        * dst = * src;
                    }
                    (* ev_ptr).LabelPtr = alloc;
                }
            }
        }
        let _ = pthread_mutex_unlock(mutex);
    }
    @extern("C") @export("chic_rt_trace_exit") public unsafe static void chic_rt_trace_exit(u64 trace_id) {
        InitMutex();
        let mutex = MutexPtr();
        if (NativePtr.IsNull (mutex))
        {
            return;
        }
        let _ = pthread_mutex_lock(mutex);
        if (NativePtr.IsNull (_events))
        {
            let _ = pthread_mutex_unlock(mutex);
            return;
        }
        var idx = _len;
        while (idx >0usize)
        {
            idx -= 1usize;
            var ev = EventAt(idx);
            if ( (* ev).TraceId == trace_id && (* ev).EndNs == 0u64)
            {
                (* ev).EndNs = NowNs();
                let _ = pthread_mutex_unlock(mutex);
                return;
            }
        }
        let _ = pthread_mutex_unlock(mutex);
    }
    @extern("C") @export("chic_rt_trace_flush") public unsafe static i32 chic_rt_trace_flush(* const @readonly @expose_address byte path_ptr,
    u64 len) {
        InitMutex();
        let mutex = MutexPtr();
        if (NativePtr.IsNull (mutex))
        {
            return 0;
        }
        let _ = pthread_mutex_lock(mutex);
        var status = - 1i32;
        if (!NativePtr.IsNullConst (path_ptr) && len >0u64)
        {
            var path_buf = TraceMalloc((usize) len + 1usize);
            if (NativePtr.IsNull (path_buf))
            {
                path_buf = TraceCalloc(1usize, (usize) len + 1usize);
            }
            if (NativePtr.IsNull (path_buf))
            {
                status = - 2i32;
            }
            else
            {
                for (var i = 0usize; i <(usize) len; i += 1usize) {
                    var dst = NativePtr.FromIsize(NativePtr.ToIsize(path_buf) + (isize) i);
                    let src = NativePtr.FromIsizeConst(NativePtr.ToIsizeConst(path_ptr) + (isize) i);
                    * dst = * src;
                }
                var term = NativePtr.FromIsize(NativePtr.ToIsize(path_buf) + (isize) len);
                * term = 0u8;
                var mode_buf = TraceMalloc(2usize);
                if (NativePtr.IsNull (mode_buf))
                {
                    mode_buf = TraceCalloc(1usize, 2usize);
                }
                if (NativePtr.IsNull (mode_buf))
                {
                    status = - 3i32;
                }
                else
                {
                    * mode_buf = 119u8;
                    // 'w'
                    * NativePtr.OffsetMut(mode_buf, 1isize) = 0u8;
                    let file = TraceFopen(NativePtr.AsConstPtr(path_buf), NativePtr.AsConstPtr(mode_buf));
                    free(mode_buf);
                    if (!NativePtr.IsNull (file))
                    {
                        let fd = fileno(file);
                        if (fd >= 0)
                        {
                            let _ = ftruncate(fd, 0);
                        }
                        var label_buf = ZeroInline64();
                        var base_ptr = & label_buf.b00;
                        StoreByte(base_ptr, 87);
                        // W
                        StoreByte(NativePtr.OffsetMut(base_ptr, 1isize), 97);
                        // a
                        StoreByte(NativePtr.OffsetMut(base_ptr, 2isize), 115);
                        // s
                        StoreByte(NativePtr.OffsetMut(base_ptr, 3isize), 109);
                        // m
                        StoreByte(NativePtr.OffsetMut(base_ptr, 4isize), 58);
                        // :
                        StoreByte(NativePtr.OffsetMut(base_ptr, 5isize), 58);
                        // :
                        StoreByte(NativePtr.OffsetMut(base_ptr, 6isize), 116);
                        // t
                        StoreByte(NativePtr.OffsetMut(base_ptr, 7isize), 114);
                        // r
                        StoreByte(NativePtr.OffsetMut(base_ptr, 8isize), 97);
                        // a
                        StoreByte(NativePtr.OffsetMut(base_ptr, 9isize), 99);
                        // c
                        StoreByte(NativePtr.OffsetMut(base_ptr, 10isize), 101);
                        // e
                        let default_label_ptr = NativePtr.AsConstPtr(& label_buf.b00);
                        let default_label_len = 11usize;
                        let metrics_count_raw = (!NativePtr.IsNull(_events) && _len >0usize) ?_len : 1usize;
                        let metrics_count = metrics_count_raw >1024usize ?1024usize : metrics_count_raw;
                        let metrics_capacity = metrics_count * 192usize + 64usize;
                        var metrics_buf = TraceMalloc(metrics_capacity);
                        if (NativePtr.IsNull (metrics_buf))
                        {
                            metrics_buf = TraceCalloc(1usize, metrics_capacity);
                        }
                        var metrics_len = 0usize;
                        var idx = 0usize;
                        while (!NativePtr.IsNull (metrics_buf) && idx <metrics_count)
                        {
                            var trace_id = 0u64;
                            var cpu_us = 0u64;
                            var label_ptr_for_emit = default_label_ptr;
                            var label_len_for_emit = default_label_len;
                            var label_tmp = ZeroInline64();
                            let max_label_bytes = 64usize;
                            if (!NativePtr.IsNull (_events) && _len >0usize && idx <_len)
                            {
                                var ev = EventAt(idx);
                                trace_id = (* ev).TraceId;
                                let start_ns = (* ev).StartNs;
                                let end_ns = (* ev).EndNs == 0u64 ?(* ev).StartNs : (* ev).EndNs;
                                cpu_us = (end_ns - start_ns) / 1000u64;
                                let has_label = !NativePtr.IsNull((* ev).LabelPtr) && (* ev).LabelLen >0usize && (* ev).LabelLen <= max_label_bytes;
                                let src_ptr = has_label ?NativePtr.AsConstPtr((* ev).LabelPtr) : default_label_ptr;
                                let src_len = has_label ?(* ev).LabelLen : default_label_len;
                                let clamped_len = src_len >max_label_bytes ?max_label_bytes : src_len;
                                var pos = 0usize;
                                var dst_ptr = & label_tmp.b00;
                                while (pos <clamped_len)
                                {
                                    var dst = NativePtr.OffsetMut(dst_ptr, (isize) pos);
                                    let src = NativePtr.OffsetConst(src_ptr, (isize) pos);
                                    * dst = * src;
                                    pos = pos + 1usize;
                                }
                                label_ptr_for_emit = NativePtr.AsConstPtr(& label_tmp.b00);
                                label_len_for_emit = clamped_len;
                            }
                            let remaining = metrics_capacity - metrics_len;
                            if (remaining <= 1usize)
                            {
                                break;
                            }
                            let written = (usize) snprintf(NativePtr.FromIsize(NativePtr.ToIsize(metrics_buf) + (isize) metrics_len),
                            remaining, "{\"trace_id\":%llu,\"mir_id\":\"\",\"label\":\"%.*s\",\"cpu_us\":%llu,\"budget_cpu_us\":null,\"budget_gpu_us\":null,\"budget_mem_bytes\":null}",
                            trace_id, (int) label_len_for_emit, label_ptr_for_emit, cpu_us);
                            if (written >= remaining)
                            {
                                break;
                            }
                            metrics_len = metrics_len + written;
                            if (idx + 1usize <metrics_count && metrics_len + 1usize <metrics_capacity)
                            {
                                var comma_ptr = NativePtr.FromIsize(NativePtr.ToIsize(metrics_buf) + (isize) metrics_len);
                                * comma_ptr = 44u8;
                                // ','
                                metrics_len = metrics_len + 1usize;
                            }
                            idx = idx + 1usize;
                        }
                        if (!NativePtr.IsNull (metrics_buf))
                        {
                            let _ = fprintf(file, "{\"version\":\"0.1\",\"target\":\"wasm-executor\",\"runs\":[{\"profile\":\"default\",\"metrics\":[%.*s]}]}\n",
                            (int) metrics_len, NativePtr.AsConstPtr(metrics_buf));
                            free(metrics_buf);
                        }
                        else
                        {
                            let _ = fprintf(file, "{\"version\":\"0.1\",\"target\":\"wasm-executor\",\"runs\":[{\"profile\":\"default\",\"metrics\":[]}]}\\n");
                        }
                        let _ = fflush(file);
                        let _ = fclose(file);
                        status = 0i32;
                    }
                    else
                    {
                        status = - 4i32;
                    }
                }
                free(path_buf);
            }
        }
        else
        {
            status = 0i32;
        }
        if (status == 0i32)
        {
            ClearEvents();
        }
        let _ = pthread_mutex_unlock(mutex);
        return status;
    }
}
