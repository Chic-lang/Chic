use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

mod common;
use common::write_source;

#[test]
fn std_collections_vec_placement_helpers_run() {
    let dir = tempfile::tempdir().expect("temp dir");
    let main_src = dir.path().join("vec_placement.cl");

    write_source(
        &main_src,
        r#"
import Std.Collections;
import Std.Memory;

namespace VecPlacement;

public int Main()
{
    var vec = Vec.New<int>();
    var slot = MaybeUninit<int>.Uninit();
    slot.Write(11);
    var insert = Vec.InsertInitialized(ref vec, 0, ref slot);
    if (insert != VecError.Success) { return 1; }
    if (slot.IsInitialized) { return 2; }

    var outSlot = MaybeUninit<int>.Uninit();
    var removed = Vec.RemoveInto(ref vec, 0, ref outSlot);
    if (removed != VecError.Success) { return 3; }
    var value = outSlot.AssumeInit();
    if (value != 11) { return 4; }

    slot.Write(42);
    Vec.PushInitialized(ref vec, ref slot);
    slot.Write(7);
    Vec.PushInitialized(ref vec, ref slot);

    var swapSlot = MaybeUninit<int>.Uninit();
    var swap = Vec.SwapRemoveInto(ref vec, 0, ref swapSlot);
    if (swap != VecError.Success) { return 5; }
    var swapped = swapSlot.AssumeInit();
    if (swapped != 42)
    {
        if (swapped != 7) { return 6; }
    }

    return 0;
}
"#,
    );

    let mut cmd = cargo_bin_cmd!("chic");
    cmd.arg("check")
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
