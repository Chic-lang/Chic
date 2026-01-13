use expect_test::expect_file;

use crate::harness::Format;

use super::{COMMANDS, frontend_filter};

log_snapshot_test!(
    text_frontend_pipeline,
    Format::Text,
    frontend_filter(),
    COMMANDS,
    expect_file!["./snapshots/frontend_text.snap"]
);
