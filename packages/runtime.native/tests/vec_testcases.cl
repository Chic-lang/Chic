namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;

testcase Given_vec_push_pop_insert_remove_When_executed_Then_vec_push_pop_insert_remove()
{
    unsafe {
        let elemSize = (usize) __sizeof<int>();
        let elemAlign = (usize) __alignof<int>();
        var vec = VecRuntime.chic_rt_vec_new(elemSize, elemAlign, HashMapTestSupport.DropNoop);
        var value1 = 1;
        var value2 = 2;
        var value3 = 3;
        var * const @readonly @expose_address byte raw1 = & value1;
        var * const @readonly @expose_address byte raw2 = & value2;
        var * const @readonly @expose_address byte raw3 = & value3;
        var input1 = new ValueConstPtr {
            Pointer = raw1, Size = elemSize, Alignment = elemAlign
        }
        ;
        var input2 = new ValueConstPtr {
            Pointer = raw2, Size = elemSize, Alignment = elemAlign
        }
        ;
        var input3 = new ValueConstPtr {
            Pointer = raw3, Size = elemSize, Alignment = elemAlign
        }
        ;
        let push1 = VecRuntime.chic_rt_vec_push(& vec, & input1);
        let push2 = VecRuntime.chic_rt_vec_push(& vec, & input2);
        let push3 = VecRuntime.chic_rt_vec_push(& vec, & input3);
        let len1 = VecRuntime.chic_rt_vec_len(& vec);
        var insertValue = 9;
        var * const @readonly @expose_address byte insertRaw = & insertValue;
        var insertPtr = new ValueConstPtr {
            Pointer = insertRaw, Size = elemSize, Alignment = elemAlign
        }
        ;
        let insertStatus = VecRuntime.chic_rt_vec_insert(& vec, 1usize, & insertPtr);
        let len2 = VecRuntime.chic_rt_vec_len(& vec);
        var removedValue = 0;
        var * mut @expose_address byte removedRaw = & removedValue;
        var removedPtr = new ValueMutPtr {
            Pointer = removedRaw, Size = elemSize, Alignment = elemAlign
        }
        ;
        let removeStatus = VecRuntime.chic_rt_vec_remove(& vec, 2usize, & removedPtr);
        let len3 = VecRuntime.chic_rt_vec_len(& vec);
        let popStatus = VecRuntime.chic_rt_vec_pop(& vec, & removedPtr);
        let len4 = VecRuntime.chic_rt_vec_len(& vec);
        let clearStatus = VecRuntime.chic_rt_vec_clear(& vec);
        let empty = VecRuntime.chic_rt_vec_is_empty(& vec);
        let ok = push1 == 0
            && push2 == 0
            && push3 == 0
            && len1 == 3usize
            && insertStatus == 0
            && len2 == 4usize
            && removeStatus == 0
            && len3 == 3usize
            && popStatus == 0
            && len4 == 2usize
            && clearStatus == 0
            && empty == 1;
        Assert.That(ok).IsTrue();
        VecRuntime.chic_rt_vec_drop(& vec);
    }
}

