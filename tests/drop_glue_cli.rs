use assert_cmd::Command;

mod common;
use common::write_source;

#[test]
fn chic_check_handles_drop_glue_registration() {
    let dir = tempfile::tempdir().expect("temp dir");
    let main_src = dir.path().join("drop_glue_check.cl");

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

    Command::cargo_bin("chic")
        .expect("chic binary")
        .arg("check")
        .arg(&main_src)
        .env("CHIC_TRACE_PIPELINE", "0")
        .assert()
        .success();
}
