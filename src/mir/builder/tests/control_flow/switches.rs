use super::prelude::*;

#[test]
fn lowers_if_else_statement() {
    let source = r"
namespace Control;

public int Check(int x)
{
if (x > 0) {
    return x;
} else {
    return -x;
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
    let func = &lowering.module.functions[0];
    let body = &func.body;
    assert_eq!(body.blocks.len(), 4);
    let graph = GraphAssert::new(body);
    graph
        .expect_switch(0)
        .expect_target_count(1)
        .assert_distinct_otherwise();
    graph.expect_return(1);
    graph.expect_return(2);
}

#[test]
fn lowers_result_propagation_into_match() {
    let source = r"
namespace Demo;

public enum IntStringResult
{
    Ok { public int Value; },
    Err { public string Error; }
}

public IntStringResult Forward(IntStringResult input)
{
    var value = input?;
    if (value > 0)
        return input;
    return input;
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
        .find(|f| f.name.ends_with("::Forward"))
        .require("missing Forward function");
    let body = &func.body;
    let graph = GraphAssert::new(body);

    let match_block_idx = body
        .blocks
        .iter()
        .position(|block| matches!(block.terminator, Some(Terminator::Match { .. })))
        .require("expected match terminator for result propagation");

    let matcher = graph.expect_match(match_block_idx);
    matcher.expect_arm_count(2);

    let ok_block = &body.blocks[matcher.arms()[0].target.0];
    assert!(
        ok_block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, MirStatementKind::Assign { .. })),
        "expected Ok branch to assign payload"
    );

    let err_block = &body.blocks[matcher.otherwise().0];
    let has_err_aggregate = err_block.statements.iter().any(|stmt| {
        if let MirStatementKind::Assign {
            value:
                Rvalue::Aggregate {
                    kind: AggregateKind::Adt { .. },
                    fields,
                },
            ..
        } = &stmt.kind
        {
            fields.len() <= 1
        } else {
            false
        }
    });
    assert!(
        has_err_aggregate,
        "expected Err branch to construct aggregate: {:?}",
        err_block.statements
    );
    assert!(
        matches!(err_block.terminator, Some(Terminator::Return)),
        "Err branch should return early"
    );
}

#[test]
fn result_propagation_in_trait_impl_lowering() {
    let source = r"
namespace Demo;

public enum WorkResult
{
    Ok,
    Err { public Error Error; }
}

public struct Error { }

public interface Worker
{
    WorkResult Run(WorkResult input);
}

public class WorkerImpl : Worker
{
    public WorkResult Run(WorkResult input)
    {
        input?;
        return WorkResult.Ok;
    }
}
";
    let parsed = parse_module(source).require("parse trait impl module");
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
        .find(|f| f.name.ends_with("WorkerImpl::Run"))
        .require("missing WorkerImpl::Run function");
    let body = &func.body;
    let graph = GraphAssert::new(body);

    let match_block_idx = body
        .blocks
        .iter()
        .position(|block| matches!(block.terminator, Some(Terminator::Match { .. })))
        .require("expected match terminator inside trait impl");
    let matcher = graph.expect_match(match_block_idx);
    matcher.expect_arm_count(2);

    let ok_block = matcher
        .arms()
        .iter()
        .map(|arm| &body.blocks[arm.target.0])
        .find(|block| matches!(block.terminator, Some(Terminator::Goto { .. })))
        .require("expected Ok branch to jump to continuation");
    assert!(
        matches!(ok_block.terminator, Some(Terminator::Goto { .. })),
        "Ok branch should fall through to continuation"
    );

    let err_block = matcher
        .arms()
        .iter()
        .map(|arm| &body.blocks[arm.target.0])
        .find(|block| matches!(block.terminator, Some(Terminator::Return)))
        .unwrap_or_else(|| &body.blocks[matcher.otherwise().0]);

    assert!(
        err_block.statements.iter().any(|stmt| matches!(
            stmt.kind,
            MirStatementKind::Assign {
                value: Rvalue::Aggregate { .. },
                ..
            }
        )),
        "Err branch should construct WorkResult.Err aggregate"
    );
}