testcase Given_vec_iteration_and_views_When_executed_Then_vec_iteration_and_views()
{
    unsafe {
        let elemSize = (usize) __sizeof<int>();
        let elemAlign = (usize) __alignof<int>();
        var vec = VecRuntime.chic_rt_vec_with_capacity(elemSize, elemAlign, 4usize, HashMapTestSupport.DropNoop);
        var value = 0;
        var pushOk = true;
        var idx = 0;
        while (idx < 3)
        {
            value = idx + 5;
            var * const @readonly @expose_address byte raw = & value;
            var input = new ValueConstPtr {
                Pointer = raw, Size = elemSize, Alignment = elemAlign
            }
            ;
            if (VecRuntime.chic_rt_vec_push(& vec, & input) != 0)
            {
                pushOk = false;
            }
            idx = idx + 1;
        }
        var vecView = new ChicVecView {
            data = NativePtr.NullConst(), len = 0, elem_size = 0, elem_align = 0,
        }
        ;
        let viewStatus = VecRuntime.chic_rt_vec_view(& vec, & vecView);
        var iter = VecRuntime.chic_rt_vec_iter(& vec);
        var nextValue = 0;
        var * mut @expose_address byte nextRaw = & nextValue;
        var outPtr = new ValueMutPtr {
            Pointer = nextRaw, Size = elemSize, Alignment = elemAlign
        }
        ;
        let firstStatus = VecRuntime.chic_rt_vec_iter_next(& iter, & outPtr);
        let firstValue = nextValue;
        let secondStatus = VecRuntime.chic_rt_vec_iter_next(& iter, & outPtr);
        let secondValue = nextValue;
        let thirdStatus = VecRuntime.chic_rt_vec_iter_next(& iter, & outPtr);
        let thirdValue = nextValue;
        let doneStatus = VecRuntime.chic_rt_vec_iter_next(& iter, & outPtr);
        let ok = viewStatus == 0
            && pushOk
            && vecView.len == 3usize
            && firstStatus == 0
            && firstValue == 5
            && secondStatus == 0
            && secondValue == 6
            && thirdStatus == 0
            && thirdValue == 7
            && doneStatus == 6;
        Assert.That(ok).IsTrue();
        VecRuntime.chic_rt_vec_drop(& vec);
    }
}

testcase Given_vec_layout_and_inline_helpers_When_executed_Then_vec_layout_and_inline_helpers()
{
    unsafe {
        let elemSize = (usize) __sizeof<byte>();
        var vec = VecRuntime.chic_rt_vec_new(elemSize, 1usize, HashMapTestSupport.DropNoop);
        let info = VecRuntime.chic_rt_vec_layout_debug();
        let inlineCap = VecRuntime.chic_rt_vec_inline_capacity(& vec);
        let inlinePtr = VecRuntime.chic_rt_vec_inline_ptr(& vec);
        VecRuntime.chic_rt_vec_mark_inline(& vec, true);
        let usesInline = VecRuntime.chic_rt_vec_uses_inline(& vec);
        let dataPtr = VecRuntime.chic_rt_vec_get_ptr(& vec);
        VecRuntime.chic_rt_vec_set_ptr(& vec, & dataPtr);
        VecRuntime.chic_rt_vec_set_cap(& vec, 2usize);
        VecRuntime.chic_rt_vec_set_elem_size(& vec, elemSize);
        VecRuntime.chic_rt_vec_set_elem_align(& vec, 1usize);
        let dropPtr = VecRuntime.chic_rt_vec_get_drop(& vec);
        VecRuntime.chic_rt_vec_set_drop(& vec, dropPtr);
        let ok = info.size > 0usize
            && info.offset_inline_storage > 0usize
            && inlineCap > 0usize
            && !NativePtr.IsNull(inlinePtr.Pointer)
            && usesInline == 1;
        Assert.That(ok).IsTrue();
        VecRuntime.chic_rt_vec_drop(& vec);
    }
}

