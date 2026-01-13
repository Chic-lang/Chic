use super::common::RequireExt;
use super::*;
use crate::drop_glue::drop_type_identity;
use crate::mir::data::StatementKind as MirStatementKind;
use crate::mir::data::{ConstValue, MirFunction, Operand, Rvalue};

fn const_assigned_to_local(function: &MirFunction, local_index: usize) -> Option<ConstValue> {
    for block in &function.body.blocks {
        for statement in &block.statements {
            if let MirStatementKind::Assign { place, value } = &statement.kind {
                if place.local.0 == local_index && place.projection.is_empty() {
                    if let Rvalue::Use(Operand::Const(constant)) = value {
                        return Some(constant.value().clone());
                    }
                }
            }
        }
    }
    None
}

fn find_first_uint_constant(function: &MirFunction) -> Option<u128> {
    for block in &function.body.blocks {
        for statement in &block.statements {
            if let MirStatementKind::Assign { value, .. } = &statement.kind {
                if let Rvalue::Use(Operand::Const(constant)) = value {
                    if let ConstValue::UInt(bits) = constant.value() {
                        return Some(*bits);
                    }
                }
            }
        }
    }
    None
}

fn find_symbol_constant(function: &MirFunction) -> Option<String> {
    for block in &function.body.blocks {
        for statement in &block.statements {
            if let MirStatementKind::Assign { value, .. } = &statement.kind {
                if let Rvalue::Use(Operand::Const(constant)) = value {
                    if let ConstValue::Symbol(sym) = constant.value() {
                        return Some(sym.clone());
                    }
                }
            }
        }
    }
    None
}

#[test]
fn drop_glue_intrinsic_returns_symbol_for_droppable_type() {
    let source = r#"
namespace Sample;

public struct Holder
{
    public Vec<int> Data;
}

public void Acquire()
{
    let drop = __drop_glue_of<Holder>();
}
"#;

    let parsed = parse_module(source).require("parse drop glue module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let acquire_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Acquire"))
        .require("missing Sample::Acquire lowering");

    let symbol = find_symbol_constant(acquire_fn)
        .require("expected symbol constant assignment for drop local");

    assert_eq!(
        symbol, "__cl_drop__Sample__Holder",
        "unexpected drop glue symbol"
    );
}

#[test]
fn drop_glue_intrinsic_returns_null_for_trivial_type() {
    let source = r#"
namespace Sample;

public struct Plain
{
    public int Value;
}

public void Acquire()
{
    let drop = __drop_glue_of<Plain>();
}
"#;

    let parsed = parse_module(source).require("parse trivial drop module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let acquire_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Acquire"))
        .require("missing Sample::Acquire lowering");

    let constant = const_assigned_to_local(acquire_fn, 1)
        .require("expected constant assignment for drop local");

    match constant {
        ConstValue::Null => {}
        other => panic!("expected null constant for trivial drop, found {other:?}"),
    }
}

#[test]
fn drop_glue_intrinsic_requires_type_argument() {
    let source = r#"
namespace Sample;

public void Acquire()
{
    let drop = __drop_glue_of();
}
"#;

    let parsed = parse_module(source).require("parse missing type argument module");
    let lowering = lower_module(&parsed.module);

    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("requires a type argument")),
        "expected missing type argument diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn drop_glue_intrinsic_rejects_runtime_arguments() {
    let source = r#"
namespace Sample;

public void Acquire(nint value)
{
    let drop = __drop_glue_of<int>(value);
}
"#;

    let parsed = parse_module(source).require("parse runtime argument module");
    let lowering = lower_module(&parsed.module);

    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("does not accept runtime arguments")),
        "expected runtime argument diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn zero_init_intrinsic_lowers_to_statement() {
    let source = r#"
namespace Std.Memory
{
    public static class Intrinsics
    {
        public unsafe static void ZeroInit<T>(out T target) { }
        public unsafe static void ZeroInitRaw(*mut byte pointer, usize length) { }
    }
}

namespace Sample
{
    public struct Holder
    {
        public int Value;
    }

    public unsafe class Ops
    {
        public void Reset(out Holder slot)
        {
            Std.Memory.Intrinsics.ZeroInit(out slot);
        }

        public void ResetRaw(*mut byte ptr, usize len)
        {
            Std.Memory.Intrinsics.ZeroInitRaw(ptr, len);
        }
    }
}
"#;

    let parsed = parse_module(source).require("parse zero init module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics lowering zero init module: {:?}",
        lowering.diagnostics
    );

    let reset_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Ops::Reset"))
        .require("missing Sample::Ops::Reset lowering");
    let saw_zero_init = reset_fn.body.blocks.iter().any(|block| {
        block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, MirStatementKind::ZeroInit { .. }))
    });
    assert!(
        saw_zero_init,
        "expected ZeroInit statement in Sample::Ops::Reset body: {:?}",
        reset_fn.body
    );

    let raw_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Ops::ResetRaw"))
        .require("missing Sample::Ops::ResetRaw lowering");
    let saw_zero_init_raw = raw_fn.body.blocks.iter().any(|block| {
        block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, MirStatementKind::ZeroInitRaw { .. }))
    });
    assert!(
        saw_zero_init_raw,
        "expected ZeroInitRaw statement in Sample::Ops::ResetRaw body: {:?}",
        raw_fn.body
    );
}

#[test]
fn instance_zero_init_method_is_not_intrinsic() {
    let source = r#"
namespace Sample;

public struct Holder
{
    public void ZeroInit()
    {
        this.Value = 42;
    }

    public int Value;
}

public class Ops
{
    public void Reset()
    {
        var slot = new Holder();
        slot.ZeroInit();
    }
}
"#;

    let parsed = parse_module(source).require("parse instance zero init module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let reset_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Ops::Reset"))
        .require("missing Sample::Ops::Reset lowering");
    let mut saw_zero_init_call = false;
    for block in &reset_fn.body.blocks {
        if let Some(Terminator::Call { func, .. }) = &block.terminator {
            if let Operand::Const(constant) = func
                && let ConstValue::Symbol(name) = &constant.value
                && name.ends_with("Holder::ZeroInit")
            {
                saw_zero_init_call = true;
                break;
            }
        }
    }
    assert!(
        saw_zero_init_call,
        "expected `slot.ZeroInit()` to lower to a method call terminator"
    );
}

#[test]
fn type_id_intrinsic_returns_constant() {
    let source = r#"
namespace Sample;

public struct Plain
{
    public int Value;
}

public ulong Acquire()
{
    return __type_id_of<Plain>();
}
"#;

    let parsed = parse_module(source).require("parse type id module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let acquire_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Acquire"))
        .require("missing Sample::Acquire lowering");

    let value = find_first_uint_constant(acquire_fn)
        .require("expected constant assignment for type id local");

    let expected = u128::from(drop_type_identity("Sample::Plain"));
    assert_eq!(value, expected, "unexpected type identity value");
}
