use std::fmt::Write;

use super::data::{BasicBlock, LocalKind, MirFunction, MirModule, StatementKind, Terminator};
use super::state::AsyncStateMachine;
use crate::mir::ParamMode;
use crate::typeck::{AutoTraitKind, BorrowEscapeCategory, ConstraintKind, TypeConstraint};

/// Render a MIR module into a human-readable string.
#[must_use]
pub fn format_module(module: &MirModule, constraints: &[TypeConstraint]) -> String {
    let mut out = String::new();
    for function in &module.functions {
        format_function(function, &mut out);
    }
    if !constraints.is_empty() {
        let _ = writeln!(out, "constraints:");
        for constraint in constraints {
            let span_text = constraint.span.map_or_else(
                || "-".into(),
                |span| format!("{}..{}", span.start, span.end),
            );
            let kind_text = format_constraint_kind(&constraint.kind);
            let _ = writeln!(out, "  [{span_text}] {kind_text}");
        }
    }
    out
}

fn format_function(function: &MirFunction, out: &mut String) {
    let params = format_parameters(function);
    let _ = writeln!(
        out,
        "fn {}({params}) -> {:?} {{",
        function.name, function.signature.ret
    );

    emit_locals(function, out);
    if let Some(machine) = &function.body.async_machine {
        emit_async_metadata(machine, out);
    }
    if !function.body.debug_notes.is_empty() {
        for note in &function.body.debug_notes {
            let span_text = note.span.map_or_else(
                || "-".into(),
                |span| format!("{}..{}", span.start, span.end),
            );
            let _ = writeln!(out, "    // note[{span_text}]: {}", note.message);
        }
    }
    for block in &function.body.blocks {
        format_block(block, out);
    }
    let _ = writeln!(out, "}}\n");
}