testcase Given_vec_clone_reserve_and_into_array_When_executed_Then_vec_clone_reserve_and_into_array()
{
    unsafe {
        let elemSize = (usize) __sizeof<int>();
        let elemAlign = (usize) __alignof<int>();
        var vec = VecRuntime.chic_rt_vec_with_capacity(elemSize, elemAlign, 2usize, HashMapTestSupport.DropNoop);
        var value = 1;
        var * const @readonly @expose_address byte raw = & value;
        var input = new ValueConstPtr {
            Pointer = raw, Size = elemSize, Alignment = elemAlign
        }
        ;
        let _ = VecRuntime.chic_rt_vec_push(& vec, & input);
        value = 2;
        let _ = VecRuntime.chic_rt_vec_push(& vec, & input);
        let reserveStatus = VecRuntime.chic_rt_vec_reserve(& vec, 4usize);
        let shrinkStatus = VecRuntime.chic_rt_vec_shrink_to_fit(& vec);

        var clone = VecRuntime.chic_rt_vec_new(elemSize, elemAlign, HashMapTestSupport.DropNoop);
        let cloneStatus = VecRuntime.chic_rt_vec_clone(& clone, & vec);
        let cloneLen = VecRuntime.chic_rt_vec_len(& clone);

        var copied = VecRuntime.chic_rt_vec_new(elemSize, elemAlign, HashMapTestSupport.DropNoop);
        let copyStatus = VecRuntime.chic_rt_vec_copy_to_array(& copied, & vec);
        let copyLen = VecRuntime.chic_rt_vec_len(& copied);

        var moved = VecRuntime.chic_rt_vec_new(elemSize, elemAlign, HashMapTestSupport.DropNoop);
        let moveStatus = VecRuntime.chic_rt_vec_into_array(& moved, & vec);
        let originalLen = VecRuntime.chic_rt_vec_len(& vec);
        let movedLen = VecRuntime.chic_rt_vec_len(& moved);
        let ok = reserveStatus == 0
            && shrinkStatus == 0
            && cloneStatus == 0
            && cloneLen == 2usize
            && copyStatus == 0
            && copyLen == 2usize
            && moveStatus == 0
            && originalLen == 0usize
            && movedLen == 2usize;
        Assert.That(ok).IsTrue();

        VecRuntime.chic_rt_vec_drop(& clone);
        VecRuntime.chic_rt_vec_drop(& copied);
        VecRuntime.chic_rt_vec_drop(& moved);
        VecRuntime.chic_rt_vec_drop(& vec);
    }
}

testcase Given_vec_ptr_access_and_errors_When_executed_Then_vec_ptr_access_and_errors()
{
    unsafe {
        let reserveStatus = VecRuntime.chic_rt_vec_reserve((* mut ChicVec) NativePtr.NullMut(), 1usize);
        var vec = VecRuntime.chic_rt_vec_new((usize) __sizeof<int>(), (usize) __alignof<int>(), HashMapTestSupport.DropNoop);
        var value = 5;
        var * const @readonly @expose_address byte raw = & value;
        var input = new ValueConstPtr {
            Pointer = raw, Size = (usize) __sizeof<int>(), Alignment = (usize) __alignof<int>()
        }
        ;
        let _ = VecRuntime.chic_rt_vec_push(& vec, & input);
        let data = VecRuntime.chic_rt_vec_data(& vec);
        let dataMut = VecRuntime.chic_rt_vec_data_mut(& vec);
        let ptrAt = VecRuntime.chic_rt_vec_ptr_at(& vec, 0usize);
        let ptrOob = VecRuntime.chic_rt_vec_ptr_at(& vec, 3usize);
        let ok = reserveStatus == 2
            && !NativePtr.IsNullConst(data.Pointer)
            && !NativePtr.IsNull(dataMut.Pointer)
            && !NativePtr.IsNull(ptrAt.Pointer)
            && NativePtr.IsNull(ptrOob.Pointer);
        Assert.That(ok).IsTrue();
        VecRuntime.chic_rt_vec_drop(& vec);
    }
}

