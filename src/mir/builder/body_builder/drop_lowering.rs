use super::*;
use crate::mir::casts::pointer_depth;
use crate::mir::{GenericArg, Statement, StatementKind};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
struct ActiveDrop {
    local: LocalId,
    place: Place,
    span: Option<Span>,
}

struct DropLowering<'a> {
    layouts: &'a TypeLayoutTable,
    needs_drop_cache: HashMap<String, bool>,
    scheduled: HashSet<LocalId>,
}

impl<'a> DropLowering<'a> {
    fn new(layouts: &'a TypeLayoutTable) -> Self {
        Self {
            layouts,
            needs_drop_cache: HashMap::new(),
            scheduled: HashSet::new(),
        }
    }

    fn record_moves_from_operand(&self, operand: &Operand, moved: &mut HashSet<LocalId>) {
        if let Operand::Move(place) = operand {
            moved.insert(place.local);
        }
    }

    fn record_moves_from_rvalue(&self, value: &Rvalue, moved: &mut HashSet<LocalId>) {
        match value {
            Rvalue::Use(operand) | Rvalue::Unary { operand, .. } | Rvalue::Cast { operand, .. } => {
                self.record_moves_from_operand(operand, moved)
            }
            Rvalue::Binary { lhs, rhs, .. } => {
                self.record_moves_from_operand(lhs, moved);
                self.record_moves_from_operand(rhs, moved);
            }
            Rvalue::Aggregate { fields, .. } => {
                for operand in fields {
                    self.record_moves_from_operand(operand, moved);
                }
            }
            Rvalue::SpanStackAlloc { length, source, .. } => {
                self.record_moves_from_operand(length, moved);
                if let Some(source) = source {
                    self.record_moves_from_operand(source, moved);
                }
            }
            Rvalue::StringInterpolate { segments } => {
                for segment in segments {
                    if let InterpolatedStringSegment::Expr { operand, .. } = segment {
                        self.record_moves_from_operand(operand, moved);
                    }
                }
            }
            Rvalue::NumericIntrinsic(intrinsic) => {
                for operand in &intrinsic.operands {
                    self.record_moves_from_operand(operand, moved);
                }
                if let Some(operand) = &intrinsic.out {
                    moved.insert(operand.local);
                }
            }
            Rvalue::DecimalIntrinsic(intrinsic) => {
                self.record_moves_from_operand(&intrinsic.lhs, moved);
                self.record_moves_from_operand(&intrinsic.rhs, moved);
                if let Some(addend) = &intrinsic.addend {
                    self.record_moves_from_operand(addend, moved);
                }
                self.record_moves_from_operand(&intrinsic.rounding, moved);
                self.record_moves_from_operand(&intrinsic.vectorize, moved);
            }
            Rvalue::AtomicRmw { value, .. } => self.record_moves_from_operand(value, moved),
            Rvalue::AtomicCompareExchange {
                expected, desired, ..
            } => {
                self.record_moves_from_operand(expected, moved);
                self.record_moves_from_operand(desired, moved);
            }
            Rvalue::AddressOf { .. }
            | Rvalue::Len(_)
            | Rvalue::AtomicLoad { .. }
            | Rvalue::StaticLoad { .. }
            | Rvalue::StaticRef { .. }
            | Rvalue::Pending(_) => {}
        }
    }

