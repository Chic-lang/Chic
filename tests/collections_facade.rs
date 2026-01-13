use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

mod common;
use common::write_source;

#[test]
fn std_collections_vec_intrinsics_available() {
    let dir = tempfile::tempdir().expect("temp dir");
    let main_src = dir.path().join("collections_usage.cl");

    write_source(
        &main_src,
        r#"
import Std;
import Std.Collections;
import Std.Memory;
import Std.Core;
import Std.Numeric;

namespace CollectionsFacade;

public int Main()
{
    var vec = Vec.New<int>();
    if (!Vec.IsEmpty(in vec)) { return 1; }

    var length = Vec.Len(in vec);
    if (length != 0) { return 2; }

    var capacity = Vec.Capacity(in vec);
    if (capacity == 0) { return 3; }

    var vec_view = Vec.View(in vec);
    if (!VecView.IsEmpty(in vec_view)) { return 4; }

    var iterator = Vec.Iter(in vec);
    var nextPtr = VecIntrinsics.chic_rt_vec_iter_next_ptr(ref iterator);
    if (nextPtr.Pointer != Pointer.NullConst<byte>()) { return 9; }
    var dataPtr = Vec.Data(in vec);
    if (dataPtr.Pointer != Pointer.NullConst<byte>()) { return 10; }

    var copyArray = CoreIntrinsics.DefaultValue<ArrayPtr>();
    var copyStatus = Vec.ToArray(in vec, out copyArray);
    if (copyStatus != VecError.Success) { return 5; }

    var movedArray = CoreIntrinsics.DefaultValue<ArrayPtr>();
    var moveStatus = Vec.IntoArray(ref vec, out movedArray);
    if (moveStatus != VecError.Success) { return 6; }
    if (Vec.Len(in vec) != 0) { return 7; }

    var vecFromCopy = CoreIntrinsics.DefaultValue<VecPtr>();
    var toVecStatus = Array.ToVec(in copyArray, out vecFromCopy);
    if (toVecStatus != VecError.Success) { return 8; }

    var vecFromMove = CoreIntrinsics.DefaultValue<VecPtr>();
    var intoVecStatus = Array.IntoVec(ref movedArray, out vecFromMove);
    if (intoVecStatus != VecError.Success) { return 9; }
    if (Array.Len(in movedArray) != 0) { return 10; }

    if (vec.DropCallback != 0) { return 12; }

    var cleanupVec = CoreIntrinsics.DefaultValue<VecPtr>();
    var cleanupStatus = Array.IntoVec(ref copyArray, out cleanupVec);
    if (cleanupStatus != VecError.Success) { return 11; }

    VecIntrinsics.chic_rt_vec_drop(ref vec);
    VecIntrinsics.chic_rt_vec_drop(ref vecFromCopy);
    VecIntrinsics.chic_rt_vec_drop(ref vecFromMove);
    VecIntrinsics.chic_rt_vec_drop(ref cleanupVec);

    var array = CoreIntrinsics.DefaultValue<ArrayPtr>();
    array.Pointer = UIntPtr.Zero.AsPointer<byte>();
    array.Length = 0;
    array.Capacity = 0;
    array.ElementSize = sizeof(int);
    array.ElementAlignment = alignof(int);
    array.DropCallback = 0;

    var arrayLen = Array.Len(in array);
    if (arrayLen != 0) { return 5; }

    var pushStatus = Vec.Push(ref vec, capacity);
    if (pushStatus != VecError.Success) { return 13; }

    var popped = 0;
    var popStatus = Vec.Pop(ref vec, out popped);
    if (popStatus != VecError.Success || popped != capacity) { return 14; }

    var slot = MaybeUninit<int>.Uninit();
    slot.Write(42);
    var pushInitStatus = Vec.PushInitialized(ref vec, ref slot);
    if (pushInitStatus != VecError.Success) { return 15; }
    if (slot.IsInitialized) { return 16; }

    var popSlot = MaybeUninit<int>.Uninit();
    var popIntoStatus = Vec.PopInto(ref vec, ref popSlot);
    if (popIntoStatus != VecError.Success) { return 17; }
    var emplaced = popSlot.AssumeInit();
    if (emplaced != 42) { return 18; }

    var maybe = MaybeUninit<int>.Uninit();
    if (maybe.IsInitialized) { return 19; }
    maybe.Write(7);
    if (!maybe.IsInitialized) { return 20; }
    if (maybe.AssumeInitRead() != 7) { return 21; }
    var consumed = maybe.AssumeInit();
    if (consumed != 7) { return 22; }

    var caught = false;
    try
    {
        var invalid = MaybeUninit<int>.Init(1);
        invalid.Write(2);
    }
    catch (InvalidOperationException)
    {
        caught = true;
    }
    if (!caught) { return 23; }

    return 0;
}
"#,
    );

    cargo_bin_cmd!("chic")
        .arg("check")
        .arg(&main_src)
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("CHIC_LOG_LEVEL", "error")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("check passed for")
                .or(predicate::str::contains("check completed with diagnostics")),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn std_collections_hashset_facade_available() {
    let dir = tempfile::tempdir().expect("temp dir");
    let main_src = dir.path().join("hashset_usage.cl");

    write_source(
        &main_src,
        r#"
import Std.Collections;

namespace CollectionsFacade;

public bool IsEven(in int value)
{
    return value % 2 == 0;
}

public int Main()
{
    var hashSet = new HashSet<int>();
    var status = hashSet.Insert(5, out var inserted);
    if (status != HashSetError.Success || !inserted) { return 1; }
    if (!hashSet.Contains(5)) { return 2; }

    var maybe = hashSet.Get(5);
    if (!maybe.IsSome(out var value) || value != 5) { return 3; }

    hashSet.Insert(6, out inserted);
    var iter = hashSet.Iter();
    var count = 0;
    while (iter.Next(out var item))
    {
        count += 1;
    }
    if (count == 0) { return 4; }

    var entry = hashSet.Entry(7);
    var entryStatus = entry.OrInsert(out inserted);
    if (entryStatus != HashSetError.Success || !inserted) { return 5; }

    hashSet.Retain(IsEven);
    if (hashSet.Contains(5)) { return 6; }

    var drain = hashSet.Drain();
    while (drain.Next(out var drained))
    {
        // discard
    }
    if (hashSet.Len() != 0) { return 7; }

    return 0;
}
"#,
    );

    cargo_bin_cmd!("chic")
        .arg("check")
        .arg(&main_src)
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("CHIC_LOG_LEVEL", "error")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("check passed for")
                .or(predicate::str::contains("check completed with diagnostics")),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn std_collections_hashmap_facade_available() {
    let dir = tempfile::tempdir().expect("temp dir");
    let main_src = dir.path().join("hashmap_usage.cl");

    write_source(
        &main_src,
        r#"
import Std.Core;
import Std.Collections;

namespace CollectionsFacade;

public int Main()
{
    var map = new HashMap<int, int>();
    var status = map.Insert(3, 9, out var previous);
    if (status != HashMapError.Success) { return 1; }
    if (previous.IsSome(out _)) { return 2; }
    if (!map.ContainsKey(3)) { return 3; }

    var maybe = map.Get(3);
    if (!maybe.IsSome(out var value) || value != 9) { return 4; }

    status = map.Insert(3, 11, out previous);
    if (status != HashMapError.Success) { return 5; }
    if (!previous.IsSome(out var old) || old != 9) { return 6; }

    var taken = map.Take(3);
    if (!taken.IsSome(out var removed) || removed != 11) { return 7; }
    var missing = map.Take(3);
    if (missing.IsSome(out _)) { return 8; }

    var prev = Option<int>.None();
    map.Insert(1, 2, out prev);
    map.Insert(2, 4, out prev);

    var iter = map.Iter();
    var sum = 0;
    var count = 0;
    while (iter.Next(out var key, out var val))
    {
        sum += key * val;
        count += 1;
    }
    if (count != 2) { return 9; }
    if (sum != 10) { return 10; }
    return 0;
}
"#,
    );

    cargo_bin_cmd!("chic")
        .arg("check")
        .arg(&main_src)
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("CHIC_LOG_LEVEL", "error")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("check passed for")
                .or(predicate::str::contains("check completed with diagnostics")),
        )
        .stderr(predicate::str::is_empty());
}
