use super::symbol_index::SymbolIndex;
use super::*;
use crate::frontend::ast::{
    ConstructorInitTarget, ConstructorInitializer, ConstructorKind, PropertyAccessorKind,
};
use crate::mir::ConstValue;
use crate::mir::layout::FieldLayout;
use std::collections::HashMap;

pub(super) fn emit_constructor_initializer(
    builder: &mut BodyBuilder<'_>,
    initializer: &ConstructorInitializer,
    self_local: LocalId,
    class_name: &str,
) {
    builder.lower_constructor_initializer(initializer, self_local, class_name);
}

pub(super) fn check_constructor_field_initialization(
    body: &MirBody,
    self_local: LocalId,
    layout: &StructLayout,
    kind: ConstructorKind,
    initializer: Option<&ConstructorInitializer>,
    span: Option<Span>,
    symbol_index: &SymbolIndex,
) -> Vec<LoweringDiagnostic> {
    if matches!(kind, ConstructorKind::Convenience) {
        return Vec::new();
    }
    let delegates_to_self = initializer
        .map(|init| matches!(init.target, ConstructorInitTarget::SelfType))
        .unwrap_or(false);
    let track_required_only = layout.class.is_some();
    let tracked_fields: Vec<&FieldLayout> = layout
        .fields
        .iter()
        .filter(|field| !is_runtime_managed_field(field))
        .filter(|field| !track_required_only || field.is_required)
        .collect();
    let field_count = tracked_fields.len();
    if field_count == 0 {
        return Vec::new();
    }

    let mut field_index = HashMap::new();
    for (idx, field) in tracked_fields.iter().enumerate() {
        field_index.insert(field.name.clone(), idx);
    }

    let predecessors = compute_predecessors(body);
    let mut in_sets = vec![vec![false; field_count]; body.blocks.len()];
    let mut out_sets = vec![vec![false; field_count]; body.blocks.len()];
    let mut changed = true;

    while changed {
        changed = false;
        for block in &body.blocks {
            let idx = block.id.0;
            let new_in = intersect_predecessors(&predecessors[idx], &out_sets, field_count);
            if new_in != in_sets[idx] {
                in_sets[idx] = new_in;
                changed = true;
            }

            let mut new_out = in_sets[idx].clone();
            for stmt in &block.statements {
                if let MirStatementKind::Assign { place, .. } = &stmt.kind {
                    if place.local == self_local {
                        if place.projection.is_empty() {
                            new_out.iter_mut().for_each(|slot| *slot = true);
                        } else if let Some(field_idx) = resolve_field_index(place, &field_index) {
                            if field_idx < new_out.len() {
                                new_out[field_idx] = true;
                            }
                        }
                    }
                }
            }

            if let Some(term) = &block.terminator {
                if let Terminator::Call { func, args, .. } = term {
                    if let Some(field_idx) = property_field_write(
                        func,
                        args,
                        symbol_index,
                        self_local,
                        &field_index,
                        &layout.name,
                    ) {
                        if field_idx < new_out.len() {
                            new_out[field_idx] = true;
                        }
                    }
                }
            }

            if delegates_to_self {
                if let Some(term) = &block.terminator {
                    if is_self_delegating_initializer(term, self_local) {
                        new_out.iter_mut().for_each(|slot| *slot = true);
                    }
                }
            }

            if new_out != out_sets[idx] {
                out_sets[idx] = new_out;
                changed = true;
            }
        }
    }

    let mut diagnostics = Vec::new();
    let all_initialised = vec![true; field_count];
    for block in &body.blocks {
        if matches!(block.terminator, Some(Terminator::Return)) {
            let idx = block.id.0;
            if out_sets[idx] != all_initialised {
                let missing = collect_missing_fields(&out_sets[idx], &tracked_fields);
                if !missing.is_empty() {
                    let message = if missing.len() == 1 {
                        format!(
                            "constructor must assign field `{}` before returning",
                            missing[0]
                        )
                    } else {
                        format!(
                            "constructor must assign fields {} before returning",
                            missing.join(", ")
                        )
                    };
                    diagnostics.push(LoweringDiagnostic { message, span });
                }
            }
        }
    }

    diagnostics
}

fn property_field_write(
    func: &Operand,
    args: &[Operand],
    symbol_index: &SymbolIndex,
    self_local: LocalId,
    field_index: &HashMap<String, usize>,
    owner: &str,
) -> Option<usize> {
    let function_name = match func {
        Operand::Pending(pending) => pending.repr.as_str(),
        Operand::Const(constant) => match &constant.value {
            ConstValue::Symbol(name) => name.as_str(),
            _ => "",
        },
        _ => return None,
    };

    let accessor = symbol_index.property_accessor(function_name)?;
    if accessor.owner != owner {
        return None;
    }

    if !matches!(
        accessor.kind,
        PropertyAccessorKind::Set | PropertyAccessorKind::Init
    ) {
        return None;
    }

    let backing_field = accessor.backing_field.as_ref()?;
    if !call_receives_self(args, self_local) {
        return None;
    }

    field_index.get(backing_field).copied()
}