    fn process_block_with_entry(
        &mut self,
        block: &mut BasicBlock,
        locals: &[LocalDecl],
        moved_out_entry: &HashSet<LocalId>,
    ) {
        let mut active: Vec<ActiveDrop> = Vec::new();
        let mut rewritten: Vec<Statement> = Vec::with_capacity(block.statements.len());
        let mut moved_out: HashSet<LocalId> = moved_out_entry.clone();

        for statement in std::mem::take(&mut block.statements) {
            if let StatementKind::Assign { place, value } = &statement.kind {
                if place.projection.is_empty() {
                    moved_out.remove(&place.local);
                }
                self.record_moves_from_rvalue(value, &mut moved_out);
            }
            match statement.kind {
                StatementKind::DeferDrop { place } => {
                    let local = place.local;
                    active.push(ActiveDrop {
                        local,
                        place,
                        span: statement.span,
                    });
                    self.scheduled.insert(local);
                }
                StatementKind::StorageDead(local) => {
                    let moved = moved_out.contains(&local);
                    let mut handled_active = false;
                    for entry in active
                        .iter()
                        .filter(|candidate| candidate.local == local)
                        .cloned()
                    {
                        handled_active = true;
                        if moved {
                            continue;
                        }
                        if let Some(ty) = self.place_ty(&entry.place, locals) {
                            let drop_needed = self.ty_needs_drop(&ty);
                            let force = !drop_needed && self.scheduled.contains(&entry.place.local);
                            if drop_needed || force {
                                let mut drops = self.build_drop_sequence(
                                    entry.place.clone(),
                                    ty,
                                    entry.span.or(block.span),
                                    block.id,
                                    force,
                                );
                                rewritten.append(&mut drops);
                            }
                        } else if self.scheduled.contains(&entry.place.local) {
                            let mut drops = self.build_drop_sequence(
                                entry.place.clone(),
                                Ty::Unknown,
                                entry.span.or(block.span),
                                block.id,
                                true,
                            );
                            rewritten.append(&mut drops);
                        }
                    }
                    if let Some(ty) = locals.get(local.0).map(|decl| decl.ty.clone()) {
                        let drop_needed = self.ty_needs_drop(&ty);
                        let force = !drop_needed && self.scheduled.contains(&local);
                        if (drop_needed || force) && !handled_active && !moved {
                            let place = Place::new(local);
                            let mut drops = self.build_drop_sequence(
                                place,
                                ty,
                                statement.span,
                                block.id,
                                force,
                            );
                            rewritten.append(&mut drops);
                        }
                    } else if self.scheduled.contains(&local) {
                        if moved {
                            active.retain(|entry| entry.local != local);
                            rewritten.push(statement);
                            continue;
                        }
                        let place = Place::new(local);
                        let mut drops = self.build_drop_sequence(
                            place,
                            Ty::Unknown,
                            statement.span,
                            block.id,
                            true,
                        );
                        rewritten.append(&mut drops);
                    }
                    active.retain(|entry| entry.local != local);
                    rewritten.push(statement);
                }
                _ => rewritten.push(statement),
            }
        }

        block.statements = rewritten;

        if Self::terminator_requires_drop(block.terminator.as_ref()) {
            for entry in active.into_iter().rev() {
                if moved_out.contains(&entry.local) {
                    continue;
                }
                if let Some(ty) = self.place_ty(&entry.place, locals) {
                    let drop_needed = self.ty_needs_drop(&ty);
                    let force = !drop_needed && self.scheduled.contains(&entry.place.local);
                    if drop_needed || force {
                        let mut drops = self.build_drop_sequence(
                            entry.place.clone(),
                            ty,
                            entry.span.or(block.span),
                            block.id,
                            force,
                        );
                        block.statements.append(&mut drops);
                    }
                } else if self.scheduled.contains(&entry.place.local) {
                    let mut drops = self.build_drop_sequence(
                        entry.place.clone(),
                        Ty::Unknown,
                        entry.span.or(block.span),
                        block.id,
                        true,
                    );
                    block.statements.append(&mut drops);
                }
            }
        }
    }

    #[cfg(test)]
    fn process_block(&mut self, block: &mut BasicBlock, locals: &[LocalDecl]) {
        let empty = HashSet::new();
        self.process_block_with_entry(block, locals, &empty);
    }

