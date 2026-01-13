use super::common::RequireExt;
use super::*;

#[test]
fn mmio_compound_assignment_emits_binary_and_store() {
    let source = r#"
namespace Sample;

@mmio(base = 8192)
public struct Device
{
    @register(offset = 0, width = 32)
    public int Data;
}

public void Bump(Device dev)
{
    unsafe
    {
        dev.Data += 1;
    }
}
"#;
    let parsed = parse_module(source).require("parse mmio compound assignment module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let bump = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Bump"))
        .require("missing Bump function");

    let body = &bump.body;
    let mut saw_binary = false;
    let mut saw_store = false;
    for block in &body.blocks {
        for stmt in &block.statements {
            match &stmt.kind {
                MirStatementKind::Assign {
                    value: Rvalue::Binary { .. },
                    ..
                } => saw_binary = true,
                MirStatementKind::MmioStore { .. } => saw_store = true,
                _ => {}
            }
        }
    }
    assert!(
        saw_binary,
        "compound MMIO assignment should emit a binary rvalue"
    );
    assert!(
        saw_store,
        "compound MMIO assignment should write back via MmioStore"
    );
    verify_body(body).require("verify mmio compound assignment body");
}

#[test]
fn compound_property_assignment_reports_diagnostic() {
    let source = r#"
namespace Demo;

public class Counter
{
    public int Value { get; set; }
}

public void Increment(Counter counter)
{
    counter.Value += 1;
}
"#;

    let parsed = parse_module(source).require("parse property module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("compound assignment on property")),
        "expected compound property diagnostic, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn question_mark_with_throws_emits_throw_terminator() {
    let source = r#"
namespace Demo;

public class Exception { }

public enum IntResult
{
    Ok { public int Value; },
    Err { public Exception Error; }
}

public int Convert(IntResult input) throws Exception
{
    return input?;
}
"#;

    let parsed = parse_module(source).require("parse throws module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let convert = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Convert"))
        .require("Convert function");
    let body = &convert.body;
    let has_match = body
        .blocks
        .iter()
        .any(|block| matches!(block.terminator, Some(Terminator::Match { .. })));
    assert!(
        has_match,
        "`?` with throws should lower to a match over the result operand"
    );
    let return_paths = body
        .blocks
        .iter()
        .filter(|block| matches!(block.terminator, Some(Terminator::Return)))
        .count();
    assert!(
        return_paths >= 1,
        "`?` lowering should introduce early return paths for Err variants"
    );
}

#[test]
fn lowers_switch_expression_into_match() {
    let source = r#"
namespace Demo;

public int Select(int value)
{
    return value switch {
        0 => 1,
        _ => 2,
    };
}
"#;

    let parsed = parse_module(source).require("parse switch expression module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Select"))
        .require("missing Select function");

    let mut saw_match = false;
    for block in &func.body.blocks {
        if let Some(Terminator::Match { .. }) = block.terminator {
            saw_match = true;
        }
        for stmt in &block.statements {
            assert!(
                !matches!(stmt.kind, MirStatementKind::Pending(_)),
                "pending statement leaked into switch expression lowering"
            );
        }
    }
    assert!(
        saw_match,
        "switch expression should lower to Match terminator"
    );
    verify_body(&func.body).require("verify switch expression body");
}

#[test]
fn question_mark_converts_error_payload_with_from_impl() {
    let source = r#"
namespace Demo;

public enum IntErrorAResult
{
    Ok,
    Err { public ErrorA Error; }
}

public enum IntErrorBResult
{
    Ok,
    Err { public ErrorB Error; }
}

public struct ErrorA { }

public class ErrorB
{
    public static ErrorB from(ErrorA value)
    {
        return new ErrorB();
    }
}

public IntErrorBResult Convert(IntErrorAResult input)
{
    input?;
    return IntErrorBResult.Ok;
}
"#;

    let parsed = parse_module(source).require("parse conversion module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let convert = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Convert"))
        .require("Convert function");
    let saw_from_call = convert
        .body
        .blocks
        .iter()
        .any(|block| match &block.terminator {
            Some(Terminator::Call { func, .. }) => match func {
                Operand::Const(ConstOperand {
                    value: ConstValue::Symbol(name),
                    ..
                }) => name.ends_with("ErrorB::from"),
                Operand::Pending(pending) => pending.repr.ends_with("ErrorB::from"),
                _ => false,
            },
            _ => false,
        });
    assert!(
        saw_from_call,
        "expected Err conversion path to invoke ErrorB::from"
    );
}

#[test]
fn conditional_expression_builds_switch() {
    let source = r#"
namespace Demo;

public int Select(bool flag, int left, int right)
{
    let value = flag ? left : right;
    return value;
}
"#;
    let parsed = parse_module(source).require("parse conditional expr module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let select = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Select"))
        .require("Select function");
    assert!(
        select
            .body
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Some(Terminator::SwitchInt { .. }))),
        "conditional expression should lower to a SwitchInt terminator"
    );
}

