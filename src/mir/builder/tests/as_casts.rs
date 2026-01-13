use super::*;
use crate::mir::AggregateKind;

fn lower_source(source: &str) -> LoweringResult {
    let parsed = parse_module(source).expect("module should parse");
    lower_module(&parsed.module)
}

fn find_function<'a>(lowering: &'a LoweringResult, name: &str) -> &'a MirFunction {
    lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == name)
        .unwrap_or_else(|| panic!("missing lowered function {name}"))
}

#[test]
fn lowers_as_cast_to_mir_with_metadata() {
    let source = r#"
namespace Numbers {
    public byte ToByte(ulong input) {
        return input as byte;
    }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("may truncate or wrap")),
        "expected truncation diagnostic, found {:?}",
        lowering.diagnostics
    );

    let func = find_function(&lowering, "Numbers::ToByte");
    let block = &func.body.blocks[0];
    let cast = block
        .statements
        .iter()
        .find_map(|stmt| match &stmt.kind {
            StatementKind::Assign { value, .. } => match value {
                Rvalue::Cast {
                    kind,
                    source,
                    target,
                    ..
                } => Some((kind, source, target)),
                _ => None,
            },
            _ => None,
        })
        .expect("expected cast rvalue");
    assert_eq!(*cast.0, CastKind::IntToInt);
    assert_eq!(cast.1, &Ty::named("ulong"));
    assert_eq!(cast.2, &Ty::named("byte"));
}

#[test]
fn reports_infallible_as_cast_guidance() {
    let source = r#"
namespace Numbers {
    public long ToLong(int input) {
        return input as long;
    }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("prefer `From`/`Into`")),
        "expected From/Into diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn lowers_pointer_to_int_cast() {
    let source = r#"
namespace Numbers {
    public uint PtrToInt(int* input) {
        return input as uint;
    }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("pointer cast using `as`")),
        "expected pointer cast diagnostic, found {:?}",
        lowering.diagnostics
    );
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("requires an `unsafe` block")),
        "expected unsafe block diagnostic, found {:?}",
        lowering.diagnostics
    );
    let func = find_function(&lowering, "Numbers::PtrToInt");
    let block = &func.body.blocks[0];
    let cast_kind = block
        .statements
        .iter()
        .find_map(|stmt| match &stmt.kind {
            StatementKind::Assign { value, .. } => match value {
                Rvalue::Cast { kind, .. } => Some(kind),
                _ => None,
            },
            _ => None,
        })
        .expect("expected cast rvalue");
    assert_eq!(*cast_kind, CastKind::PointerToInt);
}

#[test]
fn pointer_cast_inside_unsafe_block_is_allowed() {
    let source = r#"
namespace Numbers {
    public uint PtrToInt(int* input) {
        unsafe
        {
            return (uint)input;
        }
    }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("C-style pointer cast")),
        "expected pointer cast diagnostic, found {:?}",
        lowering.diagnostics
    );
    assert!(
        lowering
            .diagnostics
            .iter()
            .all(|diag| !diag.message.contains("requires an `unsafe` block")),
        "did not expect unsafe block diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn lowers_c_style_numeric_cast_to_mir() {
    let source = r#"
namespace Numbers {
    public byte ToByte(ulong input) {
        return (byte)input;
    }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering.diagnostics.iter().any(|diag| {
            diag.message.contains("C-style cast")
                && diag.message.contains("may truncate or wrap the value")
        }),
        "expected C-style cast truncation diagnostic, found {:?}",
        lowering.diagnostics
    );

    let func = find_function(&lowering, "Numbers::ToByte");
    let block = &func.body.blocks[0];
    let cast = block
        .statements
        .iter()
        .find_map(|stmt| match &stmt.kind {
            StatementKind::Assign { value, .. } => match value {
                Rvalue::Cast { kind, .. } => Some(kind),
                _ => None,
            },
            _ => None,
        })
        .expect("expected cast rvalue");
    assert_eq!(*cast, CastKind::IntToInt);
}

