use super::common::RequireExt;
use super::*;
use crate::frontend::parser::parse_module;
use crate::mir::data::{ConstValue, Operand, Ty};

#[test]
fn assigns_named_function_pointer() {
    let source = r#"
namespace Callbacks;

public int Add(int x, int y) { return x + y; }

public int Invoke(fn(int, int) -> int func, int x, int y) {
    return func(x, y);
}

public int Use() {
    let fn(int, int) -> int pointer = Add;
    return Invoke(pointer, 1, 2);
}
"#;
    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let use_function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Use"))
        .require("missing Use function lowering");

    // Local 1 should correspond to `pointer`.
    let pointer_local = use_function
        .body
        .locals
        .get(1)
        .require("expected pointer local");
    if std::env::var("CHIC_DEBUG_FN_PTR").is_ok() {
        eprintln!("{:#?}", use_function.body);
    }
    match &pointer_local.ty {
        Ty::Fn(fn_ty) => assert_eq!(
            fn_ty.canonical_name(),
            "fn(int, int) -> int",
            "unexpected function pointer signature"
        ),
        other => panic!("expected function pointer local, found {other:?}"),
    }

    // The initializer should materialize an aggregate containing the invoke pointer.
    assert!(
        use_function
            .body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .any(|stmt| {
                matches!(
                    stmt.kind,
                    MirStatementKind::Assign {
                        value: Rvalue::Aggregate { .. },
                        ..
                    }
                )
            }),
        "expected aggregate assignment for function pointer"
    );

    // The call to Invoke should pass the pointer by value.
    let call_args = use_function
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Call {
                args, arg_modes: _, ..
            }) => Some(args),
            _ => None,
        })
        .require("expected call terminator in Use");
    let pointer_arg = call_args.first().require("missing pointer argument");
    match pointer_arg {
        Operand::Copy(place) | Operand::Move(place) => {
            let arg_local = place.local.0;
            let arg_ty = use_function
                .body
                .locals
                .get(arg_local)
                .map(|local| local.ty.clone())
                .require("call argument should reference a local");
            match arg_ty {
                Ty::Fn(fn_ty) => assert_eq!(
                    fn_ty.canonical_name(),
                    pointer_local.ty.canonical_name(),
                    "pointer argument should carry the function pointer type"
                ),
                other => panic!("expected function pointer argument, found {other:?}"),
            }
        }
        other => panic!("expected pointer argument to be a local copy, found {other:?}"),
    }
}

#[test]
fn assigns_extern_function_pointer() {
    let source = r#"
namespace Callbacks;

@extern("C")
private static extern int abs_extern(int value);

public int UseExtern() {
    unsafe {
        let fn @extern("C")(int) -> int pointer = abs_extern;
        return pointer(-4);
    }
}
"#;
    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let use_function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::UseExtern"))
        .require("missing UseExtern function lowering");

    let pointer_local = use_function
        .body
        .locals
        .get(1)
        .require("expected pointer local");
    match &pointer_local.ty {
        Ty::Fn(fn_ty) => {
            assert_eq!(
                fn_ty.canonical_name(),
                "fn @extern(\"C\")(int) -> int",
                "unexpected extern function pointer signature"
            );
        }
        other => panic!("expected function pointer local, found {other:?}"),
    }

    let has_symbol_assign = use_function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .any(|stmt| match &stmt.kind {
            MirStatementKind::Assign {
                place,
                value: Rvalue::Use(Operand::Const(constant)),
            } if place.local.0 == 1 => matches!(constant.value, ConstValue::Symbol(_)),
            _ => false,
        });
    assert!(
        has_symbol_assign,
        "extern function pointer should be initialised from a symbol constant"
    );

    assert!(
        !use_function
            .body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .any(|stmt| matches!(
                stmt.kind,
                MirStatementKind::Assign {
                    value: Rvalue::Aggregate { .. },
                    ..
                }
            )),
        "extern function pointer should be thin (no aggregate assignment)"
    );
}

#[test]
fn selects_overloaded_function_by_signature() {
    let source = r#"
namespace Callbacks;

public int Add(int x, int y) { return x + y; }
public double Add(double value) { return value; }

public int Use() {
    let fn(int, int) -> int pointer = Add;
    return pointer(5, 7);
}
"#;
    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let use_function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Use"))
        .require("missing Use function lowering");

    assert!(
        use_function
            .body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .any(|stmt| {
                matches!(
                    stmt.kind,
                    MirStatementKind::Assign {
                        value: Rvalue::Aggregate { .. },
                        ..
                    }
                )
            }),
        "expected aggregate assignment for overload resolution"
    );
}

#[test]
fn reports_missing_matching_overload_for_fn_pointer() {
    let source = r#"
namespace Callbacks;

public double Add(double value) { return value; }
public double Add(double lhs, double rhs) { return lhs + rhs; }

public int Use() {
    let fn(int, int) -> int pointer = Add;
    return 0;
}
"#;
    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("does not have an overload matching")),
        "expected unresolved overload diagnostic, found: {:?}",
        lowering.diagnostics
    );
}