#[test]
fn question_mark_requires_result_return_type() {
    let source = r#"
namespace Demo;

public enum IntResult
{
    Ok { public int Value; },
    Err { public int Error; }
}

public int Convert(IntResult input)
{
    return input?;
}
"#;

    let parsed = parse_module(source).require("parse mismatched return module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("requires the enclosing function to return `Result<_, _>`")),
        "expected diagnostic about non-Result return type, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn mmio_read_from_write_only_register_is_rejected() {
    let source = r#"
namespace Sample;

@mmio(base = 12288)
public struct Device
{
    @register(offset = 0, width = 32, access = "wo")
    public int Data;
}

public int Read(Device dev)
{
    unsafe
    {
        return dev.Data;
    }
}
"#;

    let parsed = parse_module(source).require("parse write-only mmio module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("write-only")
                || diag.message.contains("cannot be read")),
        "expected write-only diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn static_property_assignment_requires_type_name() {
    let source = r#"
namespace Demo;

public class Counter
{
    public static int Count { get; set; }

    public void Touch(ref this)
    {
        this.Count = 1;
    }
}
"#;

    let parsed = parse_module(source).require("parse static property module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("must be accessed using the type name")),
        "expected static property diagnostic, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn property_assignment_invokes_setter() {
    let source = r#"
namespace Demo;

public class Counter
{
    private int backing;

    public int Value
    {
        get { return backing; }
        set { backing = value; }
    }

    public void Set(ref this, int value)
    {
        this.Value = value;
    }
}
"#;

    let parsed = parse_module(source).require("parse property setter module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let set_method = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Counter::Set"))
        .require("Counter::Set function");
    let invokes_setter = set_method.body.blocks.iter().any(|block| {
        matches!(
            &block.terminator,
            Some(Terminator::Call { func, .. })
                if matches!(
                    func,
                    Operand::Pending(pending) if pending.repr.ends_with("Counter::set_Value")
                )
        )
    });
    assert!(invokes_setter, "expected setter call in Counter::Set");
}

#[test]
fn question_mark_operand_must_be_result_enum() {
    let source = r#"
namespace Demo;

public int Convert(int input)
{
    return input?;
}
"#;

    let parsed = parse_module(source).require("parse non-result operand module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("requires an enum `Result<T, E>` operand")),
        "expected diagnostic about Result operand, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn question_mark_requires_err_variant_on_operand() {
    let source = r#"
namespace Demo;

public enum OnlyOk
{
    Ok,
}

public OnlyOk Convert(OnlyOk input)
{
    return input?;
}
"#;

    let parsed = parse_module(source).require("parse missing err variant module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("does not expose an `Err`/`Error` variant required for `?`")),
        "expected missing Err variant diagnostic, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn question_mark_reports_missing_error_conversion() {
    let source = r#"
namespace Demo;

public enum ResultA
{
    Ok,
    Err { public ErrorA Error; }
}

public enum ResultB
{
    Ok,
    Err { public ErrorB Error; }
}

public struct ErrorA { }
public class ErrorB { }

public ResultB Convert(ResultA input)
{
    return input?;
}
"#;

    let parsed = parse_module(source).require("parse missing conversion module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("cannot convert error type")),
        "expected diagnostic about missing ErrorB::from(ErrorA), got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn question_mark_reports_error_arity_mismatch() {
    let source = r#"
namespace Demo;

public enum UnaryResult
{
    Ok { public int Value; },
    Err { public int Error; }
}

public enum EmptyErrResult
{
    Ok { public int Value; },
    Err,
}

public EmptyErrResult Convert(UnaryResult input)
{
    return input?;
}
"#;

    let parsed = parse_module(source).require("parse arity mismatch module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains(
                "mismatched error payload arity between operand and return types when using `?`"
            )),
        "expected diagnostic about error payload arity mismatch, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn question_mark_requires_ok_variant_on_operand() {
    let source = r#"
namespace Demo;

public enum OnlyErr
{
    Err { public int Error; }
}

public enum ReturnResult
{
    Ok,
    Err { public int Error; }
}

public ReturnResult Convert(OnlyErr input)
{
    return input?;
}
"#;

    let parsed = parse_module(source).require("parse missing ok variant module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("does not expose an `Ok`/`Success` variant required for `?`")),
        "expected missing Ok variant diagnostic, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn question_mark_rejects_multiple_ok_fields() {
    let source = r#"
namespace Demo;

public enum DualOkResult
{
    Ok { public int Left; public int Right; },
    Err { public int Error; }
}

public DualOkResult Convert(DualOkResult input)
{
    return input?;
}
"#;

    let parsed = parse_module(source).require("parse dual ok variant module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("supports `Ok` variants with at most one payload field")),
        "expected diagnostic about Ok payload arity, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn question_mark_rejects_multiple_err_fields() {
    let source = r#"
namespace Demo;

public enum DualErrResult
{
    Ok,
    Err { public int Code; public int Extra; }
}

public DualErrResult Convert(DualErrResult input)
{
    return input?;
}
"#;

    let parsed = parse_module(source).require("parse dual err variant module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("supports `Err` variants with at most one payload field")),
        "expected diagnostic about Err payload arity, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn question_mark_reports_success_payload_arity_mismatch() {
    let source = r#"
namespace Demo;

public enum OperandResult
{
    Ok { public int Value; },
    Err { public int Error; }
}

public enum ReturnResult
{
    Ok,
    Err { public int Error; }
}

public ReturnResult Convert(OperandResult input)
{
    return input?;
}
"#;

    let parsed = parse_module(source).require("parse success arity mismatch module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains(
                "mismatched success payload arity between operand and return types when using `?`"
            )),
        "expected diagnostic about success payload arity mismatch, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn question_mark_throws_requires_exception_payload() {
    let source = r#"
namespace Demo;

public class Exception { }
public struct Problem { }

public enum ThrowResult
{
    Ok { public int Value; },
    Err { public Problem Error; }
}

public int Convert(ThrowResult input) throws Exception
{
    return input?;
}
"#;

    let parsed = parse_module(source).require("parse throws non-exception module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("requires the error payload")
                && diag.message.contains("Exception")),
        "expected diagnostic about non-exception error payload, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn question_mark_throws_requires_error_payload() {
    let source = r#"
namespace Demo;

public class Exception { }

public enum ThrowResult
{
    Ok,
    Err,
}

public int Convert(ThrowResult input) throws Exception
{
    return input?;
}
"#;

    let parsed = parse_module(source).require("parse throws empty err module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("requires the `Err` variant to carry exactly one exception payload")),
        "expected diagnostic about missing exception payload under throws, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn mmio_write_to_read_only_register_is_rejected() {
    let source = r#"
namespace Sample;

@mmio(base = 16384)
public struct Device
{
    @register(offset = 0, width = 32, access = "ro")
    public int Data;
}

public void Write(Device dev)
{
    unsafe
    {
        dev.Data = 5;
    }
}
"#;

    let parsed = parse_module(source).require("parse read-only mmio module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("read-only")),
        "expected read-only diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn mmio_access_requires_unsafe_block() {
    let source = r#"
namespace Sample;

@mmio(base = 20480)
public struct Device
{
    @register(offset = 0, width = 32)
    public int Data;
}

public int Read(Device dev)
{
    return dev.Data;
}
"#;

    let parsed = parse_module(source).require("parse unsafe-required mmio module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("requires an `unsafe` block")),
        "expected diagnostic about missing unsafe block, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn bin_op_for_assign_covers_compound_operators() {
    use crate::mir::builder::body_builder::BodyBuilder;

    assert_eq!(
        BodyBuilder::bin_op_for_assign(AssignOp::AddAssign),
        Some(BinOp::Add)
    );
    assert_eq!(
        BodyBuilder::bin_op_for_assign(AssignOp::SubAssign),
        Some(BinOp::Sub)
    );
    assert_eq!(
        BodyBuilder::bin_op_for_assign(AssignOp::MulAssign),
        Some(BinOp::Mul)
    );
    assert_eq!(
        BodyBuilder::bin_op_for_assign(AssignOp::DivAssign),
        Some(BinOp::Div)
    );
    assert_eq!(
        BodyBuilder::bin_op_for_assign(AssignOp::RemAssign),
        Some(BinOp::Rem)
    );
    assert_eq!(
        BodyBuilder::bin_op_for_assign(AssignOp::BitAndAssign),
        Some(BinOp::BitAnd)
    );
    assert_eq!(
        BodyBuilder::bin_op_for_assign(AssignOp::BitOrAssign),
        Some(BinOp::BitOr)
    );
    assert_eq!(
        BodyBuilder::bin_op_for_assign(AssignOp::BitXorAssign),
        Some(BinOp::BitXor)
    );
    assert_eq!(
        BodyBuilder::bin_op_for_assign(AssignOp::ShlAssign),
        Some(BinOp::Shl)
    );
    assert_eq!(
        BodyBuilder::bin_op_for_assign(AssignOp::ShrAssign),
        Some(BinOp::Shr)
    );
    assert!(BodyBuilder::bin_op_for_assign(AssignOp::Assign).is_none());
    assert!(BodyBuilder::bin_op_for_assign(AssignOp::NullCoalesceAssign).is_none());
}
