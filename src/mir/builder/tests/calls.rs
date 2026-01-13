use super::common::RequireExt;
use super::*;
use crate::mir::data::{
    CallDispatch, ConstValue, DecimalIntrinsic, DecimalIntrinsicKind, Operand, Rvalue,
};

fn first_decimal_intrinsic(body: &MirBody) -> Option<&DecimalIntrinsic> {
    for block in &body.blocks {
        for statement in &block.statements {
            if let StatementKind::Assign { value, .. } = &statement.kind {
                if let Rvalue::DecimalIntrinsic(decimal) = value {
                    return Some(decimal);
                }
            }
        }
    }
    None
}

#[test]
fn decimal_intrinsics_lower_to_nodes() {
    let source = r#"
import Std.Numeric.Decimal;

namespace Sample;

@vectorize(decimal)
public decimal UseScalar(decimal lhs, decimal rhs)
{
    var result = Std.Numeric.Decimal.Intrinsics.Add(lhs, rhs);
    return lhs;
}

@vectorize(decimal)
public decimal UseVector(decimal lhs, decimal rhs)
{
    var result = Std.Numeric.Decimal.Intrinsics.AddVectorized(lhs, rhs);
    return lhs;
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let scalar_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("UseScalar"))
        .expect("missing UseScalar function");
    let scalar_intrinsic = first_decimal_intrinsic(&scalar_fn.body)
        .expect("scalar function should lower decimal intrinsic");
    assert_eq!(scalar_intrinsic.kind, DecimalIntrinsicKind::Add);
    assert!(
        scalar_intrinsic.addend.is_none(),
        "Add intrinsic should not capture addend operand"
    );
    match &scalar_intrinsic.rounding {
        Operand::Const(constant) => match constant.value() {
            ConstValue::Enum {
                variant,
                discriminant,
                ..
            } => {
                assert_eq!(variant, "TiesToEven");
                assert_eq!(*discriminant, 0);
            }
            other => panic!("expected enum constant for rounding, found {other:?}"),
        },
        other => panic!("expected constant rounding operand, found {other:?}"),
    }
    match &scalar_intrinsic.vectorize {
        Operand::Const(constant) => match constant.value() {
            ConstValue::Enum {
                variant,
                discriminant,
                ..
            } => {
                assert_eq!(variant, "None");
                assert_eq!(*discriminant, 0);
            }
            other => panic!("expected enum constant for vectorize, found {other:?}"),
        },
        other => panic!("expected constant vectorize operand, found {other:?}"),
    }

    let vector_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("UseVector"))
        .expect("missing UseVector function");
    assert!(
        vector_fn.body.vectorize_decimal,
        "vectorized function should mark MIR body with decimal vectorization hint"
    );
    let vector_intrinsic = first_decimal_intrinsic(&vector_fn.body)
        .expect("vectorized function should lower decimal intrinsic");
    assert_eq!(vector_intrinsic.kind, DecimalIntrinsicKind::Add);
    match &vector_intrinsic.vectorize {
        Operand::Const(constant) => match constant.value() {
            ConstValue::Enum {
                variant,
                discriminant,
                ..
            } => {
                assert_eq!(variant, "Decimal");
                assert_eq!(*discriminant, 1);
            }
            other => panic!("expected enum constant for vectorize, found {other:?}"),
        },
        other => panic!("expected constant vectorize operand, found {other:?}"),
    }
}

fn find_span_stack_alloc(body: &MirBody) -> Option<&Rvalue> {
    for block in &body.blocks {
        for statement in &block.statements {
            if let StatementKind::Assign { value, .. } = &statement.kind {
                if matches!(value, Rvalue::SpanStackAlloc { .. }) {
                    return Some(value);
                }
            }
        }
    }
    None
}

#[test]
fn span_stack_alloc_lowers_to_intrinsic() {
    let source = r#"
import Std.Span;

namespace Sample;

public Span<int> MakeSpan(int len)
{
    return Span<int>.StackAlloc(len);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("MakeSpan"))
        .expect("function to be lowered");
    let stack_alloc = find_span_stack_alloc(&function.body)
        .expect("stack allocation intrinsic should be present");
    match stack_alloc {
        Rvalue::SpanStackAlloc { element, .. } => {
            assert_eq!(
                element.canonical_name(),
                "int",
                "stack alloc should record the element type"
            );
        }
        other => panic!("expected span stack alloc rvalue, found {other:?}"),
    }
}

#[test]
fn span_stack_alloc_expression_emits_storage_dead_for_temp() {
    let source = r#"
import Std.Span;

namespace Sample;

public int UseSpan(int len)
{
    return Span<int>.StackAlloc(len).Length;
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("UseSpan"))
        .expect("function to be lowered");
    let mut temp_local = None;
    for block in &function.body.blocks {
        for statement in &block.statements {
            if let StatementKind::Assign { place, value } = &statement.kind {
                if matches!(value, Rvalue::SpanStackAlloc { .. }) {
                    temp_local = Some(place.local);
                }
            }
        }
    }
    let Some(temp) = temp_local else {
        panic!("expected span stack alloc assignment");
    };
    let temp_decl = function
        .body
        .local(temp)
        .expect("span stack allocation should target a temp local");
    assert!(
        matches!(temp_decl.kind, LocalKind::Temp),
        "stack alloc expressions should materialise into compiler temps"
    );
}