    fn record_moves_from_terminator(
        &self,
        terminator: Option<&Terminator>,
        moved: &mut HashSet<LocalId>,
    ) {
        let Some(terminator) = terminator else {
            return;
        };
        match terminator {
            Terminator::Goto { .. }
            | Terminator::Return
            | Terminator::Panic
            | Terminator::Unreachable
            | Terminator::Pending(_) => {}
            Terminator::Throw { exception, .. } => {
                if let Some(exception) = exception {
                    self.record_moves_from_operand(exception, moved);
                }
            }
            Terminator::SwitchInt { discr, .. } => self.record_moves_from_operand(discr, moved),
            Terminator::Match { .. } => {}
            Terminator::Call {
                func,
                args,
                destination,
                ..
            } => {
                self.record_moves_from_operand(func, moved);
                for operand in args {
                    self.record_moves_from_operand(operand, moved);
                }
                if let Some(dest) = destination
                    && dest.projection.is_empty()
                {
                    moved.remove(&dest.local);
                }
            }
            Terminator::Yield { value, .. } => self.record_moves_from_operand(value, moved),
            Terminator::Await {
                future,
                destination,
                ..
            } => {
                let _ = future;
                if let Some(dest) = destination
                    && dest.projection.is_empty()
                {
                    moved.remove(&dest.local);
                }
            }
        }
    }

    fn moved_out_exit(
        &self,
        block: &BasicBlock,
        moved_out_entry: &HashSet<LocalId>,
    ) -> HashSet<LocalId> {
        let mut moved_out = moved_out_entry.clone();
        for statement in &block.statements {
            if let StatementKind::Assign { place, value } = &statement.kind {
                if place.projection.is_empty() {
                    moved_out.remove(&place.local);
                }
                self.record_moves_from_rvalue(value, &mut moved_out);
            }
        }
        self.record_moves_from_terminator(block.terminator.as_ref(), &mut moved_out);
        moved_out
    }

    fn successors(terminator: Option<&Terminator>) -> Vec<BlockId> {
        let Some(terminator) = terminator else {
            return Vec::new();
        };
        match terminator {
            Terminator::Goto { target } => vec![*target],
            Terminator::SwitchInt {
                targets, otherwise, ..
            } => {
                let mut out = targets.iter().map(|(_, id)| *id).collect::<Vec<_>>();
                out.push(*otherwise);
                out
            }
            Terminator::Match {
                arms, otherwise, ..
            } => {
                let mut out = arms.iter().map(|arm| arm.target).collect::<Vec<_>>();
                out.push(*otherwise);
                out
            }
            Terminator::Call { target, unwind, .. } => {
                let mut out = vec![*target];
                if let Some(unwind) = unwind {
                    out.push(*unwind);
                }
                out
            }
            Terminator::Yield { resume, drop, .. } | Terminator::Await { resume, drop, .. } => {
                vec![*resume, *drop]
            }
            Terminator::Return
            | Terminator::Throw { .. }
            | Terminator::Panic
            | Terminator::Unreachable
            | Terminator::Pending(_) => Vec::new(),
        }
    }

    fn process_blocks(&mut self, blocks: &mut [BasicBlock], locals: &[LocalDecl]) {
        if blocks.is_empty() {
            return;
        }
        let mut moved_out_entry: Vec<HashSet<LocalId>> = vec![HashSet::new(); blocks.len()];
        let mut changed = true;
        let max_passes = blocks.len().max(1) * 4;
        for _ in 0..max_passes {
            if !changed {
                break;
            }
            changed = false;
            for block in blocks.iter() {
                let idx = block.id.0;
                if idx >= moved_out_entry.len() {
                    continue;
                }
                let exit = self.moved_out_exit(block, &moved_out_entry[idx]);
                for succ in Self::successors(block.terminator.as_ref()) {
                    if succ.0 >= moved_out_entry.len() {
                        continue;
                    }
                    let before = moved_out_entry[succ.0].len();
                    moved_out_entry[succ.0].extend(exit.iter().copied());
                    if moved_out_entry[succ.0].len() != before {
                        changed = true;
                    }
                }
            }
        }

        for block in blocks.iter_mut() {
            let idx = block.id.0;
            let entry = moved_out_entry.get(idx).unwrap_or(&HashSet::new()).clone();
            self.process_block_with_entry(block, locals, &entry);
        }
    }

    fn terminator_requires_drop(terminator: Option<&Terminator>) -> bool {
        matches!(
            terminator,
            Some(Terminator::Return | Terminator::Throw { .. } | Terminator::Panic)
        )
    }

