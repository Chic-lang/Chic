use super::*;
use crate::mir::builder::tests::common::RequireExt;

#[test]
fn async_immediate_await_emits_no_async_attr_diagnostics() {
    let source = r#"
namespace Sample;

public async Task<int> Immediate(Future<int> future)
{
    return await future;
}
"#;
    let parsed = parse_module(source).require("parse async module");
    let lowering = lower_module(&parsed.module);
    assert!(
        !lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("[AS0")),
        "unexpected AS-series diagnostics: {:?}",
        lowering.diagnostics
    );
}

#[test]
fn async_frame_limit_violation_reports_diagnostic() {
    let source = r#"
namespace Sample;

@frame_limit(8)
public async Task<int> Limited(Future<int> future)
{
    let payload = 7;
    return await future + payload;
}
"#;
    let parsed = parse_module(source).require("parse frame limit module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("[AS0002]")),
        "expected AS0002 frame-limit diagnostic, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn async_no_capture_blocks_captured_locals() {
    let source = r#"
namespace Sample;

@no_capture
public async Task<int> NoCapture(Future<int> future)
{
    let payload = 3;
    await future;
    return payload;
}
"#;
    let parsed = parse_module(source).require("parse no-capture module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("[AS0003]") && diag.message.contains("payload")),
        "expected AS0003 no-capture diagnostic mentioning payload, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn async_stack_only_flags_captured_state() {
    let source = r#"
namespace Sample;

@stack_only
public async Task<int> StackOnly(Future<int> future)
{
    let payload = 5;
    await future;
    return payload;
}
"#;
    let parsed = parse_module(source).require("parse stack-only module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("[AS0001]")),
        "expected AS0001 stack-only diagnostic, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn async_attributes_rejected_on_non_async_functions() {
    let source = r#"
namespace Sample;

@stack_only
public int NotAsync()
{
    return 1;
}
"#;
    let parsed = parse_module(source).require("parse non-async module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("[AS0004]")),
        "expected AS0004 attribute diagnostic, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn async_frame_limit_rejects_invalid_payloads() {
    let source = r#"
namespace Sample;

@frame_limit("abc")
public async Task<int> BadLimit(Future<int> future)
{
    await future;
    return 0;
}
"#;
    let parsed = parse_module(source).require("parse invalid frame limit");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("[AS0004]") && diag.message.contains("byte count")),
        "expected AS0004 attribute payload diagnostic, got {:?}",
        lowering.diagnostics
    );
}
