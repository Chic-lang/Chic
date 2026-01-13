use expect_test::expect_file;

use crate::harness::Format;

use super::{COMMANDS, frontend_filter};

log_snapshot_test!(
    json_frontend_pipeline,
    Format::Json,
    frontend_filter(),
    COMMANDS,
    expect_file!["./snapshots/frontend_json.snap"]
);
