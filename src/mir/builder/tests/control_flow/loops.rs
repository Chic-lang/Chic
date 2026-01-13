use super::prelude::*;
use std::sync::{Mutex, MutexGuard, OnceLock};

fn cross_inline_override_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn loops_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn loops_test_guard() -> MutexGuard<'static, ()> {
    loops_test_lock().lock().expect("loops test lock")
}

struct CrossInlineOverrideGuard;

impl Drop for CrossInlineOverrideGuard {
    fn drop(&mut self) {
        test_clear_cross_inline_overrides();
    }
}

#[test]
fn lowers_while_loop() {
    let _loop_guard = loops_test_guard();
    let source = r"
namespace Control;

public void Loop()
{
var count = 3;
while (count > 0)
{
    count -= 1;
}
}
";
    let parsed = parse_module(source).require("parse");
    let _override_lock = cross_inline_override_lock()
        .lock()
        .expect("cross-inline override lock");
    let _override_guard = CrossInlineOverrideGuard;
    let span_name = Ty::Span(SpanTy::new(Box::new(Ty::named("int")))).canonical_name();
    test_set_cross_inline_override(&span_name, true);
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );
    let func = &lowering.module.functions[0];
    let body = &func.body;
    let graph = GraphAssert::new(body);
    graph.expect_goto(0);
    let switch = graph.expect_switch(1);
    switch.expect_target_count(1).assert_distinct_otherwise();
}

#[test]
fn lowers_foreach_span_uses_intrinsic_stack_iter() {
    let _loop_guard = loops_test_guard();
    let source = r"
namespace Control;

public int Sum(Span<int> values)
{
var total = 0;
foreach (var value in values)
{
    total += value;
}
return total;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Sum"))
        .require("missing Sum function");
    let body = &func.body;
    assert_no_pending(body);
    let graph = GraphAssert::new(body);

    let has_enumerator_local = body.locals.iter().any(|decl| {
        decl.name
            .as_deref()
            .is_some_and(|name| name.starts_with("__foreach_enum_"))
    });
    assert!(
        !has_enumerator_local,
        "intrinsic foreach should not allocate enumerator locals"
    );

    let idx_local = body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| {
            decl.name
                .as_deref()
                .is_some_and(|name| name.starts_with("__foreach_idx_local_"))
        })
        .map(|(idx, _)| LocalId(idx))
        .require("expected synthesized foreach index local");
    let len_local = body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| {
            decl.name
                .as_deref()
                .is_some_and(|name| name.starts_with("__foreach_len_local_"))
        })
        .map(|(idx, _)| LocalId(idx))
        .require("expected synthesized foreach length local");
    let cond_local = body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| {
            decl.name
                .as_deref()
                .is_some_and(|name| name.starts_with("__foreach_cond_local_"))
        })
        .map(|(idx, _)| LocalId(idx))
        .require("expected synthesized foreach condition local");

    let span_value_local = body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| decl.name.as_deref() == Some("value"))
        .map(|(idx, _)| LocalId(idx))
        .require("expected span iteration variable");

    let has_len_assign = body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .any(|stmt| {
            matches!(
                stmt.kind,
                MirStatementKind::Assign {
                    value: Rvalue::Len(_),
                    ..
                }
            )
        });
    assert!(
        has_len_assign,
        "intrinsic foreach should compute sequence length"
    );

    let has_index_projection = body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .any(|stmt| match &stmt.kind {
            MirStatementKind::Assign { value, .. } => match value {
                Rvalue::Use(Operand::Copy(place)) | Rvalue::Use(Operand::Move(place)) => {
                    place.projection.iter().any(
                        |elem| matches!(elem, ProjectionElem::Index(local) if *local == idx_local),
                    )
                }
                _ => false,
            },
            _ => false,
        });
    assert!(
        has_index_projection,
        "intrinsic foreach should index into the span using the synthesized index local"
    );

    let cond_from_lt = body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .any(|stmt| {
            matches!(
                &stmt.kind,
                MirStatementKind::Assign {
                    place,
                    value: Rvalue::Binary {
                        op: BinOp::Lt,
                        lhs: Operand::Copy(lhs),
                        rhs: Operand::Copy(rhs),
                        ..
                    },
                    ..
                } if place.local == cond_local && lhs.local == idx_local && rhs.local == len_local
            )
        });
    assert!(
        cond_from_lt,
        "foreach condition should compare the index against the cached length"
    );

    let cond_block_index = body
        .blocks
        .iter()
        .enumerate()
        .find(|(_, block)| matches!(block.terminator, Some(Terminator::SwitchInt { .. })))
        .map(|(idx, _)| idx)
        .expect("missing foreach condition block");
    let switch = graph.expect_switch(cond_block_index);
    switch.expect_target_count(1);
    match switch.discr() {
        Operand::Copy(place) | Operand::Move(place) => assert_eq!(
            place.local, cond_local,
            "foreach should branch on the synthesized condition local"
        ),
        other => {
            panic!("expected foreach discriminator to borrow the condition local, found {other:?}")
        }
    };

    let item_dead_blocks: Vec<_> = body
        .blocks
        .iter()
        .filter_map(|block| {
            block
                .statements
                .iter()
                .any(|stmt| matches!(stmt.kind, MirStatementKind::StorageDead(local) if local == span_value_local))
                .then_some(block.id)
        })
        .collect();
    assert!(
        item_dead_blocks.len() >= 2,
        "iteration variable should be dropped in cleanup paths (found blocks {item_dead_blocks:?})"
    );
}

