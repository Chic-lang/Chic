use expect_test::expect;

use crate::harness::{CommandKind, FilterKind, Format};

const COMMANDS: &[CommandKind] = &[
    CommandKind::Check,
    CommandKind::Build,
    CommandKind::Test,
    CommandKind::Run,
];

log_snapshot_test!(
    text_cli_pipeline,
    Format::Text,
    FilterKind::stage("cli."),
    COMMANDS,
    expect![[r#"
        == chic check (text) ==
        status: Some(0)
        stderr:
        __TIMESTAMP__  INFO pipeline: stage="cli.run.header" command="check" log_level=info log_format=text trace_pipeline=false target="aarch64-unknown-none" backend="n/a" kind="executable" input_count=1 inputs=sample.ch
        __TIMESTAMP__  INFO pipeline: stage="cli.run.start" command="check" status="start" target="aarch64-unknown-none" backend="n/a" kind="executable" input_count=1 inputs=sample.ch
        __TIMESTAMP__  INFO pipeline: stage="cli.run.footer" command="check" status="ok" target="aarch64-unknown-none" backend="n/a" kind="executable" input_count=1 inputs=sample.ch elapsed_ms=__ELAPSED__

        == chic build (text) ==
        status: Some(0)
        stderr:
        __TIMESTAMP__  INFO pipeline: stage="cli.run.header" command="build" log_level=info log_format=text trace_pipeline=false target="aarch64-unknown-none" backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.ch
        __TIMESTAMP__  INFO pipeline: stage="cli.run.start" command="build" status="start" target="aarch64-unknown-none" backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.ch
        __TIMESTAMP__  INFO pipeline: stage="cli.run.footer" command="build" status="ok" target="aarch64-unknown-none" backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.ch elapsed_ms=__ELAPSED__

        == chic test (text) ==
        status: Some(0)
        stderr:
        __TIMESTAMP__  INFO pipeline: stage="cli.run.header" command="test" log_level=info log_format=text trace_pipeline=false target="aarch64-unknown-none" backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.ch
        __TIMESTAMP__  INFO pipeline: stage="cli.run.start" command="test" status="start" target="aarch64-unknown-none" backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.ch
        __TIMESTAMP__  INFO pipeline: stage="cli.run.footer" command="test" status="ok" target="aarch64-unknown-none" backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.ch elapsed_ms=__ELAPSED__

        == chic run (text) ==
        status: Some(1)
        stderr:
        __TIMESTAMP__  INFO pipeline: stage="cli.run.header" command="run" log_level=info log_format=text trace_pipeline=false target="aarch64-unknown-none" backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.ch
        __TIMESTAMP__  INFO pipeline: stage="cli.run.start" command="run" status="start" target="aarch64-unknown-none" backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.ch
        __TIMESTAMP__ ERROR pipeline: stage="cli.run.footer" command="run" status="error" target="aarch64-unknown-none" backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.ch elapsed_ms=__ELAPSED__ error=internal error: unable to locate entry point `Main` in module for wasm execution"#]]
);

log_snapshot_test!(
    json_cli_pipeline,
    Format::Json,
    FilterKind::stage("cli."),
    COMMANDS,
    expect![[r#"
        == chic check (json) ==
        status: Some(0)
        stderr:
        {"fields":{"backend":"n/a","command":"check","input_count":"__COUNT__","inputs":"sample.ch","kind":"executable","log_format":"json","log_level":"info","stage":"cli.run.header","target":"aarch64-unknown-none","trace_pipeline":false},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"n/a","command":"check","input_count":"__COUNT__","inputs":"sample.ch","kind":"executable","stage":"cli.run.start","status":"start","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"n/a","command":"check","elapsed_ms":0,"input_count":"__COUNT__","inputs":"sample.ch","kind":"executable","stage":"cli.run.footer","status":"ok","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}

        == chic build (json) ==
        status: Some(0)
        stderr:
        {"fields":{"backend":"wasm","command":"build","input_count":"__COUNT__","inputs":"/private{TMP}/sample.ch","kind":"executable","log_format":"json","log_level":"info","stage":"cli.run.header","target":"aarch64-unknown-none","trace_pipeline":false},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"build","input_count":"__COUNT__","inputs":"/private{TMP}/sample.ch","kind":"executable","stage":"cli.run.start","status":"start","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"build","elapsed_ms":0,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.ch","kind":"executable","stage":"cli.run.footer","status":"ok","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}

        == chic test (json) ==
        status: Some(0)
        stderr:
        {"fields":{"backend":"wasm","command":"test","input_count":"__COUNT__","inputs":"/private{TMP}/sample.ch","kind":"executable","log_format":"json","log_level":"info","stage":"cli.run.header","target":"aarch64-unknown-none","trace_pipeline":false},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"test","input_count":"__COUNT__","inputs":"/private{TMP}/sample.ch","kind":"executable","stage":"cli.run.start","status":"start","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"test","elapsed_ms":0,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.ch","kind":"executable","stage":"cli.run.footer","status":"ok","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}

        == chic run (json) ==
        status: Some(1)
        stderr:
        {"fields":{"backend":"wasm","command":"run","input_count":"__COUNT__","inputs":"/private{TMP}/sample.ch","kind":"executable","log_format":"json","log_level":"info","stage":"cli.run.header","target":"aarch64-unknown-none","trace_pipeline":false},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"run","input_count":"__COUNT__","inputs":"/private{TMP}/sample.ch","kind":"executable","stage":"cli.run.start","status":"start","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"run","elapsed_ms":0,"error":"internal error: unable to locate entry point `Main` in module for wasm execution","input_count":"__COUNT__","inputs":"/private{TMP}/sample.ch","kind":"executable","stage":"cli.run.footer","status":"error","target":"aarch64-unknown-none"},"level":"ERROR","target":"pipeline","timestamp":"__TIMESTAMP__"}"#]]
);
