use super::*;
use crate::mir::async_types::task_result_ty;
use crate::mir::builder::support::resolve_type_layout_name;
use crate::mir::{
    ArcTy, ArrayTy, FnTy, NamedTy, PointerTy, RcTy, ReadOnlySpanTy, RefTy, SpanTy, TupleTy, VecTy,
    VectorTy,
};
use std::collections::HashSet;

body_builder_impl! {
    pub(crate) fn finish(
        mut self,
    ) -> (
        MirBody,
        Vec<LoweringDiagnostic>,
        Vec<TypeConstraint>,
        Vec<MirFunction>,
    ) {
        self.ensure_entry_block();
        self.ensure_final_terminator();
        self.finalize_opaque_return();
        self.record_return_constraint();
        self.initialise_return_slot();
        self.finalize_async_state();
        self.finalize_generator_state();
        self.body.vectorize_decimal = self.vectorize_decimal;
        self.apply_drop_lowering();
        self.canonicalize_local_types();
        self.collect_borrow_escape_constraints();
        self.body.locals = std::mem::take(&mut self.locals);
        self.body.blocks = std::mem::take(&mut self.blocks);
        self.body.exception_regions = std::mem::take(&mut self.exception_regions);
        self.emit_label_diagnostics();
        self.emit_pending_goto_diagnostics();
        self.body.effects = self.collect_effects();
        for effect in &self.body.effects {
            self.constraints.push(TypeConstraint::new(
                ConstraintKind::EffectEscape {
                    function: self.function_name.clone(),
                    effect: effect.canonical_name(),
                },
                self.body.span,
            ));
        }
        let diagnostics = self.diagnostics;
        let constraints = self.constraints;
        let nested = std::mem::take(&mut self.nested_functions);
        (self.body, diagnostics, constraints, nested)
    }

    fn canonicalize_local_types(&mut self) {
        let mut locals = std::mem::take(&mut self.locals);
        for local in &mut locals {
            local.ty = self.canonicalize_ty(&local.ty);
            local.is_nullable = matches!(local.ty, Ty::Nullable(_));
            self.ensure_ty_layout_for_ty(&local.ty);
        }
        self.locals = locals;
        if let Some(async_ty) = self.async_result_ty.clone() {
            self.async_result_ty = Some(self.canonicalize_ty(&async_ty));
        }
        self.return_type = self.canonicalize_ty(&self.return_type);
        let mut streams = std::mem::take(&mut self.body.stream_metadata);
        for stream in &mut streams {
            if let Some(mem_space) = stream.mem_space.as_mut() {
                *mem_space = self.canonicalize_ty(mem_space);
            }
        }
        self.body.stream_metadata = streams;
    }

    fn canonicalize_ty(&self, ty: &Ty) -> Ty {
        match ty {
            Ty::Named(named) => {
                if named.args.is_empty() && self.generic_param_index.contains_key(&named.name) {
                    return ty.clone();
                }
                let mut name = named.name.replace('.', "::");
                if name == "Self" {
                    if let Some(self_type) = self.current_self_type_name() {
                        name = self_type;
                    }
                }
                let resolved = resolve_type_layout_name(
                    self.type_layouts,
                    Some(self.import_resolver),
                    self.namespace.as_deref(),
                    self.current_self_type_name().as_deref(),
                    &name,
                )
                .or_else(|| {
                    self.type_layouts
                        .resolve_type_key(&name)
                        .map(|key| key.to_string())
                })
                .unwrap_or(name);
                let args = named
                    .args
                    .iter()
                    .map(|arg| match arg {
                        GenericArg::Type(inner) => GenericArg::Type(self.canonicalize_ty(inner)),
                        GenericArg::Const(value) => GenericArg::Const(value.clone()),
                    })
                    .collect();
                Ty::Named(NamedTy::with_args(resolved, args))
            }
            Ty::Array(array) => Ty::Array(ArrayTy {
                element: Box::new(self.canonicalize_ty(&array.element)),
                rank: array.rank,
            }),
            Ty::Vec(vec) => Ty::Vec(VecTy {
                element: Box::new(self.canonicalize_ty(&vec.element)),
            }),
            Ty::Span(span) => Ty::Span(SpanTy {
                element: Box::new(self.canonicalize_ty(&span.element)),
            }),
            Ty::ReadOnlySpan(span) => Ty::ReadOnlySpan(ReadOnlySpanTy {
                element: Box::new(self.canonicalize_ty(&span.element)),
            }),
            Ty::Rc(rc) => Ty::Rc(RcTy {
                element: Box::new(self.canonicalize_ty(&rc.element)),
            }),
            Ty::Arc(arc) => Ty::Arc(ArcTy {
                element: Box::new(self.canonicalize_ty(&arc.element)),
            }),
            Ty::Pointer(pointer) => Ty::Pointer(Box::new(PointerTy {
                element: self.canonicalize_ty(&pointer.element),
                mutable: pointer.mutable,
                qualifiers: pointer.qualifiers.clone(),
            })),
            Ty::Ref(reference) => Ty::Ref(Box::new(RefTy {
                element: self.canonicalize_ty(&reference.element),
                readonly: reference.readonly,
            })),
            Ty::Vector(vector) => Ty::Vector(VectorTy {
                element: Box::new(self.canonicalize_ty(&vector.element)),
                lanes: vector.lanes,
            }),
            Ty::Tuple(tuple) => Ty::Tuple(TupleTy {
                elements: tuple
                    .elements
                    .iter()
                    .map(|elem| self.canonicalize_ty(elem))
                    .collect(),
                element_names: tuple.element_names.clone(),
            }),
            Ty::Fn(fn_ty) => Ty::Fn(FnTy {
                params: fn_ty
                    .params
                    .iter()
                    .map(|param| self.canonicalize_ty(param))
                    .collect(),
                param_modes: fn_ty.param_modes.clone(),
                ret: Box::new(self.canonicalize_ty(&fn_ty.ret)),
                abi: fn_ty.abi.clone(),
                variadic: fn_ty.variadic,
            }),
            Ty::Nullable(inner) => Ty::Nullable(Box::new(self.canonicalize_ty(inner))),
            Ty::TraitObject(_) | Ty::String | Ty::Str | Ty::Unit | Ty::Unknown => ty.clone(),
        }
    }

    fn ensure_entry_block(&mut self) {
        if self.blocks.is_empty() {
            let block = BasicBlock::new(BlockId(0), self.body.span);
            self.blocks.push(block);
        }
    }

    fn initialise_return_slot(&mut self) {
        if !self.is_async {
            return;
        }
        if matches!(self.return_type, Ty::Unit) {
            return;
        }
        if let Some(ret) = self.locals.get_mut(0) {
            ret.is_nullable = true;
            ret.mutable = true;
        }
        if let Some(entry) = self.blocks.get_mut(0) {
            let place = Place::new(LocalId(0));
            entry.statements.insert(
                0,
                MirStatement {
                    span: self.body.span,
                    kind: MirStatementKind::Assign {
                        place,
                        value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Null))),
                    },
                },
            );
        }
        if self.is_async && self.suspend_points.is_empty() {
            let has_inner_future = task_result_ty(&self.return_type).is_some();
            let result_local = if has_inner_future {
                self.ensure_async_result_local(self.body.span)
            } else {
                None
            };
            if let Some(entry) = self.blocks.get_mut(0) {
                if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
                    eprintln!(
                        "[chic-debug] finalize_return_slot: synthesising ready task in entry block for {} using local {:?}",
                        self.function_name, result_local
                    );
                }
                let ready_flags = Operand::Const(ConstOperand::new(ConstValue::UInt(3)));
                let bool_true = Operand::Const(ConstOperand::new(ConstValue::Bool(true)));
                // Ret.Header.Flags = READY|COMPLETED
                let mut header_flags = Place::new(LocalId(0));
                header_flags
                    .projection
                    .push(ProjectionElem::FieldNamed("Header".into()));
                header_flags
                    .projection
                    .push(ProjectionElem::FieldNamed("Flags".into()));
                entry.statements.push(MirStatement {
                    span: self.body.span,
                    kind: MirStatementKind::Assign {
                        place: header_flags,
                        value: Rvalue::Use(ready_flags.clone()),
                    },
                });
                // Ret.Flags = READY|COMPLETED
                let mut task_flags = Place::new(LocalId(0));
                task_flags
                    .projection
                    .push(ProjectionElem::FieldNamed("Flags".into()));
                entry.statements.push(MirStatement {
                    span: self.body.span,
                    kind: MirStatementKind::Assign {
                        place: task_flags,
                        value: Rvalue::Use(ready_flags.clone()),
                    },
                });
                if has_inner_future {
                    if let Some(result_local) = result_local {
                        // Ret.InnerFuture.Header.Flags = READY|COMPLETED
                        let mut inner_header_flags = Place::new(LocalId(0));
                        inner_header_flags
                            .projection
                            .push(ProjectionElem::FieldNamed("InnerFuture".into()));
                        inner_header_flags
                            .projection
                            .push(ProjectionElem::FieldNamed("Header".into()));
                        inner_header_flags
                            .projection
                            .push(ProjectionElem::FieldNamed("Flags".into()));
                        entry.statements.push(MirStatement {
                            span: self.body.span,
                            kind: MirStatementKind::Assign {
                                place: inner_header_flags,
                                value: Rvalue::Use(ready_flags.clone()),
                            },
                        });
                        // Ret.InnerFuture.Completed = true
                        let mut completed_place = Place::new(LocalId(0));
                        completed_place
                            .projection
                            .push(ProjectionElem::FieldNamed("InnerFuture".into()));
                        completed_place
                            .projection
                            .push(ProjectionElem::FieldNamed("Completed".into()));
                        entry.statements.push(MirStatement {
                            span: self.body.span,
                            kind: MirStatementKind::Assign {
                                place: completed_place,
                                value: Rvalue::Use(bool_true),
                            },
                        });
                        // Ret.InnerFuture.Result = async_result
                        let mut result_place = Place::new(LocalId(0));
                        result_place
                            .projection
                            .push(ProjectionElem::FieldNamed("InnerFuture".into()));
                        result_place
                            .projection
                            .push(ProjectionElem::FieldNamed("Result".into()));
                        entry.statements.push(MirStatement {
                            span: self.body.span,
                            kind: MirStatementKind::Assign {
                                place: result_place,
                                value: Rvalue::Use(Operand::Copy(Place::new(result_local))),
                            },
                        });
                    }
                }
            }
        }
    }

    fn ensure_final_terminator(&mut self) {
        let mut return_block: Option<BlockId> = None;
        for idx in 0..self.blocks.len() {
            if self.blocks[idx].terminator.is_none() {
                if matches!(self.function_kind, FunctionKind::Testcase) {
                    let bool_true = Operand::Const(ConstOperand::new(ConstValue::Bool(true)));
                    if self.is_async {
                        if let Some(result_local) = self.ensure_async_result_local(self.body.span) {
                            self.blocks[idx].statements.push(MirStatement {
                                span: self.body.span,
                                kind: MirStatementKind::Assign {
                                    place: Place::new(result_local),
                                    value: Rvalue::Use(bool_true),
                                },
                            });
                        }
                    } else {
                        self.blocks[idx].statements.push(MirStatement {
                            span: self.body.span,
                            kind: MirStatementKind::Assign {
                                place: Place::new(LocalId(0)),
                                value: Rvalue::Use(bool_true),
                            },
                        });
                    }
                }
                let target = return_block.unwrap_or_else(|| {
                    let id = BlockId(self.blocks.len());
                    let mut block = BasicBlock::new(id, self.body.span);
                    block.terminator = Some(Terminator::Return);
                    self.blocks.push(block);
                    return_block = Some(id);
                    id
                });
                self.blocks[idx].terminator = Some(Terminator::Goto { target });
            }
        }
        if return_block.is_none()
            && self
                .blocks
                .last()
                .and_then(|block| block.terminator.as_ref())
                .is_none()
        {
            let last = self.blocks.len() - 1;
            self.blocks[last].terminator = Some(Terminator::Return);
        }
    }

    fn record_return_constraint(&mut self) {
        let constraint_ty = if self.is_async {
            self.async_result_ty.clone()
        } else if matches!(self.return_type, Ty::Named(_)) {
            Some(self.return_type.clone())
        } else {
            None
        };

        if let Some(ret_ty) = constraint_ty {
            self.constraints.push(TypeConstraint::new(
                ConstraintKind::ReturnType {
                    function: self.function_name.clone(),
                    ty: ret_ty.canonical_name(),
                },
                self.body.span,
            ));
        }
    }

    fn emit_label_diagnostics(&mut self) {
        for (label, state) in &self.labels {
            if !state.defined {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!("label `{label}` is undefined"),
                    span: state.span,
                                    });
            }
        }
    }

    fn emit_pending_goto_diagnostics(&mut self) {
        for (label, pendings) in &self.pending_gotos {
            for pending in pendings {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!("`goto {label}` references an undefined label"),
                    span: pending.span,
                                    });
            }
        }
    }

    fn collect_effects(&self) -> Vec<Ty> {
        let mut seen = HashSet::new();
        let mut effects = Vec::new();
        for block in &self.body.blocks {
            if let Some(Terminator::Throw { ty, .. }) = &block.terminator {
                let effect_ty = ty.clone().unwrap_or_else(|| Ty::named("Exception"));
                let key = effect_ty.canonical_name();
                if seen.insert(key.clone()) {
                    effects.push(effect_ty);
                }
            }
        }
        effects
    }

    fn collect_borrow_escape_constraints(&mut self) {
        let mut assignments: Vec<(Place, Vec<(Place, AssignmentSourceKind)>, Option<Span>)> =
            Vec::new();

        for block in &self.blocks {
            for statement in &block.statements {
                let MirStatementKind::Assign { place, value } = &statement.kind else {
                    continue;
                };

                let mut sources = Vec::new();
                self.collect_places_from_rvalue(value, &mut sources);
                if sources.is_empty() {
                    continue;
                }

                let cloned_sources = sources
                    .into_iter()
                    .map(|(place, kind)| (place.clone(), kind))
                    .collect::<Vec<_>>();
                assignments.push((place.clone(), cloned_sources, statement.span));
            }
        }

        for (destination, sources, span) in assignments {
            let mut recorded: Vec<LocalId> = Vec::new();
            for (source, kind) in sources {
                if recorded.iter().any(|existing| *existing == source.local) {
                    continue;
                }
                self.record_borrow_escape_from_assignment(&destination, &source, kind, span);
                recorded.push(source.local);
            }
        }
    }

    fn finalize_opaque_return(&mut self) {
        let Some(info) = self.opaque_return.as_ref() else {
            return;
        };
        if info.inferred.is_none() {
            self.diagnostics.push(LoweringDiagnostic {
                message: "unable to infer concrete type for `impl Trait` return".into(),
                span: info.declared_span.or(self.body.span),
            });
        }
    }
}
