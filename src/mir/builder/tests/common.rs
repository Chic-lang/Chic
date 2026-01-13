use super::{
    AstStatementKind, BlockId, Item, LocalId, MirBody, MirStatementKind, Span, Terminator,
};
use crate::mir::data::Statement as MirStatement;
use std::fmt::Debug;

pub(super) trait RequireExt<T> {
    fn require(self, context: &str) -> T;
}

impl<T> RequireExt<T> for Option<T> {
    fn require(self, context: &str) -> T {
        match self {
            Some(value) => value,
            None => panic!("{context}"),
        }
    }
}

impl<T, E: Debug> RequireExt<T> for Result<T, E> {
    fn require(self, context: &str) -> T {
        match self {
            Ok(value) => value,
            Err(err) => panic!("{context}: {err:?}"),
        }
    }
}

pub(super) fn find_block_with_span(body: &MirBody, span: Span) -> Option<BlockId> {
    body.blocks
        .iter()
        .find(|block| block.span == Some(span))
        .map(|block| block.id)
}

pub(super) fn find_block_with_statement_span(body: &MirBody, span: Span) -> Option<BlockId> {
    body.blocks
        .iter()
        .find(|block| {
            block
                .statements
                .iter()
                .any(|statement| statement.span == Some(span))
        })
        .map(|block| block.id)
}

pub(super) fn assert_no_pending(body: &MirBody) {
    for block in &body.blocks {
        for statement in &block.statements {
            assert!(
                !matches!(statement.kind, MirStatementKind::Pending(_)),
                "found pending statement in block {}",
                block.id
            );
        }
        if let Some(term) = &block.terminator {
            assert!(
                !matches!(term, Terminator::Pending(_)),
                "found pending terminator in block {}",
                block.id
            );
        }
    }
}

pub(super) fn assert_no_defer_drop(body: &MirBody) {
    let has_marker = body.blocks.iter().any(|block| {
        block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, MirStatementKind::DeferDrop { .. }))
    });
    assert!(
        !has_marker,
        "drop lowering should remove DeferDrop markers from the final MIR"
    );
}

pub(super) fn first_statement_index<P>(body: &MirBody, mut predicate: P) -> Option<usize>
where
    P: FnMut(&MirStatement) -> bool,
{
    let mut index = 0usize;
    for block in &body.blocks {
        for stmt in &block.statements {
            if predicate(stmt) {
                return Some(index);
            }
            index += 1;
        }
    }
    None
}

pub(super) fn drop_index(body: &MirBody, local: LocalId) -> Option<usize> {
    first_statement_index(body, |stmt| {
        matches!(
            stmt.kind,
            MirStatementKind::Drop { ref place, .. } if place.local == local
        )
    })
}

pub(super) fn deinit_index(body: &MirBody, local: LocalId) -> Option<usize> {
    first_statement_index(body, |stmt| {
        matches!(
            stmt.kind,
            MirStatementKind::Deinit(ref place) if place.local == local
        )
    })
}

pub(super) fn storage_dead_index(body: &MirBody, local: LocalId) -> Option<usize> {
    first_statement_index(
        body,
        |stmt| matches!(stmt.kind, MirStatementKind::StorageDead(target) if target == local),
    )
}

pub(super) fn assert_drop_sequence(
    body: &MirBody,
    local: LocalId,
    label: &str,
    expect_deinit: bool,
) {
    let drop_idx = drop_index(body, local);
    let deinit_idx = deinit_index(body, local);
    let effective_drop_idx = drop_idx.or(deinit_idx).unwrap_or_else(|| {
        panic!("{label}: expected drop or deinit statement for local {local:?}")
    });
    if expect_deinit {
        let deinit_idx = deinit_idx
            .unwrap_or_else(|| panic!("{label}: expected deinit statement for local {local:?}"));
        if let Some(drop_idx) = drop_idx {
            assert!(
                deinit_idx < drop_idx,
                "{label}: deinit for local {local:?} should precede its drop"
            );
        }
    }
    if let Some(dead_idx) = storage_dead_index(body, local) {
        assert!(
            effective_drop_idx < dead_idx,
            "{label}: drop for local {local:?} should execute before StorageDead"
        );
    }
}

pub(super) fn extract_while_spans(
    module: &crate::frontend::ast::Module,
    name: &str,
) -> (Span, Span, Span) {
    for item in &module.items {
        let Item::Function(func) = item else {
            continue;
        };
        if func.name != name {
            continue;
        }
        let Some(body) = func.body.as_ref() else {
            panic!("function `{name}` missing body");
        };
        for statement in &body.statements {
            if let AstStatementKind::While {
                condition,
                body: loop_body,
            } = &statement.kind
            {
                let Some(cond_span) = condition.span else {
                    panic!("while condition span missing in `{name}`");
                };
                let AstStatementKind::Block(loop_block) = &loop_body.kind else {
                    panic!("expected block loop body in `{name}` while");
                };
                let spans: Vec<Span> = loop_block
                    .statements
                    .iter()
                    .filter_map(|stmt| match &stmt.kind {
                        AstStatementKind::If(if_stmt) => if_stmt.then_branch.span,
                        _ => None,
                    })
                    .collect();
                return match spans.as_slice() {
                    [break_span, continue_span, ..] => (cond_span, *break_span, *continue_span),
                    _ => panic!("missing break/continue spans in while `{name}`"),
                };
            }
        }
    }
    panic!("while statement not found in {name}");
}

pub(super) fn extract_for_spans(
    module: &crate::frontend::ast::Module,
    name: &str,
) -> (Span, Span, Span, Span) {
    for item in &module.items {
        let Item::Function(func) = item else {
            continue;
        };
        if func.name != name {
            continue;
        }
        let Some(body) = func.body.as_ref() else {
            panic!("function `{name}` missing body");
        };
        for statement in &body.statements {
            if let AstStatementKind::For(for_stmt) = &statement.kind {
                let Some(cond_span) = for_stmt.condition.as_ref().and_then(|expr| expr.span) else {
                    panic!("for condition span missing in `{name}`");
                };
                let Some(iterator_expr) = for_stmt.iterator.first() else {
                    panic!("for iterator missing expression in `{name}`");
                };
                let Some(iterator_span) = iterator_expr.span else {
                    panic!("for iterator span missing in `{name}`");
                };
                let AstStatementKind::Block(block) = &for_stmt.body.kind else {
                    panic!("expected block loop body in `{name}` for loop");
                };
                let spans: Vec<Span> = block
                    .statements
                    .iter()
                    .filter_map(|stmt| match &stmt.kind {
                        AstStatementKind::If(if_stmt) => if_stmt.then_branch.span,
                        _ => None,
                    })
                    .collect();
                return match spans.as_slice() {
                    [break_span, continue_span, ..] => {
                        (cond_span, iterator_span, *break_span, *continue_span)
                    }
                    _ => panic!("missing break/continue spans in for `{name}`"),
                };
            }
        }
    }
    panic!("for statement not found in {name}");
}
