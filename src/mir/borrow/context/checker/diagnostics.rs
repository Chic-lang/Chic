use super::*;
use crate::diagnostics::{DiagnosticCode, Label, Suggestion};
use crate::mir::data::{BorrowId, LocalId, Place};

impl<'a> BorrowChecker<'a> {
    pub(super) fn report_error(
        &mut self,
        location: Location,
        span: Option<Span>,
        kind: ErrorKeyKind,
        message: impl Into<String>,
    ) {
        let key = match location {
            Location::Statement { block, index } => ErrorKey {
                block: block.0,
                statement_index: Some(index),
                kind,
            },
            Location::Terminator { block } => ErrorKey {
                block: block.0,
                statement_index: None,
                kind,
            },
        };

        if !self.reported.insert(key) {
            return;
        }

        self.diagnostics.push(Diagnostic::error(
            format!("{}: {}", self.function.name, message.into()),
            span,
        ));
    }

    pub(super) fn report_warning(
        &mut self,
        location: Location,
        span: Option<Span>,
        kind: ErrorKeyKind,
        message: impl Into<String>,
    ) {
        let key = match location {
            Location::Statement { block, index } => ErrorKey {
                block: block.0,
                statement_index: Some(index),
                kind,
            },
            Location::Terminator { block } => ErrorKey {
                block: block.0,
                statement_index: None,
                kind,
            },
        };

        if !self.reported.insert(key) {
            return;
        }

        self.diagnostics.push(Diagnostic::warning(
            format!("{}: {}", self.function.name, message.into()),
            span,
        ));
    }

    pub(super) fn emit_immutable_binding_error(
        &mut self,
        location: Location,
        primary_span: Option<Span>,
        local: LocalId,
        action: &str,
    ) {
        let key = match location {
            Location::Statement { block, index } => ErrorKey {
                block: block.0,
                statement_index: Some(index),
                kind: ErrorKeyKind::ImmutableAssignment(local),
            },
            Location::Terminator { block } => ErrorKey {
                block: block.0,
                statement_index: None,
                kind: ErrorKeyKind::ImmutableAssignment(local),
            },
        };

        if !self.reported.insert(key) {
            return;
        }

        let decl_span = self.function.body.local(local).and_then(|decl| decl.span);
        let name = self.local_name(local);
        let mut diagnostic = Diagnostic::error(
            format!("cannot {action} immutable binding `{name}`"),
            primary_span.or(decl_span),
        )
        .with_code(DiagnosticCode::new("LCL0002", Some("typeck".to_string())));

        if let Some(span) = primary_span.or(decl_span) {
            diagnostic.primary_label = Some(Label::primary(
                span,
                format!("cannot {action} immutable binding `{name}`"),
            ));
        }
        if let Some(span) = decl_span {
            diagnostic = diagnostic.with_secondary(Label::secondary(
                span,
                "binding declared here as immutable with `let`",
            ));
            let end = span.start.saturating_add(3).min(span.end);
            if end > span.start {
                diagnostic.add_suggestion(Suggestion::new(
                    format!("change `let {name} ...` to `var {name} ...` if this binding needs to be mutable"),
                    Some(Span::in_file(span.file_id, span.start, end)),
                    Some("var".into()),
                ));
            } else {
                diagnostic.add_suggestion(Suggestion::new(
                    format!("use `var` instead of `let` for `{name}` if it must be mutable"),
                    None,
                    None,
                ));
            }
        } else {
            diagnostic.add_suggestion(Suggestion::new(
                format!("use `var` instead of `let` for `{name}` if it must be mutable"),
                None,
                None,
            ));
        }

        self.diagnostics.push(diagnostic);
    }

    pub(super) fn register_region(&mut self, region: RegionVar, borrow: BorrowId, start: Location) {
        self.regions
            .entry(region)
            .and_modify(|info| {
                if !info.loans.contains(&borrow) {
                    info.loans.push(borrow);
                }
            })
            .or_insert_with(|| RegionInfo {
                start,
                end: None,
                loans: vec![borrow],
            });
    }

    pub(super) fn close_region(&mut self, region: RegionVar, borrow: BorrowId, end: Location) {
        self.regions
            .entry(region)
            .and_modify(|info| {
                info.end = Some(end);
                if !info.loans.contains(&borrow) {
                    info.loans.push(borrow);
                }
            })
            .or_insert_with(|| RegionInfo {
                start: end,
                end: Some(end),
                loans: vec![borrow],
            });
    }

    pub(super) fn release_loans_for_place(
        &mut self,
        state: &mut BorrowState<'a>,
        place: &Place,
        location: Location,
    ) {
        for loan in state.remove_loans_for_place(place) {
            self.close_region(loan.info.region, loan.info.borrow_id, location);
        }
    }

    pub(super) fn release_view(
        &mut self,
        state: &mut BorrowState<'a>,
        view: LocalId,
        location: Location,
    ) {
        for loan in state.remove_loans_for_view(view) {
            self.close_region(loan.info.region, loan.info.borrow_id, location);
        }
    }

    pub(super) fn release_event_loans(
        &mut self,
        state: &mut BorrowState<'a>,
        event: LocalId,
        location: Location,
    ) {
        for loan in state.remove_loans_for_event(event) {
            self.close_region(loan.info.region, loan.info.borrow_id, location);
        }
    }

    pub(super) fn place_is_pinned(state: &BorrowState<'a>, place: &Place) -> bool {
        state.local_facts(place.local).is_pinned()
    }

    pub(super) fn local_name(&self, local: LocalId) -> String {
        self.function
            .body
            .local(local)
            .and_then(|decl| decl.name.clone())
            .map_or_else(|| format!("{local}"), |name| name)
    }

    pub(super) fn place_description(&self, place: &Place) -> String {
        if place.projection.is_empty() {
            self.local_name(place.local)
        } else {
            format!("{}<?>", self.local_name(place.local))
        }
    }
}

pub(super) fn format_span(span: Option<Span>) -> String {
    span.map_or_else(
        || "unknown span".into(),
        |s| format!("{}..{}", s.start, s.end),
    )
}