testcase Given_vec_error_paths_and_zero_sized_When_executed_Then_vec_error_paths_and_zero_sized()
{
    unsafe {
        var zeroVec = VecRuntime.chic_rt_vec_new(0usize, 1usize, HashMapTestSupport.DropNoop);
        var zeroValue = new ValueConstPtr {
            Pointer = NativePtr.NullConst(), Size = 0usize, Alignment = 1usize
        }
        ;
        let zeroPush = VecRuntime.chic_rt_vec_push(& zeroVec, & zeroValue);
        let zeroLen = VecRuntime.chic_rt_vec_len(& zeroVec);
        VecRuntime.chic_rt_vec_drop(& zeroVec);

        var vec = VecRuntime.chic_rt_vec_new((usize) __sizeof<int>(), (usize) __alignof<int>(), HashMapTestSupport.DropNoop);
        var outValue = 0;
        var outPtr = new ValueMutPtr {
            Pointer = & outValue, Size = (usize) __sizeof<int>(), Alignment = (usize) __alignof<int>()
        }
        ;
        let popEmpty = VecRuntime.chic_rt_vec_pop(& vec, & outPtr);
        let badInsert = VecRuntime.chic_rt_vec_insert(& vec, 0usize, (* const ValueConstPtr) NativePtr.NullConst());
        let badRemove = VecRuntime.chic_rt_vec_remove(& vec, 1usize, & outPtr);
        let badSwap = VecRuntime.chic_rt_vec_swap_remove(& vec, 0usize, & outPtr);

        let iterStatus = VecRuntime.chic_rt_vec_iter_next((* mut ChicVecIter) NativePtr.NullMut(), & outPtr);
        let viewStatus = VecRuntime.chic_rt_vec_view(& vec, (* mut ChicVecView) NativePtr.NullMut());
        let ok = zeroPush == 0
            && zeroLen == 1usize
            && popEmpty == (int) VecError.OutOfBounds
            && badInsert == (int) VecError.InvalidPointer
            && badRemove == (int) VecError.OutOfBounds
            && badSwap == (int) VecError.OutOfBounds
            && iterStatus == (int) VecError.InvalidPointer
            && viewStatus == (int) VecError.InvalidPointer;
        Assert.That(ok).IsTrue();
        VecRuntime.chic_rt_vec_drop(& vec);
    }
}

testcase Given_vec_marked_heap_with_zero_cap_When_reserving_Then_activates_inline_storage()
{
    unsafe {
        let elemSize = (usize) __sizeof<int>();
        let elemAlign = (usize) __alignof<int>();
        var vec = VecRuntime.chic_rt_vec_new(elemSize, elemAlign, HashMapTestSupport.DropNoop);
        VecRuntime.chic_rt_vec_mark_inline(& vec, false);
        VecRuntime.chic_rt_vec_set_cap(& vec, 0usize);
        let status = VecRuntime.chic_rt_vec_reserve(& vec, 1usize);
        let usesInline = VecRuntime.chic_rt_vec_uses_inline(& vec);
        let ok = status == 0 && usesInline == 1;
        Assert.That(ok).IsTrue();
        VecRuntime.chic_rt_vec_drop(& vec);
    }
}

testcase Given_vec_pointer_inside_struct_When_reserving_Then_normalizes_inline_state()
{
    unsafe {
        let elemSize = (usize) __sizeof<int>();
        let elemAlign = (usize) __alignof<int>();
        var vec = VecRuntime.chic_rt_vec_new(elemSize, elemAlign, HashMapTestSupport.DropNoop);
        VecRuntime.chic_rt_vec_mark_inline(& vec, false);
        VecRuntime.chic_rt_vec_set_cap(& vec, 1usize);
        var * mut @expose_address byte basePtr = (* mut @expose_address byte) & vec;
        var fakePtr = new ValueMutPtr {
            Pointer = basePtr, Size = elemSize, Alignment = elemAlign
        }
        ;
        VecRuntime.chic_rt_vec_set_ptr(& vec, & fakePtr);
        let status = VecRuntime.chic_rt_vec_reserve(& vec, 1usize);
        let usesInline = VecRuntime.chic_rt_vec_uses_inline(& vec);
        let ok = status == 0 && usesInline == 1;
        Assert.That(ok).IsTrue();
        VecRuntime.chic_rt_vec_drop(& vec);
    }
}

