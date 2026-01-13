use expect_test::expect;

use crate::harness::{CommandKind, FilterKind, Format};

const COMMANDS: &[CommandKind] = &[
    CommandKind::Check,
    CommandKind::Build,
    CommandKind::Test,
    CommandKind::Run,
];

log_snapshot_test!(
    text_driver_pipeline,
    Format::Text,
    FilterKind::stage("driver."),
    COMMANDS,
    expect![[r#"
        == chic check (text) ==
        status: Some(0)
        stderr:
        __TIMESTAMP__  INFO pipeline: stage="driver.check.start" command="check" status="start" target=aarch64-unknown-none backend="llvm" kind="executable" input_count=1 inputs=sample.cl load_stdlib=false
        __TIMESTAMP__  INFO pipeline: stage="driver.check.complete" command="check" status="ok" target=aarch64-unknown-none backend="llvm" kind="executable" input_count=1 inputs=sample.cl module_count=__MODULES__ lowering_diagnostics=__LOWERING__ borrow_diagnostics=__BORROW__ type_diagnostics=__TYPE__ elapsed_ms=__ELAPSED__

        == chic build (text) ==
        status: Some(0)
        stderr:
        __TIMESTAMP__  INFO pipeline: stage="driver.build.start" command="build" status="start" target=aarch64-unknown-none backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.cl load_stdlib=false emit_wat=false
        __TIMESTAMP__  INFO pipeline: stage="driver.build.frontend" command="build" status="ok" target=aarch64-unknown-none backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.cl elapsed_ms=__ELAPSED__
        __TIMESTAMP__  INFO pipeline: stage="driver.build.codegen.wasm" command="build" status="ok" target=aarch64-unknown-none backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.cl elapsed_ms=__ELAPSED__
        __TIMESTAMP__  INFO pipeline: stage="driver.build.complete" command="build" status="ok" target=aarch64-unknown-none backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.cl load_stdlib=false emit_wat=false artifact="out.wasm" elapsed_ms=__ELAPSED__

        == chic test (text) ==
        status: Some(0)
        stderr:
        __TIMESTAMP__  INFO pipeline: stage="driver.run_tests.start" command="test" status="start" target=aarch64-unknown-none backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.cl load_stdlib=false
        __TIMESTAMP__  INFO pipeline: stage="driver.run_tests_wasm.start" command="test" status="start" target=aarch64-unknown-none backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.cl load_stdlib=false
        __TIMESTAMP__  INFO pipeline: stage="driver.build.start" command="build" status="start" target=aarch64-unknown-none backend="wasm" kind="static-library" input_count=1 inputs=/private{TMP}/sample.cl load_stdlib=false emit_wat=false
        __TIMESTAMP__  INFO pipeline: stage="driver.build.frontend" command="build" status="ok" target=aarch64-unknown-none backend="wasm" kind="static-library" input_count=1 inputs=/private{TMP}/sample.cl elapsed_ms=__ELAPSED__
        __TIMESTAMP__  INFO pipeline: stage="driver.build.codegen.wasm" command="build" status="ok" target=aarch64-unknown-none backend="wasm" kind="static-library" input_count=1 inputs=/private{TMP}/sample.cl elapsed_ms=__ELAPSED__
        __TIMESTAMP__  INFO pipeline: stage="driver.build.complete" command="build" status="ok" target=aarch64-unknown-none backend="wasm" kind="static-library" input_count=1 inputs=/private{TMP}/sample.cl load_stdlib=false emit_wat=false artifact="sample.wasm" elapsed_ms=__ELAPSED__
        __TIMESTAMP__  INFO pipeline: stage="driver.run_tests_wasm.complete" command="test" status="ok" target=aarch64-unknown-none backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.cl testcase_count=1 load_stdlib=false elapsed_ms=__ELAPSED__
        __TIMESTAMP__  INFO pipeline: stage="driver.run_tests.complete" command="test" status="ok" target=aarch64-unknown-none backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.cl testcase_count=1 load_stdlib=false elapsed_ms=__ELAPSED__

        == chic run (text) ==
        status: Some(1)
        stderr:
        __TIMESTAMP__  INFO pipeline: stage="driver.run.start" command="run" status="start" target=aarch64-unknown-none backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.cl load_stdlib=false
        __TIMESTAMP__  INFO pipeline: stage="driver.build.start" command="build" status="start" target=aarch64-unknown-none backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.cl load_stdlib=false emit_wat=false
        __TIMESTAMP__  INFO pipeline: stage="driver.build.frontend" command="build" status="ok" target=aarch64-unknown-none backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.cl elapsed_ms=__ELAPSED__
        __TIMESTAMP__  INFO pipeline: stage="driver.build.codegen.wasm" command="build" status="ok" target=aarch64-unknown-none backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.cl elapsed_ms=__ELAPSED__
        __TIMESTAMP__  INFO pipeline: stage="driver.build.complete" command="build" status="ok" target=aarch64-unknown-none backend="wasm" kind="executable" input_count=1 inputs=/private{TMP}/sample.cl load_stdlib=false emit_wat=false artifact="chic-run.wasm" elapsed_ms=__ELAPSED__"#]]
);

log_snapshot_test!(
    json_driver_pipeline,
    Format::Json,
    FilterKind::stage("driver."),
    COMMANDS,
    expect![[r#"
        == chic check (json) ==
        status: Some(0)
        stderr:
        {"fields":{"backend":"llvm","command":"check","input_count":"__COUNT__","inputs":"sample.cl","kind":"executable","load_stdlib":false,"stage":"driver.check.start","status":"start","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"llvm","borrow_diagnostics":"__COUNT__","command":"check","elapsed_ms":0,"input_count":"__COUNT__","inputs":"sample.cl","kind":"executable","lowering_diagnostics":"__COUNT__","module_count":"__COUNT__","stage":"driver.check.complete","status":"ok","target":"aarch64-unknown-none","type_diagnostics":"__COUNT__"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}

        == chic build (json) ==
        status: Some(0)
        stderr:
        {"fields":{"backend":"wasm","command":"build","emit_wat":false,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"executable","load_stdlib":false,"stage":"driver.build.start","status":"start","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"build","elapsed_ms":0,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"executable","stage":"driver.build.frontend","status":"ok","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"build","elapsed_ms":0,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"executable","stage":"driver.build.codegen.wasm","status":"ok","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"artifact":"out.wasm","backend":"wasm","command":"build","elapsed_ms":0,"emit_wat":false,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"executable","load_stdlib":false,"stage":"driver.build.complete","status":"ok","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}

        == chic test (json) ==
        status: Some(0)
        stderr:
        {"fields":{"backend":"wasm","command":"test","input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"executable","load_stdlib":false,"stage":"driver.run_tests.start","status":"start","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"test","input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"executable","load_stdlib":false,"stage":"driver.run_tests_wasm.start","status":"start","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"build","emit_wat":false,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"static-library","load_stdlib":false,"stage":"driver.build.start","status":"start","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"build","elapsed_ms":0,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"static-library","stage":"driver.build.frontend","status":"ok","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"build","elapsed_ms":0,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"static-library","stage":"driver.build.codegen.wasm","status":"ok","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"artifact":"sample.wasm","backend":"wasm","command":"build","elapsed_ms":0,"emit_wat":false,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"static-library","load_stdlib":false,"stage":"driver.build.complete","status":"ok","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"test","elapsed_ms":0,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"executable","load_stdlib":false,"stage":"driver.run_tests_wasm.complete","status":"ok","target":"aarch64-unknown-none","testcase_count":1},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"test","elapsed_ms":0,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"executable","load_stdlib":false,"stage":"driver.run_tests.complete","status":"ok","target":"aarch64-unknown-none","testcase_count":1},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}

        == chic run (json) ==
        status: Some(1)
        stderr:
        {"fields":{"backend":"wasm","command":"run","input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"executable","load_stdlib":false,"stage":"driver.run.start","status":"start","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"build","emit_wat":false,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"executable","load_stdlib":false,"stage":"driver.build.start","status":"start","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"build","elapsed_ms":0,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"executable","stage":"driver.build.frontend","status":"ok","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"backend":"wasm","command":"build","elapsed_ms":0,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"executable","stage":"driver.build.codegen.wasm","status":"ok","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}
        {"fields":{"artifact":"chic-run.wasm","backend":"wasm","command":"build","elapsed_ms":0,"emit_wat":false,"input_count":"__COUNT__","inputs":"/private{TMP}/sample.cl","kind":"executable","load_stdlib":false,"stage":"driver.build.complete","status":"ok","target":"aarch64-unknown-none"},"level":"INFO","target":"pipeline","timestamp":"__TIMESTAMP__"}"#]]
);