fn format_parameters(function: &MirFunction) -> String {
    let mut names: Vec<Option<String>> = vec![None; function.signature.params.len()];
    for local in &function.body.locals {
        if let LocalKind::Arg(arg_index) = local.kind {
            if arg_index < names.len() {
                names[arg_index] = local.name.clone();
            }
        }
    }

    function
        .signature
        .params
        .iter()
        .enumerate()
        .map(|(idx, ty)| {
            let name = names
                .get(idx)
                .and_then(|opt| opt.as_ref())
                .map_or_else(|| format!("_{}", idx + 1), Clone::clone);
            format!("{name}: {:?}", ty)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn emit_locals(function: &MirFunction, out: &mut String) {
    for (index, local) in function.body.locals.iter().enumerate() {
        let pinned_suffix = if local.is_pinned { ", pinned" } else { "" };
        let name_suffix = local
            .name
            .as_deref()
            .map(|name| format!(" name={name}"))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "    let _{index}: {:?} ({:?}{pinned_suffix}{name_suffix});",
            local.ty, local.kind
        );
    }
}

fn emit_async_metadata(machine: &AsyncStateMachine, out: &mut String) {
    if !machine.suspend_points.is_empty() {
        emit_suspend_points(machine, out);
    }
    if !machine.pinned_locals.is_empty() {
        emit_pinned_locals(machine, out);
    }
    if let Some(local) = machine.result_local {
        let _ = writeln!(out, "    // async result stored in {local}");
    }
}

fn emit_suspend_points(machine: &AsyncStateMachine, out: &mut String) {
    let _ = writeln!(out, "    // async suspend points");
    for point in &machine.suspend_points {
        let dest = point
            .destination
            .map_or_else(|| "None".into(), |local| format!("{local}"));
        let _ = writeln!(
            out,
            "    // state {}: await {} future {} -> resume {} drop {} dest {dest}",
            point.id, point.await_block, point.future, point.resume_block, point.drop_block
        );
    }
}

fn emit_pinned_locals(machine: &AsyncStateMachine, out: &mut String) {
    let pinned = machine
        .pinned_locals
        .iter()
        .map(|local| format!("{local}"))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(out, "    // pinned locals: {pinned}");
}

fn format_block(block: &BasicBlock, out: &mut String) {
    let block_span = block.span.map_or_else(
        || "[unknown]".into(),
        |span| format!("[{}..{}]", span.start, span.end),
    );
    let _ = writeln!(out, "  {} {block_span}:", block.id);
    for statement in &block.statements {
        let span_text = statement.span.map_or_else(
            || "[unknown]".into(),
            |span| format!("[{}..{}]", span.start, span.end),
        );
        match &statement.kind {
            StatementKind::Assign { place, value } => {
                let _ = writeln!(out, "    {span_text} {place:?} = {value:?};");
            }
            StatementKind::StorageLive(local) => {
                let _ = writeln!(out, "    {span_text} StorageLive({local});");
            }
            StatementKind::StorageDead(local) => {
                let _ = writeln!(out, "    {span_text} StorageDead({local});");
            }
            StatementKind::MarkFallibleHandled { local } => {
                let _ = writeln!(out, "    {span_text} FallibleHandled({local});");
            }
            other => {
                let _ = writeln!(out, "    {span_text} {other:?};");
            }
        }
    }
    if let Some(term) = &block.terminator {
        match term {
            Terminator::Return => {
                let _ = writeln!(out, "    return;");
            }
            other => {
                let _ = writeln!(out, "    {other:?};");
            }
        }
    }
}

fn format_constraint_kind(kind: &ConstraintKind) -> String {
    match kind {
        ConstraintKind::ParameterType {
            function,
            param,
            ty,
        } => {
            format!("param {function}.{param}: {ty}")
        }
        ConstraintKind::VariableInit {
            function,
            name,
            declared,
            expr,
        } => {
            if let Some(declared) = declared {
                format!("var {function}.{name}: {declared} = {expr}")
            } else {
                format!("var {function}.{name} = {expr}")
            }
        }
        ConstraintKind::ReturnType { function, ty } => {
            format!("return {function}: {ty}")
        }
        ConstraintKind::ImplTraitBound {
            function,
            opaque_ty,
            bound,
        } => {
            format!("opaque return {function}: {opaque_ty} implements {bound}")
        }
        ConstraintKind::ImplementsInterface {
            type_name,
            interface,
        } => {
            format!("{type_name} implements {interface}")
        }
        ConstraintKind::ExtensionTarget { extension, target } => {
            format!("extension {extension} targets {target}")
        }
        ConstraintKind::RequiresAutoTrait {
            function,
            target,
            ty,
            trait_kind,
            origin: _,
        } => {
            let trait_name = match trait_kind {
                AutoTraitKind::ThreadSafe => "ThreadSafe",
                AutoTraitKind::Shareable => "Shareable",
                AutoTraitKind::Copy => "Copy",
            };
            format!("{function}.{target} requires {trait_name} for {ty}")
        }
        ConstraintKind::ThreadingBackendAvailable {
            function,
            backend,
            call,
        } => format!("{function} requires threads on {backend} for {call}"),
        ConstraintKind::EffectEscape { function, effect } => {
            format!("{function} throws {effect}")
        }
        ConstraintKind::RandomDuplication { function } => {
            format!("{function} duplicates RNG handle")
        }
        ConstraintKind::BorrowEscape {
            function,
            parameter,
            parameter_mode,
            escape,
        } => {
            let mode = match parameter_mode {
                ParamMode::Value => "value",
                ParamMode::In => "in",
                ParamMode::Ref => "ref",
                ParamMode::Out => "out",
            };
            let escape_desc = match escape {
                BorrowEscapeCategory::Return => "return".to_string(),
                BorrowEscapeCategory::Store { target } => format!("store -> {target}"),
                BorrowEscapeCategory::Capture { closure } => {
                    format!("capture -> {closure}")
                }
            };
            format!("{function} {mode} {parameter} escapes via {escape_desc}")
        }
        ConstraintKind::RequiresTrait {
            function,
            ty,
            trait_name,
        } => format!("{function}: {ty} requires {trait_name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::diagnostics::Span;
    use crate::mir::AsyncFramePolicy;
    use crate::mir::data::{
        Abi, BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl,
        LocalId, LocalKind, MirBody, MirFunction, MirModule, Operand, Place, Rvalue, Statement,
        StatementKind, Terminator, Ty,
    };
    use crate::mir::layout::TypeLayoutTable;
    use crate::mir::state::{AsyncStateMachine, AsyncSuspendPoint};
    use crate::typeck::{ConstraintKind, TypeConstraint};

    #[test]
    fn renders_stable_snapshot_for_simple_module() {
        let mut body = MirBody::new(1, Some(Span::new(0, 42)));
        body.locals.push(LocalDecl::new(
            Some("ret".into()),
            Ty::Unit,
            false,
            None,
            LocalKind::Return,
        ));
        body.locals.push(LocalDecl::new(
            Some("param".into()),
            Ty::named("int"),
            true,
            None,
            LocalKind::Arg(0),
        ));
        body.locals.push(LocalDecl::new(
            Some("temp".into()),
            Ty::named("int"),
            true,
            None,
            LocalKind::Local,
        ));
        if let Some(local) = body.local_mut(LocalId(2)) {
            local.is_pinned = true;
        }

        let mut entry = BasicBlock::new(BlockId(0), Some(Span::new(5, 15)));
        entry.statements.push(Statement {
            span: Some(Span::new(6, 7)),
            kind: StatementKind::StorageLive(LocalId(1)),
        });
        entry.statements.push(Statement {
            span: Some(Span::new(8, 9)),
            kind: StatementKind::Assign {
                place: Place::new(LocalId(2)),
                value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(7)))),
            },
        });
        entry.terminator = Some(Terminator::Return);
        body.blocks.push(entry);

        body.async_machine = Some(AsyncStateMachine {
            suspend_points: vec![AsyncSuspendPoint {
                id: 0,
                await_block: BlockId(1),
                resume_block: BlockId(2),
                drop_block: BlockId(3),
                future: LocalId(1),
                destination: Some(LocalId(2)),
                span: Some(Span::new(12, 18)),
            }],
            pinned_locals: vec![LocalId(1), LocalId(2)],
            cross_locals: Vec::new(),
            frame_fields: Vec::new(),
            result_local: None,
            result_ty: None,
            context_local: None,
            policy: AsyncFramePolicy::default(),
        });

        let function = MirFunction {
            name: "Test::case".into(),
            kind: FunctionKind::Function,
            signature: FnSig {
                params: vec![Ty::named("int")],
                ret: Ty::Unit,
                abi: Abi::Chic,
                effects: Vec::new(),

                lends_to_return: None,

                variadic: false,
            },
            body,
            is_async: true,
            async_result: None,
            is_generator: false,
            span: None,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        };

        let type_layouts = TypeLayoutTable::default();
        let primitive_registry = type_layouts.primitive_registry.clone();
        let module = MirModule {
            functions: vec![function],
            test_cases: Vec::new(),
            statics: Vec::new(),
            type_layouts,
            primitive_registry,
            interned_strs: Vec::new(),
            exports: Vec::new(),
            attributes: crate::mir::module_metadata::ModuleAttributes::default(),
            trait_vtables: Vec::new(),
            class_vtables: Vec::new(),
            interface_defaults: Vec::new(),
            default_arguments: Vec::new(),
            type_variance: std::collections::HashMap::new(),
            async_plans: Vec::new(),
        };
        let constraints = vec![TypeConstraint::new(
            ConstraintKind::ReturnType {
                function: "Test::case".into(),
                ty: "int".into(),
            },
            Some(Span::new(50, 60)),
        )];

        let rendered = format_module(&module, &constraints);
        let expected = concat!(
            "fn Test::case(param: Named(\"int\")) -> Unit {\n",
            "    let _0: Unit (Return name=ret);\n",
            "    let _1: Named(\"int\") (Arg(0) name=param);\n",
            "    let _2: Named(\"int\") (Local, pinned name=temp);\n",
            "    // async suspend points\n",
            "    // state 0: await bb1 future _1 -> resume bb2 drop bb3 dest _2\n",
            "    // pinned locals: _1, _2\n",
            "  bb0 [5..15]:\n",
            "    [6..7] StorageLive(_1);\n",
            "    [8..9] Place { local: LocalId(2), projection: [] } = Use(Const(ConstOperand { value: Int(7), literal: None }));\n",
            "    return;\n",
            "}\n",
            "\n",
            "constraints:\n",
            "  [50..60] return Test::case: int\n"
        );
        assert_eq!(rendered, expected);
    }
}