    fn place_ty(&self, place: &Place, locals: &[LocalDecl]) -> Option<Ty> {
        let mut current = locals.get(place.local.0)?.ty.clone();
        for projection in &place.projection {
            match projection {
                ProjectionElem::Field(index) => match &current {
                    Ty::Named(name) => {
                        let layout = self.layout_for(name.as_str())?;
                        let field = layout.fields.iter().find(|field| field.index == *index)?;
                        current = field.ty.clone();
                    }
                    Ty::Tuple(tuple) => {
                        let idx = usize::try_from(*index).ok()?;
                        current = tuple.elements.get(idx)?.clone();
                    }
                    _ => return None,
                },
                ProjectionElem::Deref => {
                    current = self.deref_ty(&current)?;
                }
                _ => return None,
            }
        }
        Some(current)
    }

    fn deref_ty(&self, ty: &Ty) -> Option<Ty> {
        match ty {
            Ty::Named(name) => {
                let trimmed = name.trim_end();
                let without_star = trimmed.strip_suffix('*')?;
                let base = without_star.trim_end();
                if base.is_empty() {
                    None
                } else {
                    Some(Ty::named(base.to_string()))
                }
            }
            Ty::Nullable(inner) => self.deref_ty(inner),
            _ => None,
        }
    }

    fn build_drop_sequence(
        &mut self,
        place: Place,
        ty: Ty,
        span: Option<Span>,
        block_id: BlockId,
        force: bool,
    ) -> Vec<Statement> {
        if !force && !self.ty_needs_drop(&ty) {
            return Vec::new();
        }

        if self.maybe_uninit_inner(&ty).is_some() {
            return vec![Statement {
                span,
                kind: StatementKind::Drop {
                    place,
                    target: block_id,
                    unwind: None,
                },
            }];
        }

        let mut statements = Vec::new();

        if !force && self.has_dispose(&ty) {
            statements.push(Statement {
                span,
                kind: StatementKind::Deinit(place.clone()),
            });
        }

        match &ty {
            Ty::Named(name) if !force => {
                if let Some(layout) = self.layout_for(name.as_str()) {
                    let mut view_fields: Vec<&FieldLayout> = Vec::new();
                    let mut other_fields: Vec<&FieldLayout> = Vec::new();
                    for field in layout.fields.iter().rev() {
                        if field.view_of.is_some() {
                            view_fields.push(field);
                        } else {
                            other_fields.push(field);
                        }
                    }
                    for field in view_fields.into_iter().chain(other_fields.into_iter()) {
                        if self.ty_needs_drop(&field.ty) {
                            let mut field_place = place.clone();
                            field_place
                                .projection
                                .push(ProjectionElem::Field(field.index));
                            let mut nested = self.build_drop_sequence(
                                field_place,
                                field.ty.clone(),
                                span,
                                block_id,
                                false,
                            );
                            statements.append(&mut nested);
                        }
                    }
                }
            }
            Ty::Tuple(tuple) if !force => {
                for (index, element) in tuple.elements.iter().enumerate().rev() {
                    if self.ty_needs_drop(element) {
                        let mut element_place = place.clone();
                        element_place
                            .projection
                            .push(ProjectionElem::Field(index as u32));
                        let mut nested = self.build_drop_sequence(
                            element_place,
                            element.clone(),
                            span,
                            block_id,
                            false,
                        );
                        statements.append(&mut nested);
                    }
                }
            }
            _ => {}
        }

        statements.push(Statement {
            span,
            kind: StatementKind::Drop {
                place,
                target: block_id,
                unwind: None,
            },
        });

        statements
    }

