use super::super::*;
use crate::mir::AutoTraitStatus;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssignmentSourceKind {
    Value,
    Borrow,
}

body_builder_impl! {
    pub(crate) fn maybe_move_operand_for_value_assignment(&self, operand: Operand) -> Operand {
        let Operand::Copy(place) = operand else {
            return operand;
        };
        if !place.projection.is_empty() {
            return Operand::Copy(place);
        }
        if self.borrow_param_mode(place.local).is_some() {
            return Operand::Copy(place);
        }
        let Some(ty) = self.place_ty(&place) else {
            return Operand::Copy(place);
        };
        if matches!(ty, Ty::Unknown) {
            return Operand::Copy(place);
        }
        if self.type_layouts.ty_requires_drop(&ty) {
            return Operand::Move(place);
        }
        let traits = self.type_layouts.resolve_auto_traits(&ty.canonical_name());
        if matches!(traits.copy, AutoTraitStatus::Yes) {
            Operand::Copy(place)
        } else {
            Operand::Move(place)
        }
    }

    pub(crate) fn place_assignment_kind(&self, place: &Place) -> AssignmentSourceKind {
        if place.projection.is_empty() && self.borrow_param_mode(place.local).is_some() {
            AssignmentSourceKind::Borrow
        } else {
            AssignmentSourceKind::Value
        }
    }

    pub(crate) fn borrow_param_mode(&self, local: LocalId) -> Option<ParamMode> {
        self.locals
            .get(local.0)
            .and_then(|decl| decl.param_mode)
            .and_then(|mode| match mode {
                ParamMode::In | ParamMode::Ref | ParamMode::Out => Some(mode),
                ParamMode::Value => None,
            })
    }

    pub(crate) fn describe_place(&self, place: &Place) -> String {
        let base = self
            .locals
            .get(place.local.0)
            .and_then(|decl| decl.name.clone())
            .unwrap_or_else(|| format!("_{}", place.local.0));

        if place.projection.is_empty() {
            return base;
        }

        let mut description = base;
        for elem in &place.projection {
            match elem {
                ProjectionElem::Field(index) => {
                    description.push('.');
                    description.push_str(&format!("{index}"));
                }
                ProjectionElem::FieldNamed(name) => {
                    description.push('.');
                    description.push_str(name);
                }
                ProjectionElem::UnionField { name, .. } => {
                    description.push('.');
                    description.push_str(name);
                }
                ProjectionElem::Index(local) => {
                    description.push('[');
                    let label = self
                        .locals
                        .get(local.0)
                        .and_then(|decl| decl.name.clone())
                        .unwrap_or_else(|| format!("_{}", local.0));
                    description.push_str(&label);
                    description.push(']');
                }
                ProjectionElem::ConstantIndex { offset, .. } => {
                    description.push('[');
                    description.push_str(&offset.to_string());
                    description.push(']');
                }
                ProjectionElem::Deref => {
                    description.push_str(".*");
                }
                ProjectionElem::Downcast { variant } => {
                    description.push_str(&format!("#{}", variant));
                }
                ProjectionElem::Subslice { from, to } => {
                    description.push('[');
                    description.push_str(&from.to_string());
                    description.push_str("..");
                    description.push_str(&to.to_string());
                    description.push(']');
                }
            }
        }

        description
    }

    pub(crate) fn collect_places_from_operand<'b>(&'b self, operand: &'b Operand, out: &mut Vec<(&'b Place, AssignmentSourceKind)>) {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                let kind = self.place_assignment_kind(place);
                out.push((place, kind))
            }
            Operand::Borrow(borrow) => {
                out.push((&borrow.place, AssignmentSourceKind::Borrow))
            }
            Operand::Mmio(_) | Operand::Const(_) | Operand::Pending(_) => {}
        }
    }

    pub(crate) fn collect_places_from_rvalue<'b>(
        &'b self,
        value: &'b Rvalue,
        out: &mut Vec<(&'b Place, AssignmentSourceKind)>,
    ) {
        match value {
            Rvalue::Use(operand) => self.collect_places_from_operand(operand, out),
            Rvalue::Unary { operand, .. } => self.collect_places_from_operand(operand, out),
            Rvalue::Binary { lhs, rhs, .. } => {
                self.collect_places_from_operand(lhs, out);
                self.collect_places_from_operand(rhs, out);
            }
            Rvalue::Aggregate { fields, .. } => {
                for field in fields {
                    self.collect_places_from_operand(field, out);
                }
            }
            Rvalue::AddressOf { place, .. } | Rvalue::Len(place) => {
                let kind = self.place_assignment_kind(place);
                out.push((place, kind));
            }
            Rvalue::SpanStackAlloc { length, source, .. } => {
                self.collect_places_from_operand(length, out);
                if let Some(source) = source {
                    self.collect_places_from_operand(source, out);
                }
            }
            Rvalue::Cast { operand, .. } => self.collect_places_from_operand(operand, out),
            Rvalue::StringInterpolate { segments } => {
                for segment in segments {
                    if let InterpolatedStringSegment::Expr { operand, .. } = segment {
                        self.collect_places_from_operand(operand, out);
                    }
                }
            }
            Rvalue::NumericIntrinsic(intrinsic) => {
                for operand in &intrinsic.operands {
                    self.collect_places_from_operand(operand, out);
                }
                if let Some(out_place) = &intrinsic.out {
                    let kind = self.place_assignment_kind(out_place);
                    out.push((out_place, kind));
                }
            }
            Rvalue::DecimalIntrinsic(intrinsic) => {
                self.collect_places_from_operand(&intrinsic.lhs, out);
                self.collect_places_from_operand(&intrinsic.rhs, out);
                if let Some(addend) = &intrinsic.addend {
                    self.collect_places_from_operand(addend, out);
                }
                self.collect_places_from_operand(&intrinsic.rounding, out);
                self.collect_places_from_operand(&intrinsic.vectorize, out);
            }
            Rvalue::AtomicLoad { target, .. } => {
                let kind = self.place_assignment_kind(target);
                out.push((target, kind));
            }
            Rvalue::AtomicRmw { target, value, .. } => {
                let kind = self.place_assignment_kind(target);
                out.push((target, kind));
                self.collect_places_from_operand(value, out);
            }
            Rvalue::AtomicCompareExchange {
                target,
                expected,
                desired,
                ..
            } => {
                let kind = self.place_assignment_kind(target);
                out.push((target, kind));
                self.collect_places_from_operand(expected, out);
                self.collect_places_from_operand(desired, out);
            }
            Rvalue::StaticLoad { .. } => {}
            Rvalue::StaticRef { .. } => {}
            Rvalue::Pending(_) => {}
        }
    }

    pub(crate) fn operand_to_place(&mut self, operand: Operand, span: Option<Span>) -> Place {
        match operand {
            Operand::Move(place) | Operand::Copy(place) => place,
            Operand::Borrow(borrow) => borrow.place,
            Operand::Const(_) | Operand::Pending(_) | Operand::Mmio(_) => {
                let temp = self.create_temp(span);
                Place::new(temp)
            }
        }
    }

    pub(crate) fn prepare_call_destination(
        &mut self,
        destination: Option<Place>,
        span: Option<Span>,
    ) -> (Place, Option<LocalId>) {
        match destination {
            Some(place) => (place, None),
            None => {
                let temp = self.create_temp(span);
                (Place::new(temp), Some(temp))
            }
        }
    }

    pub(crate) fn record_borrow_capture_constraints(
        &mut self,
        captures: &[CapturedLocal],
        span: Option<Span>,
        closure_ty: &str,
    ) {
        for capture in captures {
            let Some(mode) = self.borrow_param_mode(capture.local) else {
                continue;
            };
            let param_name = self
                .locals
                .get(capture.local.0)
                .and_then(|decl| decl.name.clone())
                .unwrap_or_else(|| format!("_{}", capture.local.0));
            self.constraints.push(TypeConstraint::new(
                ConstraintKind::BorrowEscape {
                    function: self.function_name.clone(),
                    parameter: param_name,
                    parameter_mode: mode,
                    escape: BorrowEscapeCategory::Capture {
                        closure: closure_ty.to_string(),
                    },
                },
                span,
            ));
        }
    }

    pub(crate) fn record_borrow_escape_from_assignment(
        &mut self,
        destination: &Place,
        source: &Place,
        source_kind: AssignmentSourceKind,
        span: Option<Span>,
    ) {
        if !matches!(source_kind, AssignmentSourceKind::Borrow) {
            return;
        }

        let Some(mode) = self.borrow_param_mode(source.local) else {
            return;
        };

        if destination.local == source.local {
            return;
        }

        let Some(dest_decl) = self.locals.get(destination.local.0) else {
            return;
        };

        if destination.projection.is_empty() && matches!(dest_decl.ty, Ty::Ref(_)) {
            return;
        }

        if dest_decl.kind == LocalKind::Temp && destination.projection.is_empty() {
            return;
        }

        let param_name = self
            .locals
            .get(source.local.0)
            .and_then(|decl| decl.name.clone())
            .unwrap_or_else(|| format!("_{}", source.local.0));

        let escape = if dest_decl.kind == LocalKind::Return && destination.projection.is_empty() {
            if self.lending_return_params.contains(&source.local) {
                return;
            }
            BorrowEscapeCategory::Return
        } else {
            let target = self.describe_place(destination);
            BorrowEscapeCategory::Store { target }
        };

        self.constraints.push(TypeConstraint::new(
            ConstraintKind::BorrowEscape {
                function: self.function_name.clone(),
                parameter: param_name,
                parameter_mode: mode,
                escape,
            },
            span,
        ));
    }
}