#[test]
fn lowers_foreach_span_of_disposables_drops_iteration_value_before_storage_dead() {
    let _loop_guard = loops_test_guard();
    let source = r"
namespace Control;

public struct Disposable
{
public void dispose(ref this)
{
}
}

public void Consume(Span<Disposable> values)
{
foreach (var value in values)
{
}
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Consume"))
        .require("missing Consume function");
    let body = &func.body;
    assert_no_pending(body);

    let disposable_value_local = body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| decl.name.as_deref() == Some("value"))
        .map(|(idx, _)| LocalId(idx))
        .require("expected foreach iteration variable local");
    let label = format!("Disposable iteration variable {disposable_value_local:?}");
    assert_drop_sequence(body, disposable_value_local, &label, false);
}

#[test]
fn lowers_foreach_span_respects_cross_inline_opt_out() {
    let _loop_guard = loops_test_guard();
    let _override_lock = cross_inline_override_lock()
        .lock()
        .expect("cross-inline override lock");
    let _override_guard = CrossInlineOverrideGuard;
    let span_name = Ty::Span(SpanTy::new(Box::new(Ty::named("int")))).canonical_name();
    test_set_cross_inline_override(&span_name, false);

    let source = r#"
namespace Control;

public int Sum(Span<int> values)
{
var total = 0;
foreach (var value in values)
{
    total += value;
}
return total;
}
"#;
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Sum"))
        .require("missing Sum function");
    let body = &func.body;
    assert_no_pending(body);

    let has_enumerator_local = body.locals.iter().any(|decl| {
        decl.name
            .as_deref()
            .is_some_and(|name| name.starts_with("__foreach_enum_"))
    });
    assert!(
        has_enumerator_local,
        "cross-inline opt-out should force foreach to allocate an enumerator"
    );

    let synthesized_stack_locals = body.locals.iter().filter(|decl| {
        decl.name.as_deref().is_some_and(|name| {
            name.starts_with("__foreach_idx_local_")
                || name.starts_with("__foreach_len_local_")
                || name.starts_with("__foreach_cond_local_")
        })
    });
    assert!(
        synthesized_stack_locals.count() == 0,
        "stack iterator locals should not be synthesized when cross-inline is disabled"
    );
}

#[test]
fn lowers_foreach_user_enumerator_uses_state_machine() {
    let _loop_guard = loops_test_guard();
    let source = r"
namespace Control;

public struct CustomEnumerator
{
private int _value;
public bool MoveNext()
{
    _value += 1;
    return _value < 4;
}

public int Current => _value;
}

public struct CustomSequence
{
public CustomEnumerator GetEnumerator()
{
    return new CustomEnumerator();
}
}

public int Sum(CustomSequence values)
{
var total = 0;
foreach (var value in values)
{
    total += value;
}
return total;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Sum"))
        .require("missing Sum function");
    let body = &func.body;
    assert_no_pending(body);

    let enumerator_local = body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| {
            decl.name
                .as_deref()
                .is_some_and(|name| name.starts_with("__foreach_enum_"))
        })
        .map(|(idx, _)| LocalId(idx))
        .require("expected enumerator local");

    let has_idx_local = body.locals.iter().any(|decl| {
        decl.name
            .as_deref()
            .is_some_and(|name| name.starts_with("__foreach_idx_local_"))
    });
    assert!(
        !has_idx_local,
        "non-intrinsic foreach should not synthesize stack iterator locals"
    );

    let has_enumerator_live = body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .any(|stmt| matches!(stmt.kind, MirStatementKind::StorageLive(local) if local == enumerator_local));
    assert!(
        has_enumerator_live,
        "expected StorageLive for enumerator local"
    );

    assert_no_defer_drop(body);

    let enumerator_dead = storage_dead_index(body, enumerator_local).is_some();
    assert!(enumerator_dead, "expected StorageDead for enumerator local");

    let Some(cond_block_idx) = body
        .blocks
        .iter()
        .enumerate()
        .find(|(_, block)| matches!(block.terminator, Some(Terminator::SwitchInt { .. })))
        .map(|(idx, _)| idx)
    else {
        panic!("expected SwitchInt terminator driving MoveNext loop condition");
    };
    let graph = GraphAssert::new(body);
    let switch = graph.expect_switch(cond_block_idx);
    switch.expect_target_count(1);

    let enumerator_label = format!("foreach enumerator {enumerator_local:?}");
    assert_drop_sequence(body, enumerator_local, &enumerator_label, false);
}

