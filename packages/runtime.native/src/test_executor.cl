namespace Std.Runtime.Native;
// Bootstrap test executor hook. The driver launches test binaries with `--run-tests` and
// `--chic-test-indexes=<comma list>` / `--chic-test-fail-fast` arguments. This runtime routine executes
// the selected testcases from the startup descriptor and emits machine-readable status lines:
// `CHIC_TESTCASE\t<index>\tPASS|FAIL\t<duration_ms>\t<message>`
public static class TestExecutor
{
    private const int StdoutFd = 1;
    private const int MaxEnvScan = 16384;
    private static bool _running = false;
    @extern("C") private unsafe static extern isize write(int fd, * const @readonly @expose_address byte buf, usize len);
    private static InlineBytes64 ZeroInline64() {
        return new InlineBytes64 {
            b00 = 0, b01 = 0, b02 = 0, b03 = 0, b04 = 0, b05 = 0, b06 = 0, b07 = 0, b08 = 0, b09 = 0, b10 = 0, b11 = 0, b12 = 0, b13 = 0, b14 = 0, b15 = 0, b16 = 0, b17 = 0, b18 = 0, b19 = 0, b20 = 0, b21 = 0, b22 = 0, b23 = 0, b24 = 0, b25 = 0, b26 = 0, b27 = 0, b28 = 0, b29 = 0, b30 = 0, b31 = 0, b32 = 0, b33 = 0, b34 = 0, b35 = 0, b36 = 0, b37 = 0, b38 = 0, b39 = 0, b40 = 0, b41 = 0, b42 = 0, b43 = 0, b44 = 0, b45 = 0, b46 = 0, b47 = 0, b48 = 0, b49 = 0, b50 = 0, b51 = 0, b52 = 0, b53 = 0, b54 = 0, b55 = 0, b56 = 0, b57 = 0, b58 = 0, b59 = 0, b60 = 0, b61 = 0, b62 = 0, b63 = 0,
        }
        ;
    }
    private unsafe static void WriteLiteral(string value) {
        let slice = StringRuntime.chic_rt_string_as_slice(& value);
        if (slice.ptr == null || slice.len == 0usize)
        {
            return;
        }
        let _ = write(StdoutFd, slice.ptr, slice.len);
    }
    private unsafe static void WriteBytes(* const @readonly @expose_address byte ptr, usize len) {
        if (ptr == null || len == 0usize)
        {
            return;
        }
        let _ = write(StdoutFd, ptr, len);
    }
    private unsafe static void WriteTestcasePrefix() {
        var bytes = ZeroInline64();
        bytes.b00 = (byte) 'C';
        bytes.b01 = (byte) 'H';
        bytes.b02 = (byte) 'I';
        bytes.b03 = (byte) 'C';
        bytes.b04 = (byte) '_';
        bytes.b05 = (byte) 'T';
        bytes.b06 = (byte) 'E';
        bytes.b07 = (byte) 'S';
        bytes.b08 = (byte) 'T';
        bytes.b09 = (byte) 'C';
        bytes.b10 = (byte) 'A';
        bytes.b11 = (byte) 'S';
        bytes.b12 = (byte) 'E';
        bytes.b13 = (byte) '\t';
        let ptr = (* const @readonly @expose_address byte) & bytes.b00;
        WriteBytes(ptr, 14usize);
    }
    private unsafe static void WriteTestcasePassSuffix() {
        var bytes = ZeroInline64();
        bytes.b00 = (byte) '\t';
        bytes.b01 = (byte) 'P';
        bytes.b02 = (byte) 'A';
        bytes.b03 = (byte) 'S';
        bytes.b04 = (byte) 'S';
        bytes.b05 = (byte) '\t';
        bytes.b06 = (byte) '0';
        bytes.b07 = (byte) '\t';
        bytes.b08 = (byte) '\n';
        let ptr = (* const @readonly @expose_address byte) & bytes.b00;
        WriteBytes(ptr, 9usize);
    }
    private unsafe static void WriteTestcaseFailSuffix() {
        var bytes = ZeroInline64();
        bytes.b00 = (byte) '\t';
        bytes.b01 = (byte) 'F';
        bytes.b02 = (byte) 'A';
        bytes.b03 = (byte) 'I';
        bytes.b04 = (byte) 'L';
        bytes.b05 = (byte) '\t';
        bytes.b06 = (byte) '0';
        bytes.b07 = (byte) '\t';
        bytes.b08 = (byte) '\n';
        let ptr = (* const @readonly @expose_address byte) & bytes.b00;
        WriteBytes(ptr, 9usize);
    }
    private unsafe static void WriteUsize(usize value) {
        var scratch = ZeroInline64();
        let basePtr = (* mut @expose_address byte) & scratch;
        let endPtr = NativePtr.OffsetMut(basePtr, 64isize);
        var cursor = endPtr;
        var remaining = value;
        if (remaining == 0usize)
        {
            cursor = NativePtr.OffsetMut(cursor, - 1isize);
            * cursor = 48u8;
        }
        else
        {
            while (remaining >0usize)
            {
                cursor = NativePtr.OffsetMut(cursor, - 1isize);
                let digit = (byte)(remaining % 10usize);
                * cursor = (byte)(digit + 48u8);
                remaining = remaining / 10usize;
            }
        }
        let totalLen = (usize)(NativePtr.ToIsize(endPtr) - NativePtr.ToIsize(cursor));
        let _ = write(StdoutFd, cursor, totalLen);
    }
    private unsafe static bool ByteStartsWith(* const @readonly @expose_address byte text, * const @readonly @expose_address byte prefix,
    usize prefixLen) {
        if (text == null || prefix == null)
        {
            return false;
        }
        var idx = 0usize;
        while (idx <prefixLen)
        {
            let a = NativePtr.ReadByteConst(NativePtr.OffsetConst(text, (isize) idx));
            let b = NativePtr.ReadByteConst(NativePtr.OffsetConst(prefix, (isize) idx));
            if (a != b)
            {
                return false;
            }
            idx += 1usize;
        }
        return true;
    }
    private unsafe static bool IsTruthy(* const @readonly @expose_address byte value) {
        if (value == null)
        {
            return false;
        }
        let first = * value;
        return first == (byte) '1' || first == (byte) 't' || first == (byte) 'T' || first == (byte) 'y' || first == (byte) 'Y';
    }
    private unsafe static bool CStrEqualsBytes(* const @readonly @expose_address byte text, * const @readonly @expose_address byte literal,
    usize literalLen) {
        if (!ByteStartsWith (text, literal, literalLen))
        {
            return false;
        }
        return NativePtr.ReadByteConst(NativePtr.OffsetConst(text, (isize) literalLen)) == 0u8;
    }
    private unsafe static * const @readonly @expose_address byte CStrListAt(* mut * mut char list, int index) {
        if (list == null || index <0)
        {
            return NativePtr.NullConst();
        }
        let elemSize = (isize) sizeof(* mut char);
        let base = (* const @readonly @expose_address byte) list;
        let slotAddr = NativePtr.OffsetConst(base, (isize) index * elemSize);
        let slot = (* const * const char) slotAddr;
        let value = * slot;
        return value == null ?NativePtr.NullConst() : (* const @readonly @expose_address byte) value;
    }
    private unsafe static * const @readonly @expose_address byte FindArgValueBytes(* mut * mut char argv, * const @readonly @expose_address byte prefix,
    usize prefixLen) {
        if (argv == null)
        {
            return NativePtr.NullConst();
        }
        var index = 0;
        while (index <MaxEnvScan)
        {
            let entryBytes = CStrListAt(argv, index);
            if (entryBytes == null)
            {
                break;
            }
            if (ByteStartsWith (entryBytes, prefix, prefixLen))
            {
                return NativePtr.OffsetConst(entryBytes, (isize) prefixLen);
            }
            index += 1;
        }
        return NativePtr.NullConst();
    }
    private unsafe static bool HasArgBytes(* mut * mut char argv, * const @readonly @expose_address byte literal, usize literalLen) {
        if (argv == null)
        {
            return false;
        }
        var index = 0;
        while (index <MaxEnvScan)
        {
            let entryBytes = CStrListAt(argv, index);
            if (entryBytes == null)
            {
                break;
            }
            if (CStrEqualsBytes (entryBytes, literal, literalLen))
            {
                return true;
            }
            index += 1;
        }
        return false;
    }
    private unsafe static bool AllowsIndex(usize index, * const @readonly @expose_address byte selection) {
        if (selection == null)
        {
            return true;
        }
        let first = NativePtr.ReadByteConst(selection);
        if (first == 0u8)
        {
            return true;
        }
        var cursor = selection;
        while (cursor != null)
        {
            let current = NativePtr.ReadByteConst(cursor);
            if (current == 0u8)
            {
                break;
            }
            if (current <48u8 || current >57u8)
            {
                cursor = NativePtr.OffsetConst(cursor, 1isize);
                continue;
            }
            var value = 0usize;
            while (cursor != null)
            {
                let digit = NativePtr.ReadByteConst(cursor);
                if (digit <48u8 || digit >57u8)
                {
                    break;
                }
                value = (value * 10usize) + (usize)(digit - 48u8);
                cursor = NativePtr.OffsetConst(cursor, 1isize);
            }
            if (value == index)
            {
                return true;
            }
        }
        return false;
    }
    private unsafe static bool HasPendingException() {
        return PendingExceptionRuntime.chic_rt_has_pending_exception() != 0;
    }
    private unsafe static bool RunSyncTestcase(* const @readonly @expose_address byte fnPtr) {
        if (fnPtr == null)
        {
            return false;
        }
        PendingExceptionRuntime.chic_rt_clear_pending_exception();
        let func = (fn @extern("C")() -> bool) fnPtr;
        let passed = func();
        if (HasPendingException ())
        {
            PendingExceptionRuntime.chic_rt_clear_pending_exception();
            return false;
        }
        return passed;
    }
    private unsafe static bool RunAsyncTestcase(* const @readonly @expose_address byte fnPtr) {
        if (fnPtr == null)
        {
            return false;
        }
        PendingExceptionRuntime.chic_rt_clear_pending_exception();
        let func = (fn @extern("C")() -> AsyncTaskBool) fnPtr;
        var task = func();
        if (HasPendingException ())
        {
            PendingExceptionRuntime.chic_rt_clear_pending_exception();
            return false;
        }
        chic_rt_async_block_on((* mut NativeFutureHeader) & task);
        let completed = (task.BaseHeader.Flags & AsyncFlags.Completed) != 0u;
        return completed && task.Result != 0u8;
    }
    private unsafe static void ReportResult(usize index, bool passed) {
        if (passed)
        {
            WriteTestcasePrefix();
            WriteUsize(index);
            WriteTestcasePassSuffix();
        }
        else
        {
            WriteTestcasePrefix();
            WriteUsize(index);
            WriteTestcaseFailSuffix();
        }
    }
    @extern("C") @export("chic_rt_test_executor_run_all") public unsafe static int chic_rt_test_executor_run_all() {
        if (_running)
        {
            return 0;
        }
        _running = true;
        let argv = StartupState.chic_rt_startup_raw_argv();
        var indexesPrefix = ZeroInline64();
        indexesPrefix.b00 = (byte) '-';
        indexesPrefix.b01 = (byte) '-';
        indexesPrefix.b02 = (byte) 'c';
        indexesPrefix.b03 = (byte) 'h';
        indexesPrefix.b04 = (byte) 'i';
        indexesPrefix.b05 = (byte) 'c';
        indexesPrefix.b06 = (byte) '-';
        indexesPrefix.b07 = (byte) 't';
        indexesPrefix.b08 = (byte) 'e';
        indexesPrefix.b09 = (byte) 's';
        indexesPrefix.b10 = (byte) 't';
        indexesPrefix.b11 = (byte) '-';
        indexesPrefix.b12 = (byte) 'i';
        indexesPrefix.b13 = (byte) 'n';
        indexesPrefix.b14 = (byte) 'd';
        indexesPrefix.b15 = (byte) 'e';
        indexesPrefix.b16 = (byte) 'x';
        indexesPrefix.b17 = (byte) 'e';
        indexesPrefix.b18 = (byte) 's';
        indexesPrefix.b19 = (byte) '=';
        let indexesPrefixPtr = (* const @readonly @expose_address byte) & indexesPrefix.b00;
        let selection = FindArgValueBytes(argv, indexesPrefixPtr, 20usize);
        var failFastBytes = ZeroInline64();
        failFastBytes.b00 = (byte) '-';
        failFastBytes.b01 = (byte) '-';
        failFastBytes.b02 = (byte) 'c';
        failFastBytes.b03 = (byte) 'h';
        failFastBytes.b04 = (byte) 'i';
        failFastBytes.b05 = (byte) 'c';
        failFastBytes.b06 = (byte) '-';
        failFastBytes.b07 = (byte) 't';
        failFastBytes.b08 = (byte) 'e';
        failFastBytes.b09 = (byte) 's';
        failFastBytes.b10 = (byte) 't';
        failFastBytes.b11 = (byte) '-';
        failFastBytes.b12 = (byte) 'f';
        failFastBytes.b13 = (byte) 'a';
        failFastBytes.b14 = (byte) 'i';
        failFastBytes.b15 = (byte) 'l';
        failFastBytes.b16 = (byte) '-';
        failFastBytes.b17 = (byte) 'f';
        failFastBytes.b18 = (byte) 'a';
        failFastBytes.b19 = (byte) 's';
        failFastBytes.b20 = (byte) 't';
        let failFastPtr = (* const @readonly @expose_address byte) & failFastBytes.b00;
        let failFast = HasArgBytes(argv, failFastPtr, 21usize);
        let trace = false;
        let descriptor = StartupState.chic_rt_startup_descriptor_snapshot();
        let testCount = descriptor.Tests.Len;
        if (descriptor.Tests.Cases == null || testCount == 0usize)
        {
            return 0;
        }
        var sawFailure = false;
        var index = 0usize;
        while (index <testCount)
        {
            if (!AllowsIndex (index, selection))
            {
                index += 1usize;
                continue;
            }
            var test = new TestCaseDescriptorSnapshot {
                Function = (* const @readonly @expose_address byte) NativePtr.NullConst(), NamePtr = (* const @readonly @expose_address byte) NativePtr.NullConst(), NameLen = 0usize, Flags = 0u, Reserved = 0u,
            }
            ;
            StartupState.chic_rt_startup_test_descriptor(& test, index);
            if (trace)
            {
                WriteLiteral("CHIC_TEST_BEGIN\t");
                WriteUsize(index);
                WriteLiteral("\t");
                WriteBytes(test.NamePtr, test.NameLen);
                WriteLiteral("\n");
            }
            let isAsync = (test.Flags & StartupConstants.TestAsync) != 0u;
            let passed = isAsync ?RunAsyncTestcase(test.Function) : RunSyncTestcase(test.Function);
            if (!passed)
            {
                sawFailure = true;
            }
            ReportResult(index, passed);
            if (failFast && sawFailure)
            {
                break;
            }
            index += 1usize;
        }
        let code = sawFailure ?1 : 0;
        return code;
    }
}