#[test]
fn c_style_user_defined_conversion_invokes_operator() {
    let source = r#"
namespace Numbers {
    public class Value { }

    public class Input {
        public static explicit operator Value(Input value) => new Value();
    }

    public Value Convert(Input value) {
        return (Value)value;
    }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let func = find_function(&lowering, "Numbers::Convert");
    let call_operand = func
        .body
        .blocks
        .iter()
        .filter_map(|block| block.terminator.as_ref())
        .find_map(|term| match term {
            Terminator::Call { func, .. } => Some(func),
            _ => None,
        })
        .expect("expected conversion call terminator");

    match call_operand {
        Operand::Pending(pending) => assert!(
            pending.repr.contains("op_Explicit_Value"),
            "expected explicit conversion pending operand, found {:?}",
            pending
        ),
        other => panic!("expected pending operand for conversion, found {other:?}"),
    }
}

#[test]
fn c_style_nullable_cast_promotes_value() {
    let source = r#"
namespace Numbers {
    public int? Promote(int value) {
        return (int?)value;
    }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let func = find_function(&lowering, "Numbers::Promote");
    let block = &func.body.blocks[0];
    let saw_nullable_aggregate = block.statements.iter().any(|stmt| match &stmt.kind {
        StatementKind::Assign { value, .. } => match value {
            Rvalue::Aggregate { kind, fields } => match (kind, fields.as_slice()) {
                (AggregateKind::Adt { name, .. }, [Operand::Const(constant), Operand::Copy(_)])
                    if name == "int?" =>
                {
                    matches!(constant.value, ConstValue::Bool(true))
                }
                _ => false,
            },
            _ => false,
        },
        _ => false,
    });
    assert!(
        saw_nullable_aggregate,
        "expected nullable aggregate construction for cast, found {:?}",
        block.statements
    );

    let ret_decl = func.body.locals.first().expect("return local should exist");
    match &ret_decl.ty {
        Ty::Nullable(inner) => {
            assert_eq!(
                inner.canonical_name(),
                "int",
                "expected nullable int return type"
            )
        }
        other => panic!("expected nullable return type, found {:?}", other),
    }
}

#[test]
fn reports_unsupported_cast_target() {
    let source = r#"
namespace Numbers {
    public string InvalidCast(int input) {
        return input as string;
    }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("no explicit conversion")),
        "expected unsupported cast diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn reports_invalid_c_style_cast_with_c_style_message() {
    let source = r#"
namespace Numbers {
    public string InvalidCast(int input) {
        return (string)input;
    }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("no C-style cast")),
        "expected C-style cast diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn flag_enum_numeric_casts_round_trip() {
    let source = r#"
namespace Flags {
    @flags
    public enum Mode { None = 0, Read = 1, Write = 2 }

    public int ToInt(Mode mode) { return (int)mode; }
    public Mode FromInt(int raw) { return (Mode)raw; }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let to_int = find_function(&lowering, "Flags::ToInt");
    let kind = to_int.body.blocks[0]
        .statements
        .iter()
        .find_map(|stmt| match &stmt.kind {
            StatementKind::Assign { value, .. } => match value {
                Rvalue::Cast { kind, .. } => Some(kind),
                _ => None,
            },
            _ => None,
        })
        .expect("expected cast rvalue in ToInt");
    assert_eq!(*kind, CastKind::IntToInt);

    let from_int = find_function(&lowering, "Flags::FromInt");
    let kind = from_int.body.blocks[0]
        .statements
        .iter()
        .find_map(|stmt| match &stmt.kind {
            StatementKind::Assign { value, .. } => match value {
                Rvalue::Cast { kind, .. } => Some(kind),
                _ => None,
            },
            _ => None,
        })
        .expect("expected cast rvalue in FromInt");
    assert_eq!(*kind, CastKind::IntToInt);
}

#[test]
fn payload_enum_numeric_cast_is_rejected() {
    let source = r#"
namespace Shapes {
    public enum Shape {
        Circle { public int Radius; },
        Square { public int Edge; }
    }

    public int BadCast(Shape shape) { return (int)shape; }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("payload")),
        "expected payload diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn class_upcasts_are_allowed() {
    let source = r#"
namespace Objects {
    public class Base { }
    public class Derived : Base { }

    public Base Upcast(Derived value) { return (Base)value; }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let func = find_function(&lowering, "Objects::Upcast");
    let kind = func.body.blocks[0]
        .statements
        .iter()
        .find_map(|stmt| match &stmt.kind {
            StatementKind::Assign { value, .. } => match value {
                Rvalue::Cast { kind, .. } => Some(kind),
                _ => None,
            },
            _ => None,
        })
        .expect("expected cast rvalue in Upcast");
    assert_eq!(*kind, CastKind::Unknown);
}

#[test]
fn class_downcasts_surface_diagnostics() {
    let source = r#"
namespace Objects {
    public class Base { }
    public class Derived : Base { }

    public Derived Downcast(Base value) { return (Derived)value; }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("downcasting")),
        "expected downcast diagnostic, found {:?}",
        lowering.diagnostics
    );
}
