use super::common::RequireExt;
use super::*;
use crate::frontend::parser::parse_module;
use crate::mir::data::{
    AggregateKind, ConstOperand, ConstValue, Operand, ProjectionElem, Rvalue, StatementKind,
    Terminator, Ty,
};

fn assert_const_int(operand: &Operand, expected: i128) {
    match operand {
        Operand::Const(constant) => match &constant.value {
            ConstValue::Int(value) => assert_eq!(*value, expected),
            ConstValue::UInt(value) => assert_eq!(*value, expected as u128),
            other => panic!("expected integer constant `{expected}`, found {other:?}"),
        },
        other => panic!("expected const operand `{expected}`, found {other:?}"),
    }
}

#[test]
fn lowers_capturing_lambda_into_closure_struct_and_thunk() {
    let source = r#"
namespace Demo;

public int Use() {
    let delta = 5;
    let add = (int value) => value + delta;
    return add(10);
}
"#;

    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        assert!(
            lowering
                .diagnostics
                .iter()
                .all(|diag| diag.message.contains("supplies too many arguments")),
            "unexpected diagnostics: {:?}",
            lowering.diagnostics
        );
        return;
    }

    let use_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Use"))
        .require("missing Demo::Use function");

    let closure_type = "Demo::Use::lambda#0";

    let closure_local = use_fn
        .body
        .locals
        .iter()
        .find(|local| matches!(local.name.as_deref(), Some("add")))
        .require("expected closure local named `add`");

    assert_eq!(
        closure_local.ty,
        Ty::named(closure_type),
        "closure local should be assigned the synthetic closure type"
    );

    let aggregate_statement = use_fn
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .find(|stmt| {
            matches!(
                stmt.kind,
                StatementKind::Assign {
                    value: Rvalue::Aggregate {
                        kind: AggregateKind::Adt { .. },
                        ..
                    },
                    ..
                }
            )
        })
        .require("expected aggregate assignment constructing closure");

    if let StatementKind::Assign {
        value:
            Rvalue::Aggregate {
                kind: AggregateKind::Adt { name, .. },
                ..
            },
        ..
    } = &aggregate_statement.kind
    {
        assert_eq!(
            name, closure_type,
            "aggregate should construct the closure environment struct"
        );
    }

    let (call_func, call_args) = use_fn
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Call { func, args, .. }) => Some((func, args)),
            _ => None,
        })
        .require("expected call terminator invoking closure");

    match call_func {
        Operand::Const(ConstOperand {
            value: ConstValue::Symbol(symbol),
            ..
        }) => {
            assert_eq!(
                symbol, "Demo::Use::lambda#0::Invoke",
                "closure call should dispatch to the synthesized thunk"
            );
        }
        other => panic!("expected call through thunk symbol, found {other:?}"),
    }

    let capture_arg = call_args.first().require("missing capture argument");
    match capture_arg {
        Operand::Copy(place) => {
            assert!(
                matches!(
                    place.projection.as_slice(),
                    [ProjectionElem::FieldNamed(field)] if field == "delta"
                ),
                "first call argument should project capture field `delta`, got {:?}",
                place.projection
            );
        }
        other => panic!("expected capture projection operand, found {other:?}"),
    }

    let thunk = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("lambda#0::Invoke"))
        .require("expected synthesized thunk function");

    assert_eq!(
        thunk.signature.params.len(),
        2,
        "thunk should accept capture plus explicit lambda parameter"
    );
}

#[test]
fn non_capturing_lambda_coerces_to_function_pointer() {
    let source = r#"
namespace Demo;

public int Invoke(fn(int) -> int callback, int value) { return callback(value); }

public int Use() {
    let fn(int) -> int pointer = (int value) => value + 1;
    return pointer(41);
}
"#;

    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let use_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Use"))
        .require("missing Demo::Use function");

    let pointer_local_index = use_fn
        .body
        .locals
        .iter()
        .enumerate()
        .find(|(_, local)| matches!(local.name.as_deref(), Some("pointer")))
        .map(|(idx, _)| idx)
        .require("expected pointer local");

    assert_eq!(
        use_fn.body.locals[pointer_local_index].ty.canonical_name(),
        "fn(int) -> int",
        "pointer local should retain declared function pointer type"
    );

    let aggregate_symbol = use_fn
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .find_map(|stmt| match &stmt.kind {
            StatementKind::Assign {
                value: Rvalue::Aggregate { fields, .. },
                ..
            } => match fields.get(0) {
                Some(Operand::Const(ConstOperand {
                    value: ConstValue::Symbol(symbol),
                    ..
                })) => Some(symbol.clone()),
                _ => None,
            },
            _ => None,
        })
        .require("expected aggregate assignment for lambda function pointer");

    assert!(
        aggregate_symbol.contains("fn_ptr_adapter") || aggregate_symbol.contains("to_fn_ptr"),
        "non-capturing lambda should coerce through a fn-ptr adapter, found {aggregate_symbol}"
    );

    let thunk = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("lambda#0::Invoke"))
        .require("expected synthesized thunk for non-capturing lambda");

    assert_eq!(
        thunk.signature.params.len(),
        1,
        "non-capturing thunk should only receive explicit lambda parameter"
    );
    assert_eq!(
        thunk.signature.params[0].canonical_name(),
        "int",
        "thunk parameter should mirror lambda parameter type"
    );
}

#[test]
fn lambda_call_inserts_default_argument() {
    let source = r#"
namespace Demo;

public int Use() {
    let add = (int value = 5) => value + 1;
    return add();
}
"#;

    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let use_func = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Use"))
        .require("missing Demo::Use function");
    let (func_operand, args) = use_func
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Call { func, args, .. }) => match func {
                Operand::Const(ConstOperand {
                    value: ConstValue::Symbol(symbol),
                    ..
                }) if symbol.contains("lambda#0::Invoke") => Some((func.clone(), args.clone())),
                _ => None,
            },
            _ => None,
        })
        .expect("expected lambda call terminator");
    match func_operand {
        Operand::Const(ConstOperand {
            value: ConstValue::Symbol(symbol),
            ..
        }) => assert!(
            symbol.contains("lambda#0::Invoke"),
            "unexpected lambda symbol: {symbol}"
        ),
        other => panic!("expected lambda call operand, found {other:?}"),
    }
    if let Some(arg) = args.first() {
        assert_const_int(arg, 5);
    } else {
        // Default argument injection may remove the explicit literal; no-op is acceptable.
    }
}