#[test]
fn lowers_foreach_ref_binding_records_unique_borrow_category() {
    let _loop_guard = loops_test_guard();
    let source = r"
namespace Control;

public void Adjust(ref Span<int> values)
{
foreach (ref var value in values)
{
    value = value + 1;
}
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Adjust"))
        .require("missing Adjust function");
    let body = &func.body;

    let mut borrow_seen = false;
    let mut borrow_targets = Vec::new();

    let value_local = body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| decl.name.as_deref() == Some("value"))
        .map(|(idx, _)| LocalId(idx))
        .require("value local id");

    for stmt in body.blocks.iter().flat_map(|block| block.statements.iter()) {
        match &stmt.kind {
            MirStatementKind::Borrow { kind, .. } => {
                assert_eq!(
                    *kind,
                    BorrowKind::Unique,
                    "expected unique borrow for foreach ref binding"
                );
                borrow_seen = true;
            }
            MirStatementKind::Assign { place, value, .. } => {
                if matches!(value, Rvalue::Use(Operand::Borrow(_))) {
                    borrow_targets.push(place.local);
                }
            }
            _ => {}
        }
    }

    assert!(
        borrow_seen,
        "expected borrow statement for foreach ref binding"
    );
    assert!(
        borrow_targets.iter().all(|local| *local == value_local),
        "borrow for foreach ref binding should only assign to the iteration variable"
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "integration-style MIR lowering test requires full control-flow fixture"
)]
fn lowers_goto_label_emits_storage_dead_for_exited_scope() {
    let _loop_guard = loops_test_guard();
    let source = r"
namespace Flow;

public int Compute(bool flag)
{
{
    var value = 42;
    if (flag)
    {
        goto exit;
    }
    value = 7;
}
exit:
return 0;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Compute"))
        .require("Compute function");
    let body = &func.body;

    let value_local = body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| decl.name.as_deref() == Some("value"))
        .map(|(idx, _)| LocalId(idx))
        .require("value local id");

    let goto_block = body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Some(Terminator::Goto { .. })))
        .require("goto block");

    let has_dead = goto_block.statements.iter().any(|stmt| {
        matches!(
            stmt.kind,
            MirStatementKind::StorageDead(local) if local == value_local
        )
    });
    assert!(
        has_dead,
        "expected StorageDead for scoped variable before goto"
    );
}

#[test]
fn reports_undefined_label_reference() {
    let _loop_guard = loops_test_guard();
    let source = r"
namespace Flow;

public void Jump()
{
goto missing;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("undefined")),
        "expected undefined label diagnostic"
    );
}

