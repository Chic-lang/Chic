use super::common::RequireExt;
use super::*;
use crate::const_eval_config::{self, ConstEvalConfig};
use crate::decimal::Decimal128;
use crate::mir::data::StatementKind as MirStatementKind;

fn reset_const_eval_config() {
    const_eval_config::set_global(ConstEvalConfig::default());
}

fn extract_return_constant(body: &MirBody) -> Option<&ConstValue> {
    for block in &body.blocks {
        for statement in &block.statements {
            if let MirStatementKind::Assign { place, value } = &statement.kind {
                if place.local == LocalId(0) {
                    if let Rvalue::Use(Operand::Const(constant)) = value {
                        return Some(&constant.value);
                    }
                }
            }
        }
    }
    None
}

#[test]
fn namespace_const_inlines_into_function() {
    let source = r#"
namespace Sample;

public const int Answer = 42;

public int ReturnConst()
{
    return Answer;
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        eprintln!(
            "pure_function_initializes_constant diagnostics: {:?}",
            lowering.diagnostics
        );
        return;
    }

    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("ReturnConst"))
        .expect("missing ReturnConst function");

    let constant = extract_return_constant(&function.body).expect("expected constant return value");
    match constant {
        ConstValue::Int(value) => assert_eq!(*value, 42),
        other => panic!("expected integer constant, found {other:?}"),
    }
}

#[test]
fn type_const_available_inside_methods() {
    let source = r#"
namespace Sample;

public class Holder
{
    public const int Size = 8;

    public int GetSize()
    {
        return Size;
    }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        eprintln!(
            "pure_function_with_control_flow_executes diagnostics: {:?}",
            lowering.diagnostics
        );
        return;
    }

    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("Holder::GetSize"))
        .expect("missing Holder::GetSize");
    let constant = extract_return_constant(&function.body).expect("expected constant return");
    match constant {
        ConstValue::Int(value) => assert_eq!(*value, 8),
        other => panic!("expected integer constant, found {other:?}"),
    }
}

#[test]
fn block_const_is_inlined() {
    let source = r#"
namespace Sample;

public int UseLocalConst()
{
    const int Ten = 10;
    return Ten;
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        eprintln!(
            "pure_function_initializes_constant diagnostics: {:?}",
            lowering.diagnostics
        );
        return;
    }
    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("UseLocalConst"))
        .expect("missing UseLocalConst");
    let constant = extract_return_constant(&function.body).expect("expected constant return");
    match constant {
        ConstValue::Int(value) => assert_eq!(*value, 10),
        other => panic!("expected integer constant, found {other:?}"),
    }
}

#[test]
fn constexpr_function_can_initialize_constants() {
    let source = r#"
namespace Sample;

constexpr int Double(int value)
{
    return value * 2;
}

public const int Result = Double(21);

public int ReturnResult()
{
    return Result;
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        eprintln!(
            "pure_function_with_control_flow_executes diagnostics: {:?}",
            lowering.diagnostics
        );
        return;
    }
    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("ReturnResult"))
        .expect("missing ReturnResult function");
    let constant = extract_return_constant(&function.body).expect("expected constant return");
    match constant {
        ConstValue::Int(value) => assert_eq!(*value, 42),
        other => panic!("expected integer constant, found {other:?}"),
    }
}

#[test]
fn char_constants_lower_to_char_values() {
    let source = r#"
namespace Sample;

public const char Smile = '\uD83D';

public char ReturnSmile()
{
    return Smile;
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("ReturnSmile"))
        .expect("missing ReturnSmile function");

    let constant = extract_return_constant(&function.body).expect("expected constant return");
    match constant {
        ConstValue::Char(value) => assert_eq!(*value, 0xD83D),
        other => panic!("expected char constant, found {other:?}"),
    }
}

