use expect_test::expect;

use crate::harness::{CommandKind, FilterKind, Format};

const COMMANDS: &[CommandKind] = &[
    CommandKind::Check,
    CommandKind::Build,
    CommandKind::Test,
    CommandKind::Run,
];

log_snapshot_test!(
    text_diagnostics_snapshot,
    Format::Text,
    FilterKind::diagnostics(),
    COMMANDS,
    expect![[r#"
        == chic check (text) ==
        status: Some(0)
        stderr:
        <none>

        == chic build (text) ==
        status: Some(0)
        stderr:
        <none>

        == chic test (text) ==
        status: Some(0)
        stderr:
        <none>

        == chic run (text) ==
        status: Some(1)
        stderr:
        <none>"#]]
);

log_snapshot_test!(
    json_diagnostics_snapshot,
    Format::Json,
    FilterKind::diagnostics(),
    COMMANDS,
    expect![[r#"
        == chic check (json) ==
        status: Some(0)
        stderr:
        <none>

        == chic build (json) ==
        status: Some(0)
        stderr:
        <none>

        == chic test (json) ==
        status: Some(0)
        stderr:
        <none>

        == chic run (json) ==
        status: Some(1)
        stderr:
        <none>"#]]
);