    fn maybe_uninit_inner<'b>(&self, ty: &'b Ty) -> Option<&'b Ty> {
        let named = ty.as_named()?;
        let path = named.canonical_path();
        let base = path.split('<').next().unwrap_or(path.as_str()).trim();
        let short = base.rsplit("::").next().unwrap_or(base);
        if short != "MaybeUninit" {
            return None;
        }
        for arg in named.args() {
            if let GenericArg::Type(inner) = arg {
                return Some(inner);
            }
        }
        None
    }

    fn is_primitive_name(&self, name: &str) -> bool {
        self.layouts
            .primitive_registry
            .descriptors()
            .iter()
            .any(|desc| {
                desc.primitive_name == name || desc.aliases.iter().any(|alias| alias == name)
            })
    }

    fn ty_needs_drop(&mut self, ty: &Ty) -> bool {
        if let Ty::Named(name) = ty {
            if pointer_depth(name.as_str()) == 0 {
                if let Some(cached) = self.needs_drop_cache.get(name.as_str()) {
                    return *cached;
                }
                let mut needs = self.layouts.ty_requires_drop(ty);
                if !needs {
                    if let Some(layout) = self.layout_for(name) {
                        // Prefer the discovered layout even when the canonical lookup missed; dispose
                        // means we must emit a drop sequence and field types may also require it.
                        if layout.dispose.is_some()
                            || layout
                                .fields
                                .iter()
                                .any(|field| self.layouts.ty_requires_drop(&field.ty))
                        {
                            needs = true;
                        }
                    } else if !self.is_primitive_name(name) {
                        // Conservatively assume user-defined types may require drops even without full layout metadata.
                        needs = true;
                    }
                }
                self.needs_drop_cache
                    .insert(name.as_str().to_string(), needs);
                return needs;
            }
        }
        self.layouts.ty_requires_drop(ty)
    }

    fn layout_for(&self, name: &str) -> Option<StructLayout> {
        if let Some(layout) = self.layouts.types.get(name) {
            return match layout {
                TypeLayout::Struct(layout) | TypeLayout::Class(layout) => Some(layout.clone()),
                _ => None,
            };
        }
        let short = name.rsplit("::").next().unwrap_or(name);
        let mut candidate: Option<StructLayout> = None;
        for (key, layout) in &self.layouts.types {
            let key_short = key.rsplit("::").next().unwrap_or(key);
            if key_short == short {
                if candidate.is_some() {
                    // Ambiguous short name; require fully qualified match.
                    return None;
                }
                match layout {
                    TypeLayout::Struct(layout) | TypeLayout::Class(layout) => {
                        candidate = Some(layout.clone())
                    }
                    _ => {}
                }
            }
        }
        candidate
    }

    fn has_dispose(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Named(name) => self
                .layout_for(name.as_str())
                .and_then(|layout| layout.dispose)
                .is_some(),
            _ => false,
        }
    }
}

pub(crate) fn synthesise_drop_statements(
    layouts: &TypeLayoutTable,
    place: Place,
    ty: Ty,
    span: Option<Span>,
    block_id: BlockId,
    force: bool,
) -> Vec<Statement> {
    let mut lowering = DropLowering::new(layouts);
    lowering.build_drop_sequence(place, ty, span, block_id, force)
}