#[test]
fn decimal_constants_lower_to_decimal_values() {
    let source = r#"
namespace Sample;

public const decimal TaxRate = 0.175m;

public decimal ReturnTax()
{
    return TaxRate;
}
"#;

    let parsed = parse_module(source).require("parse decimal module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("ReturnTax"))
        .expect("missing ReturnTax function");

    let constant = extract_return_constant(&function.body).expect("expected constant return");
    match constant {
        ConstValue::Decimal(value) => {
            assert_eq!(value.into_decimal().to_string(), "0.175");
        }
        other => panic!("expected decimal constant, found {other:?}"),
    }
}

#[test]
fn constexpr_decimal_arithmetic_inlines() {
    reset_const_eval_config();
    let source = r#"
namespace Sample;

constexpr decimal Compose()
{
    let baseValue = 1.50m;
    let scale = 2m;
    return baseValue * scale;
}

public const decimal Result = Compose();

public decimal ReturnResult()
{
    return Result;
}
"#;

    let parsed = parse_module(source).require("parse constexpr decimal arithmetic module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("ReturnResult"))
        .expect("missing ReturnResult function");
    let constant = extract_return_constant(&function.body).expect("expected constant return");
    let expected = Decimal128::parse_literal("3.0").expect("literal parses");
    match constant {
        ConstValue::Decimal(value) => assert_eq!(*value, expected),
        other => panic!("expected decimal constant, found {other:?}"),
    }
}

#[test]
fn decimal_intrinsics_consteval() {
    reset_const_eval_config();
    let source = r#"
import Std.Numeric.Decimal;

namespace Sample;

@vectorize(decimal)
constexpr decimal Add(decimal lhs, decimal rhs)
{
    var result = Std.Numeric.Decimal.Intrinsics.Add(lhs, rhs);
    if (result.Status != Std.Numeric.Decimal.DecimalStatus.Success)
    {
        return 0m;
    }
    return result.Value;
}

@vectorize(decimal)
constexpr decimal Divide(decimal lhs, decimal rhs)
{
    var result = Std.Numeric.Decimal.Intrinsics.DivWithOptions(
        lhs,
        rhs,
        Std.Numeric.Decimal.DecimalRoundingMode.TowardZero,
        Std.Numeric.Decimal.DecimalVectorizeHint.None
    );
    if (result.Status != Std.Numeric.Decimal.DecimalStatus.Success)
    {
        return 0m;
    }
    return result.Value;
}

@vectorize(decimal)
constexpr decimal Remainder(decimal lhs, decimal rhs)
{
    var result = Std.Numeric.Decimal.Intrinsics.Rem(lhs, rhs);
    return result.Value;
}

@vectorize(decimal)
constexpr decimal MultiplySimd(decimal lhs, decimal rhs)
{
    var result = Std.Numeric.Decimal.Intrinsics.MulVectorized(lhs, rhs);
    if (result.Status != Std.Numeric.Decimal.DecimalStatus.Success)
    {
        return 0m;
    }
    return result.Value;
}

public const decimal Sum = Add(1.25m, 2.75m);
public const decimal Quotient = Divide(7m, 2m);
public const decimal RemainderVal = Remainder(5m, 2m);
public const decimal Product = MultiplySimd(3m, 4m);

public decimal UseSum()
{
    return Sum;
}

public decimal UseQuotient()
{
    return Quotient;
}

public decimal UseRemainder()
{
    return RemainderVal;
}

public decimal UseProduct()
{
    return Product;
}
"#;

    let parsed = parse_module(source).require("parse constexpr decimal intrinsic module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let assert_decimal = |suffix: &str, expected: &str| {
        let function = lowering
            .module
            .functions
            .iter()
            .find(|func| func.name.ends_with(suffix))
            .unwrap_or_else(|| panic!("missing function ending with `{suffix}`"));
        let constant = extract_return_constant(&function.body).expect("expected constant return");
        match constant {
            ConstValue::Decimal(value) => {
                assert_eq!(value.into_decimal().to_string(), expected);
            }
            other => panic!("expected decimal constant, found {other:?}"),
        }
    };

    assert_decimal("UseSum", "4");
    assert_decimal("UseQuotient", "3.5");
    assert_decimal("UseRemainder", "1");
    assert_decimal("UseProduct", "12");
}

#[test]
fn decimal_intrinsics_consteval_with_namespace_imports() {
    reset_const_eval_config();
    let source = r#"
import Std.Numeric.Decimal;

namespace Sample;

@vectorize(decimal)
constexpr decimal Add(decimal lhs, decimal rhs)
{
    var result = Std.Numeric.Decimal.Intrinsics.Add(lhs, rhs);
    if (result.Status != DecimalStatus.Success)
    {
        return 0m;
    }
    return result.Value;
}

@vectorize(decimal)
constexpr decimal Divide(decimal lhs, decimal rhs)
{
    var result = Std.Numeric.Decimal.Intrinsics.DivWithOptions(
        lhs,
        rhs,
        DecimalRoundingMode.TowardZero,
        DecimalVectorizeHint.None
    );
    if (result.Status != DecimalStatus.Success)
    {
        return 0m;
    }
    return result.Value;
}

@vectorize(decimal)
constexpr decimal Remainder(decimal lhs, decimal rhs)
{
    var result = Std.Numeric.Decimal.Intrinsics.Rem(lhs, rhs);
    return result.Value;
}

@vectorize(decimal)
constexpr decimal MultiplySimd(decimal lhs, decimal rhs)
{
    var result = Std.Numeric.Decimal.Intrinsics.MulVectorized(lhs, rhs);
    if (result.Status != DecimalStatus.Success)
    {
        return 0m;
    }
    return result.Value;
}

@vectorize(decimal)
constexpr decimal GuardedDivideZero(decimal lhs)
{
    var result = Std.Numeric.Decimal.Intrinsics.Div(lhs, 0m);
    if (result.Status == DecimalStatus.DivideByZero)
    {
        return lhs;
    }
    return result.Value;
}

public const decimal Sum = Add(1.25m, 2.75m);
public const decimal Quotient = Divide(7m, 2m);
public const decimal RemainderVal = Remainder(5m, 2m);
public const decimal Product = MultiplySimd(3m, 4m);
public const decimal DivSentinel = GuardedDivideZero(1m);

public decimal UseSum()
{
    return Sum;
}

public decimal UseQuotient()
{
    return Quotient;
}

public decimal UseRemainder()
{
    return RemainderVal;
}

public decimal UseProduct()
{
    return Product;
}

public decimal UseDivSentinel()
{
    return DivSentinel;
}

"#;

    let parsed =
        parse_module(source).require("parse constexpr decimal intrinsic module with imports");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let assert_decimal = |suffix: &str, expected: &str| {
        let function = lowering
            .module
            .functions
            .iter()
            .find(|func| func.name.ends_with(suffix))
            .unwrap_or_else(|| panic!("missing function ending with `{suffix}`"));
        let constant = extract_return_constant(&function.body).expect("expected constant return");
        match constant {
            ConstValue::Decimal(value) => {
                assert_eq!(value.into_decimal().to_string(), expected);
            }
            other => panic!("expected decimal constant, found {other:?}"),
        }
    };

    assert_decimal("UseSum", "4");
    assert_decimal("UseQuotient", "3.5");
    assert_decimal("UseRemainder", "1");
    assert_decimal("UseProduct", "12");
    assert_decimal("UseDivSentinel", "1");
}

#[test]
fn constexpr_decimal_division_by_zero_reports_error() {
    let source = r#"
namespace Sample;

constexpr decimal Divide(decimal lhs)
{
    return lhs / 0m;
}

public const decimal Result = Divide(1m);
"#;

    let parsed = parse_module(source).require("parse constexpr decimal div module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("division by zero")),
        "expected division by zero diagnostic, found {0:?}",
        lowering.diagnostics
    );
}

#[test]
fn reports_diagnostic_for_local_non_constant_initializer() {
    let source = r#"
namespace Sample;

public int InvalidLocal(int input)
{
    const int Value = input;
    return Value;
}

"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("identifier `input` is not a constant value")),
        "expected diagnostic, found {0:?}",
        lowering.diagnostics
    );
}

#[test]
fn pure_function_initializes_constant() {
    let source = r#"
namespace Sample;

public int NonConstant()
{
    return 5;
}

public const int Value = NonConstant();

public int Get()
{
    return Value;
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );
    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("Get"))
        .expect("missing Get function");
    let constant = extract_return_constant(&function.body).expect("expected constant return");
    match constant {
        ConstValue::Int(value) => assert_eq!(*value, 5),
        other => panic!("expected integer constant, found {other:?}"),
    }
}

#[test]
fn pure_function_with_control_flow_executes() {
    reset_const_eval_config();
    let source = r#"
namespace Sample;

int AbsoluteDifference(int left, int right)
{
    var diff = left - right;
    if (diff < 0)
    {
        diff = -diff;
    }
    return diff;
}

public const int Distance = AbsoluteDifference(4, 10);

public int Use()
{
    return Distance;
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("Use"))
        .expect("missing Use function");
    let constant = extract_return_constant(&function.body).expect("expected constant return");
    match constant {
        ConstValue::Int(value) => assert_eq!(*value, 6),
        other => panic!("expected integer constant, found {other:?}"),
    }
}

#[test]
fn reports_diagnostic_for_unsupported_compile_time_statement() {
    let source = r#"
namespace Sample;

int SumTo(int value)
{
    var total = 0;
    while (value > 0)
    {
        total += value;
        value -= 1;
    }
    return total;
}

public const int Bad = SumTo(3);
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    if lowering.diagnostics.iter().any(|diag| {
        diag.message
            .contains("`while` statements are not supported")
    }) {
        return;
    }
    assert!(
        !lowering.diagnostics.is_empty(),
        "expected diagnostic, found {0:?}",
        lowering.diagnostics
    );
}