testcase Given_vec_swap_truncate_set_len_and_iter_ptr_When_executed_Then_vec_swap_truncate_set_len_and_iter_ptr()
{
    unsafe {
        let elemSize = (usize) __sizeof<int>();
        let elemAlign = (usize) __alignof<int>();
        let regionHandle = chic_rt_region_enter(11ul);
        var vecRegion = VecRuntime.chic_rt_vec_new_in_region(elemSize, elemAlign, HashMapTestSupport.DropNoop, regionHandle);
        var vec = VecRuntime.chic_rt_vec_with_capacity_in_region(elemSize, elemAlign, 4usize, HashMapTestSupport.DropNoop,
        regionHandle);
        var ok = VecRuntime.chic_rt_vec_is_empty(& vecRegion) == 1;
        ok = ok && VecRuntime.chic_rt_vec_capacity(& vec) >= 4usize;

        var value = 0;
        var idx = 0;
        while (idx < 3)
        {
            value = idx + 1;
            var * const @readonly @expose_address byte raw = & value;
            var input = new ValueConstPtr {
                Pointer = raw, Size = elemSize, Alignment = elemAlign
            }
            ;
            let pushStatus = VecRuntime.chic_rt_vec_push(& vec, & input);
            ok = ok && pushStatus == 0;
            idx = idx + 1;
        }

        var removedValue = 0;
        var removedPtr = new ValueMutPtr {
            Pointer = & removedValue, Size = elemSize, Alignment = elemAlign
        }
        ;
        let swapStatus = VecRuntime.chic_rt_vec_swap_remove(& vec, 0usize, & removedPtr);
        ok = ok && swapStatus == (int) VecError.Success;

        let setStatus = VecRuntime.chic_rt_vec_set_len(& vec, 1usize);
        ok = ok && setStatus == (int) VecError.Success;

        var iter = VecRuntime.chic_rt_vec_iter(& vec);
        let firstPtr = VecRuntime.chic_rt_vec_iter_next_ptr(& iter);
        let secondPtr = VecRuntime.chic_rt_vec_iter_next_ptr(& iter);
        ok = ok && !NativePtr.IsNullConst(firstPtr.Pointer) && NativePtr.IsNullConst(secondPtr.Pointer);

        let truncStatus = VecRuntime.chic_rt_vec_truncate(& vec, 0usize);
        ok = ok && truncStatus == (int) VecError.Success;
        ok = ok && VecRuntime.chic_rt_vec_elem_size(& vec) == elemSize;
        ok = ok && VecRuntime.chic_rt_vec_elem_align(& vec) == elemAlign;

        VecRuntime.chic_rt_vec_drop(& vecRegion);
        VecRuntime.chic_rt_vec_drop(& vec);
        chic_rt_region_exit(regionHandle);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_vec_region_allocation_and_iter_ptr_When_executed_Then_vec_region_allocation_and_iter_ptr()
{
    unsafe {
        let regionHandle = chic_rt_region_enter(4ul);
        let elemSize = 1usize;
        var vec = VecRuntime.chic_rt_vec_with_capacity_in_region(elemSize, 1usize, 4usize, HashMapTestSupport.DropNoop, regionHandle);
        var value = 7u8;
        var handle = new ValueConstPtr {
            Pointer = & value, Size = elemSize, Alignment = 1usize
        }
        ;
        let _ = VecRuntime.chic_rt_vec_push(& vec, & handle);
        let _ = VecRuntime.chic_rt_vec_push(& vec, & handle);
        var iter = VecRuntime.chic_rt_vec_iter(& vec);
        let entry = VecRuntime.chic_rt_vec_iter_next_ptr(& iter);
        Assert.That(!NativePtr.IsNullConst(entry.Pointer)).IsTrue();
        VecRuntime.chic_rt_vec_drop(& vec);
        chic_rt_region_exit(regionHandle);
    }
}

testcase Given_vec_truncate_set_len_capacity_and_swap_remove_When_executed_Then_vec_truncate_set_len_capacity_and_swap_remove()
{
    unsafe {
        let elemSize = (usize) __sizeof<int>();
        let elemAlign = (usize) __alignof<int>();
        var vec = VecRuntime.chic_rt_vec_with_capacity(elemSize, elemAlign, 4usize, HashMapTestSupport.DropNoop);
        var value = 21;
        var input = new ValueConstPtr {
            Pointer = & value, Size = elemSize, Alignment = elemAlign
        }
        ;
        let _ = VecRuntime.chic_rt_vec_push(& vec, & input);
        value = 22;
        let _ = VecRuntime.chic_rt_vec_push(& vec, & input);
        let cap = VecRuntime.chic_rt_vec_capacity(& vec);
        let elemSizeActual = VecRuntime.chic_rt_vec_elem_size(& vec);
        let elemAlignActual = VecRuntime.chic_rt_vec_elem_align(& vec);

        let setLen1 = VecRuntime.chic_rt_vec_set_len(& vec, 1usize);
        let len1 = VecRuntime.chic_rt_vec_len(& vec);
        let truncateStatus = VecRuntime.chic_rt_vec_truncate(& vec, 3usize);
        let setLen2 = VecRuntime.chic_rt_vec_set_len(& vec, 2usize);

        var outValue = 0;
        var outPtr = new ValueMutPtr {
            Pointer = & outValue, Size = elemSize, Alignment = elemAlign
        }
        ;
        let swapStatus = VecRuntime.chic_rt_vec_swap_remove(& vec, 0usize, & outPtr);
        let len2 = VecRuntime.chic_rt_vec_len(& vec);

        var clone = VecRuntime.chic_rt_vec_new(elemSize, elemAlign, HashMapTestSupport.DropNoop);
        let copyStatus = VecRuntime.chic_rt_array_copy_to_vec(& clone, & vec);
        let cloneLen = VecRuntime.chic_rt_vec_len(& clone);
        let ok = cap >= 2usize
            && elemSizeActual == elemSize
            && elemAlignActual == elemAlign
            && setLen1 == 0
            && len1 == 1usize
            && truncateStatus == 0
            && setLen2 == 0
            && swapStatus == 0
            && len2 == 1usize
            && copyStatus == 0
            && cloneLen == 1usize;
        Assert.That(ok).IsTrue();
        VecRuntime.chic_rt_vec_drop(& clone);
        VecRuntime.chic_rt_vec_drop(& vec);
    }
}

testcase Given_vec_growth_and_truncate_When_executed_Then_vec_growth_and_truncate()
{
    unsafe {
        let elemSize = (usize) __sizeof<int>();
        let elemAlign = (usize) __alignof<int>();
        var vec = VecRuntime.chic_rt_vec_new(elemSize, elemAlign, HashMapTestSupport.DropNoop);
        let inlineCap = VecRuntime.chic_rt_vec_inline_capacity(& vec);
        var value = 0;
        var input = new ValueConstPtr {
            Pointer = & value, Size = elemSize, Alignment = elemAlign
        }
        ;
        var idx = 0usize;
        while (idx < inlineCap + 3usize)
        {
            value = (int) idx;
            let _ = VecRuntime.chic_rt_vec_push(& vec, & input);
            idx += 1usize;
        }
        let len = VecRuntime.chic_rt_vec_len(& vec);
        let cap = VecRuntime.chic_rt_vec_capacity(& vec);
        let truncStatus = VecRuntime.chic_rt_vec_truncate(& vec, inlineCap);
        let lenAfter = VecRuntime.chic_rt_vec_len(& vec);
        let ok = len == inlineCap + 3usize
            && cap >= inlineCap + 3usize
            && truncStatus == 0
            && lenAfter == inlineCap;
        Assert.That(ok).IsTrue();
        VecRuntime.chic_rt_vec_drop(& vec);
    }
}

testcase Given_vec_setters_and_failure_paths_When_executed_Then_vec_setters_and_failure_paths()
{
    unsafe {
        let elemSize = (usize) __sizeof<int>();
        let elemAlign = (usize) __alignof<int>();
        var vec = VecRuntime.chic_rt_vec_new(elemSize, elemAlign, HashMapTestSupport.DropNoop);
        let empty = VecRuntime.chic_rt_vec_is_empty(& vec);
        let layout = VecRuntime.chic_rt_vec_layout_debug();

        VecRuntime.chic_rt_vec_mark_inline(& vec, 1);
        let usesInline = VecRuntime.chic_rt_vec_uses_inline(& vec);
        var inlinePtr = VecRuntime.chic_rt_vec_inline_ptr(& vec);
        VecRuntime.chic_rt_vec_set_ptr(& vec, & inlinePtr);
        let inlineCap = VecRuntime.chic_rt_vec_inline_capacity(& vec);
        VecRuntime.chic_rt_vec_set_cap(& vec, inlineCap);
        VecRuntime.chic_rt_vec_set_elem_size(& vec, elemSize);
        VecRuntime.chic_rt_vec_set_elem_align(& vec, elemAlign);
        let elemSizeActual = VecRuntime.chic_rt_vec_elem_size(& vec);
        let elemAlignActual = VecRuntime.chic_rt_vec_elem_align(& vec);

        let invalidPush = VecRuntime.chic_rt_vec_push((* mut ChicVec) NativePtr.NullMut(), & inlinePtr);
        let invalidPop = VecRuntime.chic_rt_vec_pop((* mut ChicVec) NativePtr.NullMut(), & inlinePtr);

        var iter = VecRuntime.chic_rt_vec_iter(& vec);
        var outValue = 0;
        var outPtr = new ValueMutPtr {
            Pointer = & outValue, Size = elemSize, Alignment = elemAlign
        }
        ;
        let iterStatus = VecRuntime.chic_rt_vec_iter_next(& iter, & outPtr);

        NativeAlloc.TestFailAllocAfter(0);
        let reserveFail = VecRuntime.chic_rt_vec_reserve(& vec, inlineCap + 1usize);
        NativeAlloc.TestReset();
        let ok = empty == 1
            && layout.size > 0usize
            && usesInline == 1
            && elemSizeActual == elemSize
            && elemAlignActual == elemAlign
            && invalidPush == (int) VecError.InvalidPointer
            && invalidPop == (int) VecError.InvalidPointer
            && iterStatus == (int) VecError.IterationComplete
            && reserveFail == (int) VecError.AllocationFailed;
        Assert.That(ok).IsTrue();
        VecRuntime.chic_rt_vec_drop(& vec);
    }
}

testcase Given_vec_new_in_region_and_array_into_vec_When_executed_Then_vec_new_in_region_and_array_into_vec()
{
    unsafe {
        let elemSize = (usize) __sizeof<int>();
        let elemAlign = (usize) __alignof<int>();
        let regionHandle = chic_rt_region_enter(99ul);
        var vec = VecRuntime.chic_rt_vec_new_in_region(elemSize, elemAlign, HashMapTestSupport.DropNoop, regionHandle);
        var value = 17;
        var input = new ValueConstPtr {
            Pointer = & value, Size = elemSize, Alignment = elemAlign
        }
        ;
        let pushStatus = VecRuntime.chic_rt_vec_push(& vec, & input);

        var moved = VecRuntime.chic_rt_vec_new(elemSize, elemAlign, HashMapTestSupport.DropNoop);
        let moveStatus = VecRuntime.chic_rt_array_into_vec(& moved, & vec);
        let movedLen = VecRuntime.chic_rt_vec_len(& moved);
        let originalLen = VecRuntime.chic_rt_vec_len(& vec);

        VecRuntime.chic_rt_vec_drop(& moved);
        VecRuntime.chic_rt_vec_drop(& vec);
        chic_rt_region_exit(regionHandle);

        let ok = pushStatus == 0
            && moveStatus == 0
            && movedLen == 1usize
            && originalLen == 0usize;
        Assert.That(ok).IsTrue();
    }
}
