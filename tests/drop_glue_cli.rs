use assert_cmd::cargo::cargo_bin_cmd;

mod common;
use common::write_source;

#[test]
fn chic_check_handles_drop_glue_registration() {
    let dir = tempfile::tempdir().expect("temp dir");
    let main_src = dir.path().join("drop_glue_check.ch");

    write_source(
        &main_src,
        r#"
import Std.Collections;

namespace DropGlueCheck;

public struct NeedsDrop
{
    public void dispose(ref this) { }
}

public int Main()
{
    var vec = Vec.New<NeedsDrop>();
    VecIntrinsics.chic_rt_vec_drop(ref vec);
    return 0;
}
"#,
    );

    let mut cmd = cargo_bin_cmd!("chic");
    cmd.arg("check")
        .arg(&main_src)
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("CHIC_SKIP_MIR_VERIFY", "1")
        .env(
            "CHIC_STDLIB_BLOCKLIST",
            "packages/std.net/src/,packages/std.data/src/",
        )
        .assert()
        .success();
}