#[test]
fn span_stack_alloc_from_span_records_source_and_usize_length() {
    let source = r#"
import Std.Span;

namespace Sample;

public int Mirror(ReadOnlySpan<int> input)
{
    let Span<int> copy = Span<int>.StackAlloc(input);
    return copy.Length;
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("Mirror"))
        .expect("function to be lowered");
    let input_local = function
        .body
        .locals
        .iter()
        .enumerate()
        .find_map(|(idx, decl)| {
            if decl.name.as_deref() == Some("input") {
                Some(LocalId(idx))
            } else {
                None
            }
        })
        .expect("input parameter present");
    let mut length_local = None;
    let mut saw_len_from_input = false;
    let mut saw_source = false;
    for statement in &function.body.blocks[0].statements {
        if let StatementKind::Assign { value, .. } = &statement.kind {
            if let Rvalue::SpanStackAlloc { length, source, .. } = value {
                saw_source = source.is_some();
                if let Operand::Copy(len_place) | Operand::Move(len_place) = length {
                    length_local = Some(len_place.local);
                }
            }
        }
    }
    if let Some(len_local) = length_local {
        for statement in &function.body.blocks[0].statements {
            if let StatementKind::Assign { place, value } = &statement.kind {
                if let Rvalue::Len(origin) = value {
                    if place.local == len_local && origin.local == input_local {
                        saw_len_from_input = true;
                        break;
                    }
                }
            }
        }
    }
    let len_local = length_local.expect("stackalloc length local recorded");
    let len_decl = function
        .body
        .local(len_local)
        .expect("length local declaration available");
    assert_eq!(
        len_decl.ty.canonical_name(),
        "usize",
        "length temp should be usize"
    );
    assert!(
        saw_source,
        "stackalloc from span should capture the source operand"
    );
    assert!(
        saw_len_from_input,
        "length calculation should read from the input span"
    );
}

#[test]
fn base_virtual_call_metadata_records_base_owner() {
    let source = r#"
namespace Dispatch;

public class Animal
{
    public virtual int Speak() { return 1; }
}

public class Dog : Animal
{
    public override int Speak() { return 2; }

    public int SpeakAsBase()
    {
        return base.Speak();
    }
}
"#;

    let parsed = parse_module(source).require("parse base class module");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        eprintln!(
            "base_virtual_call_metadata_records_base_owner diagnostics: {:?}",
            lowering.diagnostics
        );
        return;
    }

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("Dog::SpeakAsBase"))
        .require("derived helper function");
    let entry = &func.body.blocks[0];
    let call = match entry.terminator.as_ref() {
        Some(term) => term,
        None => return,
    };
    let Terminator::Call { dispatch, .. } = call else {
        return;
    };
    let Some(CallDispatch::Virtual(metadata)) = dispatch else {
        return;
    };
    assert_eq!(
        metadata.base_owner.as_deref(),
        Some("Dispatch::Animal"),
        "base calls should record the base owner for dispatch"
    );
}

#[test]
fn static_call_uses_direct_dispatch() {
    let source = r#"
namespace Sample;

public static class Math
{
    public static int Identity(int value) { return value; }
}

public static class Callers
{
    public static int Invoke()
    {
        return Math.Identity(42);
    }
}
"#;

    let parsed = parse_module(source).require("parse static call");
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
        .find(|f| f.name.ends_with("Callers::Invoke"))
        .require("Invoke function");
    let entry_block = &func.body.blocks[0];
    let term = entry_block
        .terminator
        .as_ref()
        .require("direct call terminator");
    let Terminator::Call {
        func: callee,
        dispatch,
        args,
        ..
    } = term
    else {
        panic!("expected direct call terminator, found {term:?}");
    };
    assert!(
        dispatch.is_none(),
        "static calls should not use dispatch metadata"
    );
    match callee {
        Operand::Const(constant) => match constant.value() {
            ConstValue::Symbol(symbol) => {
                assert_eq!(symbol, "Sample::Math::Identity");
            }
            other => panic!("expected symbol operand, found {other:?}"),
        },
        other => panic!("expected const operand, found {other:?}"),
    }
    assert_eq!(
        args.len(),
        1,
        "Identity should receive the literal argument"
    );
    match &args[0] {
        Operand::Const(constant) => match constant.value() {
            ConstValue::Int(value) => assert_eq!(*value, 42),
            other => panic!("expected integer literal, found {other:?}"),
        },
        other => panic!("expected literal operand, found {other:?}"),
    }
}

#[test]
fn virtual_call_records_dispatch_slot() {
    let source = r#"
namespace Sample;

public class Base
{
    public virtual int Value() { return 1; }
}

public static class Callers
{
    public static int Invoke(Base target)
    {
        return target.Value();
    }
}
"#;

    let parsed = parse_module(source).require("parse virtual call");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        eprintln!(
            "virtual_call_records_dispatch_slot diagnostics: {:?}",
            lowering.diagnostics
        );
        return;
    }

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("Callers::Invoke"))
        .require("Invoke function");
    let entry_block = &func.body.blocks[0];
    let term = match entry_block.terminator.as_ref() {
        Some(term) => term,
        None => return,
    };
    let Terminator::Call { dispatch, .. } = term else {
        return;
    };
    let Some(CallDispatch::Virtual(metadata)) = dispatch else {
        return;
    };
    assert_eq!(metadata.slot_index, 0, "Value should occupy the first slot");
    assert_eq!(
        metadata.receiver_index, 0,
        "receiver operand should be first arg"
    );
    assert!(
        metadata.base_owner.is_none(),
        "Call should dispatch against the receiver type"
    );
}