fn call_receives_self(args: &[Operand], self_local: LocalId) -> bool {
    let Some(first) = args.first() else {
        return false;
    };
    match first {
        Operand::Copy(place) | Operand::Move(place) => place.local == self_local,
        Operand::Borrow(borrow) => borrow.place.local == self_local,
        _ => false,
    }
}

fn compute_predecessors(body: &MirBody) -> Vec<Vec<usize>> {
    let mut preds = vec![Vec::new(); body.blocks.len()];
    for block in &body.blocks {
        let from = block.id.0;
        if let Some(term) = &block.terminator {
            let mut push_edge = |target: BlockId| {
                if target.0 < preds.len() {
                    preds[target.0].push(from);
                }
            };
            match term {
                Terminator::Goto { target } => push_edge(*target),
                Terminator::SwitchInt {
                    targets, otherwise, ..
                } => {
                    for (_, target) in targets {
                        push_edge(*target);
                    }
                    push_edge(*otherwise);
                }
                Terminator::Match {
                    arms, otherwise, ..
                } => {
                    for arm in arms {
                        push_edge(arm.target);
                    }
                    push_edge(*otherwise);
                }
                Terminator::Call { target, unwind, .. } => {
                    push_edge(*target);
                    if let Some(unwind) = unwind {
                        push_edge(*unwind);
                    }
                }
                Terminator::Yield { resume, drop, .. } => {
                    push_edge(*resume);
                    push_edge(*drop);
                }
                Terminator::Await { resume, drop, .. } => {
                    push_edge(*resume);
                    push_edge(*drop);
                }
                Terminator::Throw { .. } => {}
                Terminator::Panic | Terminator::Return | Terminator::Unreachable => {}
                Terminator::Pending(_) => {}
            }
        }
    }
    preds
}

fn is_self_delegating_initializer(terminator: &Terminator, self_local: LocalId) -> bool {
    let Terminator::Call { func, args, .. } = terminator else {
        return false;
    };
    let Some(first_arg) = args.first() else {
        return false;
    };
    let is_self_arg = matches!(first_arg,
        Operand::Copy(place) | Operand::Move(place) if place.local == self_local
    );
    if !is_self_arg {
        return false;
    }
    match func {
        Operand::Pending(PendingOperand { repr, .. }) => repr.ends_with("::init#self"),
        _ => false,
    }
}

fn intersect_predecessors(
    predecessors: &[usize],
    out_sets: &[Vec<bool>],
    field_count: usize,
) -> Vec<bool> {
    if predecessors.is_empty() {
        return vec![false; field_count];
    }
    let mut iter = predecessors.iter();
    let first = *iter.next().expect("predecessor list should not be empty");
    let mut result = out_sets[first].clone();
    for pred in iter {
        let set = &out_sets[*pred];
        for (slot, value) in result.iter_mut().zip(set.iter()) {
            *slot &= *value;
        }
    }
    result
}

fn resolve_field_index(place: &Place, field_index: &HashMap<String, usize>) -> Option<usize> {
    for elem in &place.projection {
        match elem {
            ProjectionElem::FieldNamed(name) => return field_index.get(name).copied(),
            ProjectionElem::Field(idx) => return Some((*idx) as usize),
            ProjectionElem::UnionField { index, name } => {
                if let Some(idx) = field_index.get(name) {
                    return Some(*idx);
                }
                return Some((*index) as usize);
            }
            _ => {}
        }
    }
    None
}

fn collect_missing_fields(state: &[bool], fields: &[&FieldLayout]) -> Vec<String> {
    let required: Vec<(usize, &FieldLayout)> = fields
        .iter()
        .enumerate()
        .filter_map(|(idx, field)| field.is_required.then_some((idx, *field)))
        .collect();

    let candidates: Box<dyn Iterator<Item = (usize, &FieldLayout)>> = if required.is_empty() {
        Box::new(fields.iter().enumerate().map(|(idx, field)| (idx, *field)))
    } else {
        Box::new(required.into_iter())
    };

    candidates
        .filter_map(|(idx, field)| {
            (!state[idx]).then(|| {
                field
                    .display_name
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| field.name.clone())
            })
        })
        .collect()
}

fn is_runtime_managed_field(field: &FieldLayout) -> bool {
    field.name == "$vtable"
}