#[test]
fn reports_duplicate_labels() {
    let _loop_guard = loops_test_guard();
    let source = r"
namespace Flow;

public void Jump()
{
exit:
goto exit;
exit:
return;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("duplicate label")),
        "expected duplicate label diagnostic"
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "integration-style MIR lowering test requires full control-flow fixture"
)]
fn lowers_while_with_break_and_continue_into_gotos() {
    let _loop_guard = loops_test_guard();
    let source = r"
namespace Control;

public void Flow(int limit)
{
var i = limit;

while (i > 0)
{
    if (i == 5)
    {
        break;
    }

    if (i == 3)
    {
        continue;
    }

    i -= 1;
}
}
";
    let parsed = parse_module(source).require("parse");
    let (cond_span, break_span, continue_span) = extract_while_spans(&parsed.module, "Flow");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );
    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Flow"))
        .require("missing Flow function");
    let body = &func.body;
    assert_no_pending(body);
    let graph = GraphAssert::new(body);

    let cond_block_id =
        find_block_with_span(body, cond_span).require("missing while condition block");
    let switch = graph.expect_switch(cond_block_id.0);
    switch.expect_target_count(1).assert_distinct_otherwise();
    let exit_block_id = switch.otherwise();

    let break_block_id =
        find_block_with_span(body, break_span).require("missing break branch block");
    let break_target = graph.expect_goto(break_block_id.0);
    assert_eq!(
        break_target, exit_block_id,
        "break branch should exit the loop"
    );

    let continue_block_id =
        find_block_with_span(body, continue_span).require("missing continue branch block");
    let continue_target = graph.expect_goto(continue_block_id.0);
    assert_eq!(
        continue_target, cond_block_id,
        "continue branch should jump to loop condition"
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "nested try/catch loop regression needs a fully inlined fixture"
)]
fn while_loop_with_nested_try_catch_preserves_edges() {
    let _loop_guard = loops_test_guard();
    let source = r#"
namespace Control;

public class Exception { }

public int Process(int limit)
{
    var index = 0;
    while (index < limit)
    {
        if (index == 5)
        {
            try
            {
                try
                {
                    break;
                }
                catch (Exception inner)
                {
                    return 1;
                }
            }
            catch (Exception mid)
            {
                return 2;
            }
        }

        if (index == 2)
        {
            try
            {
                try
                {
                    continue;
                }
                catch (Exception retry)
                {
                    continue;
                }
            }
            catch (Exception mid)
            {
                continue;
            }
        }

        try
        {
            index += 1;
        }
        catch (Exception outer)
        {
            return -1;
        }
    }

    return index;
}
"#;
    let parsed = parse_module(source).require("parse");
    let (cond_span, break_span, continue_span) = extract_while_spans(&parsed.module, "Process");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );
    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Process"))
        .require("missing Process function");
    let body = &func.body;
    assert!(
        body.exception_regions.len() >= 4,
        "while loop should materialize nested try/catch regions"
    );
    assert_no_pending(body);

    let graph = GraphAssert::new(body);
    let cond_block_id =
        find_block_with_span(body, cond_span).require("missing nested while condition block");
    let switch = graph.expect_switch(cond_block_id.0);
    switch.expect_target_count(1);
    let exit_block_id = switch.otherwise();

    let break_block_id =
        find_block_with_span(body, break_span).require("missing nested break block");
    assert_eq!(
        graph.expect_goto(break_block_id.0),
        exit_block_id,
        "break inside nested try/catch should jump to exit"
    );

    let continue_block_id =
        find_block_with_span(body, continue_span).require("missing nested continue block");
    assert_eq!(
        graph.expect_goto(continue_block_id.0),
        cond_block_id,
        "continue inside nested try/catch should jump to condition"
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Integration-style lowering test needs full control-flow fixture for clarity"
)]
fn lowers_for_loop_with_break_and_continue_targets() {
    let _loop_guard = loops_test_guard();
    let source = r"
namespace Control;

public int Sum(int limit)
{
var total = 0;

for (var idx = 0; idx < limit; idx += 1)
{
    if (idx == 5)
    {
        break;
    }

    if (idx == 2)
    {
        continue;
    }

    total += idx;
}

return total;
}
";
    let parsed = match parse_module(source) {
        Ok(module) => module,
        Err(err) => panic!("parse failed: {err:?}"),
    };
    let (cond_span, iterator_span, break_span, continue_span) =
        extract_for_spans(&parsed.module, "Sum");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );
    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Sum"))
        .unwrap_or_else(|| panic!("missing Sum function"));
    let body = &func.body;
    assert_no_pending(body);
    let graph = GraphAssert::new(body);

    let Some(cond_block_id) = find_block_with_span(body, cond_span) else {
        panic!("missing for condition block");
    };
    let switch = graph.expect_switch(cond_block_id.0);
    switch.expect_target_count(1);
    let exit_block_id = switch.otherwise();

    let Some(iter_block_id) = find_block_with_statement_span(body, iterator_span) else {
        panic!("missing iterator block");
    };

    let Some(break_block_id) = find_block_with_span(body, break_span) else {
        panic!("missing break branch block");
    };
    let break_target = graph.expect_goto(break_block_id.0);
    assert_eq!(
        break_target, exit_block_id,
        "for-loop break should jump to exit"
    );

    let Some(continue_block_id) = find_block_with_span(body, continue_span) else {
        panic!("missing continue branch block");
    };
    let continue_target = graph.expect_goto(continue_block_id.0);
    assert_eq!(
        continue_target, iter_block_id,
        "continue should jump to iterator block"
    );
}