body_builder_impl! {
    pub(super) fn apply_drop_lowering(&mut self) {
        let layouts = &*self.type_layouts;
        let locals_snapshot = self.locals.clone();
        let mut lowering = DropLowering::new(layouts);
        lowering.process_blocks(&mut self.blocks, &locals_snapshot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_lowering_emits_field_drops_in_reverse_order() {
        let mut layouts = TypeLayoutTable::default();
        layouts.types.insert(
            "Wrapper".into(),
            TypeLayout::Struct(StructLayout {
                name: "Wrapper".into(),
                repr: TypeRepr::Default,
                packing: None,
                fields: vec![
                    FieldLayout {
                        name: "First".into(),
                        ty: Ty::String,
                        index: 0,
                        offset: None,
                        span: None,
                        mmio: None,
                        display_name: None,
                        is_required: false,
                        is_nullable: false,
                        is_readonly: false,
                        view_of: None,
                    },
                    FieldLayout {
                        name: "Second".into(),
                        ty: Ty::String,
                        index: 1,
                        offset: None,
                        span: None,
                        mmio: None,
                        display_name: None,
                        is_required: false,
                        is_nullable: false,
                        is_readonly: false,
                        view_of: None,
                    },
                ],
                positional: Vec::new(),
                list: None,
                size: None,
                align: None,
                is_readonly: false,
                is_intrinsic: false,
                allow_cross_inline: false,
                auto_traits: AutoTraitSet::all_unknown(),
                overrides: AutoTraitOverride::default(),
                mmio: None,
                dispose: None,
                class: None,
            }),
        );

        let mut block = BasicBlock::new(BlockId(0), None);
        block.statements.push(Statement {
            span: None,
            kind: StatementKind::DeferDrop {
                place: Place::new(LocalId(0)),
            },
        });
        block.statements.push(Statement {
            span: None,
            kind: StatementKind::StorageDead(LocalId(0)),
        });
        block.terminator = Some(Terminator::Return);

        let locals = vec![LocalDecl::new(
            Some("wrapper".into()),
            Ty::named("Wrapper"),
            true,
            None,
            LocalKind::Local,
        )];

        let mut lowering = DropLowering::new(&layouts);
        lowering.process_block(&mut block, &locals);

        let mut kinds: Vec<(&'static str, Vec<u32>)> = Vec::new();
        for stmt in &block.statements {
            match &stmt.kind {
                StatementKind::Drop { place, .. } if place.local == LocalId(0) => {
                    let indices = place
                        .projection
                        .iter()
                        .map(|elem| match elem {
                            ProjectionElem::Field(index) => *index,
                            other => panic!("unexpected projection in test: {other:?}"),
                        })
                        .collect();
                    kinds.push(("drop", indices));
                }
                StatementKind::Deinit(place) if place.local == LocalId(0) => {
                    let indices = place
                        .projection
                        .iter()
                        .map(|elem| match elem {
                            ProjectionElem::Field(index) => *index,
                            other => panic!("unexpected projection in test: {other:?}"),
                        })
                        .collect();
                    kinds.push(("deinit", indices));
                }
                StatementKind::StorageDead(LocalId(0)) => {
                    kinds.push(("storage_dead", Vec::new()));
                }
                StatementKind::DeferDrop { .. } => panic!("defer_drop should be lowered"),
                _ => {}
            }
        }

        assert_eq!(
            kinds,
            vec![
                ("drop", vec![1]),
                ("drop", vec![0]),
                ("drop", vec![]),
                ("storage_dead", vec![]),
            ],
            "drop lowering should emit drops for each field in reverse order"
        );
    }

    #[test]
    fn drop_lowering_handles_pointer_deref() {
        let mut layouts = TypeLayoutTable::default();
        layouts.types.insert(
            "Payload".into(),
            TypeLayout::Struct(StructLayout {
                name: "Payload".into(),
                repr: TypeRepr::Default,
                packing: None,
                fields: vec![FieldLayout {
                    name: "Value".into(),
                    ty: Ty::String,
                    index: 0,
                    offset: None,
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                }],
                positional: Vec::new(),
                list: None,
                size: None,
                align: None,
                is_readonly: false,
                is_intrinsic: false,
                allow_cross_inline: false,
                auto_traits: AutoTraitSet::all_unknown(),
                overrides: AutoTraitOverride::default(),
                mmio: None,
                dispose: None,
                class: None,
            }),
        );

        let mut block = BasicBlock::new(BlockId(0), None);
        block.statements.push(Statement {
            span: None,
            kind: StatementKind::DeferDrop {
                place: Place {
                    local: LocalId(0),
                    projection: vec![ProjectionElem::Deref],
                },
            },
        });
        block.statements.push(Statement {
            span: None,
            kind: StatementKind::StorageDead(LocalId(0)),
        });
        block.terminator = Some(Terminator::Return);

        let locals = vec![LocalDecl::new(
            Some("ptr".into()),
            Ty::named("Payload*"),
            true,
            None,
            LocalKind::Local,
        )];

        let mut lowering = DropLowering::new(&layouts);
        lowering.process_block(&mut block, &locals);

        let drops: Vec<Vec<ProjectionElem>> = block
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::Drop { place, .. } => Some(place.projection.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            drops.len(),
            2,
            "expected pointer deref to produce two drops"
        );
        assert_eq!(
            drops[0],
            vec![ProjectionElem::Deref, ProjectionElem::Field(0)],
            "first drop should target the pointee field"
        );
        assert_eq!(
            drops[1],
            vec![ProjectionElem::Deref],
            "final drop should target the dereferenced payload"
        );
    }

    #[test]
    fn drop_lowering_drops_views_before_owners() {
        let mut layouts = TypeLayoutTable::default();
        layouts.types.insert(
            "HasView".into(),
            TypeLayout::Struct(StructLayout {
                name: "HasView".into(),
                repr: TypeRepr::Default,
                packing: None,
                fields: vec![
                    FieldLayout {
                        name: "View".into(),
                        ty: Ty::String,
                        index: 0,
                        offset: None,
                        span: None,
                        mmio: None,
                        display_name: None,
                        is_required: false,
                        is_nullable: false,
                        is_readonly: false,
                        view_of: Some("Data".into()),
                    },
                    FieldLayout {
                        name: "Data".into(),
                        ty: Ty::String,
                        index: 1,
                        offset: None,
                        span: None,
                        mmio: None,
                        display_name: None,
                        is_required: false,
                        is_nullable: false,
                        is_readonly: false,
                        view_of: None,
                    },
                ],
                positional: Vec::new(),
                list: None,
                size: None,
                align: None,
                is_readonly: false,
                is_intrinsic: false,
                allow_cross_inline: false,
                auto_traits: AutoTraitSet::all_unknown(),
                overrides: AutoTraitOverride::default(),
                mmio: None,
                dispose: None,
                class: None,
            }),
        );

        let mut block = BasicBlock::new(BlockId(0), None);
        block.statements.push(Statement {
            span: None,
            kind: StatementKind::DeferDrop {
                place: Place::new(LocalId(0)),
            },
        });
        block.statements.push(Statement {
            span: None,
            kind: StatementKind::StorageDead(LocalId(0)),
        });
        block.terminator = Some(Terminator::Return);

        let locals = vec![LocalDecl::new(
            Some("value".into()),
            Ty::named("HasView"),
            true,
            None,
            LocalKind::Local,
        )];

        let mut lowering = DropLowering::new(&layouts);
        lowering.process_block(&mut block, &locals);

        let drops: Vec<Vec<ProjectionElem>> = block
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::Drop { place, .. } => Some(place.projection.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            drops,
            vec![
                vec![ProjectionElem::Field(0)],
                vec![ProjectionElem::Field(1)],
                Vec::new()
            ],
            "view field must be dropped before its owning field"
        );
    }

    #[test]
    fn drop_lowering_avoids_recursing_into_maybe_uninit() {
        let layouts = TypeLayoutTable::default();
        let mut block = BasicBlock::new(BlockId(0), None);
        block.statements.push(Statement {
            span: None,
            kind: StatementKind::DeferDrop {
                place: Place::new(LocalId(0)),
            },
        });
        block.statements.push(Statement {
            span: None,
            kind: StatementKind::StorageDead(LocalId(0)),
        });
        block.terminator = Some(Terminator::Return);

        let slot_ty = Ty::named_generic(
            "Std::Memory::MaybeUninit",
            vec![GenericArg::Type(Ty::named("Demo::DropMe"))],
        );
        let locals = vec![LocalDecl::new(
            Some("slot".into()),
            slot_ty,
            true,
            None,
            LocalKind::Local,
        )];

        let mut lowering = DropLowering::new(&layouts);
        lowering.process_block(&mut block, &locals);

        let drops: Vec<&Statement> = block
            .statements
            .iter()
            .filter(|stmt| matches!(stmt.kind, StatementKind::Drop { .. }))
            .collect();
        assert_eq!(
            drops.len(),
            1,
            "MaybeUninit drop should lower to a single drop statement"
        );
        if let StatementKind::Drop { place, .. } = &drops[0].kind {
            assert!(
                place.projection.is_empty(),
                "drop of MaybeUninit should not project into payload"
            );
        }
    }
}
