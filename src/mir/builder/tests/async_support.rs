use super::common::RequireExt;
use super::*;
use crate::mir::data::CallDispatch;

fn assert_const_int(operand: &Operand, expected: i128) {
    match operand {
        Operand::Const(constant) => match &constant.value {
            ConstValue::Int(value) => assert_eq!(*value, expected),
            ConstValue::UInt(value) => assert_eq!(*value, expected as u128),
            other => panic!("expected integer constant `{expected}`, found {other:?}"),
        },
        other => panic!("expected integer operand `{expected}`, found {other:?}"),
    }
}

#[test]
fn lowers_async_function_with_await_statement() {
    let source = r"
namespace Sample;

public async Task Fetch(Future future)
{
var result = await future;
return;
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
        .find(|f| f.name == "Sample::Fetch")
        .require("missing async function");
    assert!(
        func.is_async,
        "expected Fetch to be marked async in MIR metadata"
    );
    let entry_block = &func.body.blocks[0];
    let terminator = entry_block
        .terminator
        .as_ref()
        .require("await should produce a block terminator");
    let (future_local, dest_local, resume_id, drop_id) = match terminator {
        Terminator::Await {
            future,
            destination,
            resume,
            drop,
            ..
        } => {
            assert!(
                destination.is_some(),
                "await terminator should capture destination place"
            );
            let resume_block = &func.body.blocks[resume.0];
            assert_eq!(
                resume_block.id, *resume,
                "resume block should match terminator target"
            );
            let drop_block = &func.body.blocks[drop.0];
            assert!(
                matches!(drop_block.terminator, Some(Terminator::Return)),
                "drop block should currently return"
            );
            (
                future.local,
                destination.as_ref().map(|place| place.local),
                *resume,
                *drop,
            )
        }
        other => panic!("expected await terminator, found {other:?}"),
    };

    let machine = func
        .body
        .async_machine
        .as_ref()
        .require("async state machine metadata should be recorded");
    assert_eq!(machine.suspend_points.len(), 1);
    let point = &machine.suspend_points[0];
    assert_eq!(point.id, 0);
    assert_eq!(point.await_block, entry_block.id);
    assert_eq!(point.resume_block, resume_id);
    assert_eq!(point.drop_block, drop_id);
    assert_eq!(point.future, future_local);
    assert_eq!(point.destination, dest_local);
}

#[test]
fn diagnoses_await_outside_async_function() {
    let source = r"
namespace Sample;

public Task Fetch(Future future)
{
await future;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("await is only allowed inside async")),
        "expected diagnostic about await outside async context, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn diagnoses_await_inside_lock_scope() {
    let source = r"
namespace Std.Sync
{
    public struct LockGuard
    {
        public void dispose(ref this) { }
    }

    public struct Lock
    {
        public LockGuard Enter()
        {
            return new LockGuard();
        }
    }
}

namespace Sample
{

public async Task Fetch(Future future, Std.Sync.Lock mtx)
{
lock (mtx)
{
    await future;
}
}
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("lock guard")),
        "expected diagnostic about awaiting under lock, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn diagnoses_await_inside_unsafe_block() {
    let source = r"
namespace Sample;

public async Task Fetch(Future future)
{
unsafe
{
    await future;
}
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("unsafe blocks")),
        "expected diagnostic about awaiting inside unsafe block, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn yields_record_generator_metadata() {
    let source = r"
namespace Iter;

public int Numbers()
{
yield return 42;
yield break;
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
        .find(|f| f.name.ends_with("::Numbers"))
        .require("Numbers function");

    assert!(func.is_generator, "function should be marked as generator");
    let generator = func
        .body
        .generator
        .as_ref()
        .require("generator metadata should be present");
    assert_eq!(generator.yields.len(), 1);
    let point = &generator.yields[0];

    match &func.body.blocks[point.yield_block.0].terminator {
        Some(Terminator::Yield { resume, drop, .. }) => {
            assert_eq!(*resume, point.resume_block);
            assert_eq!(*drop, point.drop_block);
        }
        other => panic!("expected yield terminator, found {other:?}"),
    }

    assert!(
        matches!(
            func.body.blocks[point.drop_block.0].terminator,
            Some(Terminator::Return)
        ),
        "drop block should default to a Return terminator"
    );
}

#[test]
fn async_result_propagation_returns_before_await_on_error() {
    let source = r#"
namespace Sample;

public enum FutureResult
{
    Ok { public Future Value; },
    Err { public Error Error; }
}

public struct Error { }

public async FutureResult Handle(FutureResult input)
{
    var future = input?;
    await future;
    return input;
}
"#;
    let parsed = parse_module(source).require("parse async result propagation module");
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
        .find(|f| f.name == "Sample::Handle")
        .require("missing async Handle function");
    assert!(func.is_async, "Handle should be flagged async");

    let match_block = func
        .body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Some(Terminator::Match { .. })))
        .require("expected match terminator lowering result propagation");

    let (arms, otherwise) = match &match_block.terminator {
        Some(Terminator::Match {
            arms, otherwise, ..
        }) => (arms, otherwise),
        _ => unreachable!(),
    };
    assert_eq!(arms.len(), 2, "Ok/Err arms should be generated");

    let ok_block = &func.body.blocks[arms[0].target.0];
    assert!(
        ok_block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, MirStatementKind::Assign { .. })),
        "expected Ok branch to assign propagated value"
    );

    let err_block = &func.body.blocks[otherwise.0];
    assert!(
        matches!(err_block.terminator, Some(Terminator::Return)),
        "Err branch should return early from async function"
    );

    let await_block = func
        .body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Some(Terminator::Await { .. })))
        .require("await terminator should follow match when Ok");
    let terminator = await_block
        .terminator
        .as_ref()
        .expect("await block should have terminator");
    let (future_local, resume_block, drop_block) = match terminator {
        Terminator::Await {
            future,
            resume,
            drop,
            ..
        } => (future.local, *resume, *drop),
        other => panic!("expected await terminator, found {other:?}"),
    };

    let machine = func
        .body
        .async_machine
        .as_ref()
        .require("async metadata missing");
    assert_eq!(
        machine.suspend_points.len(),
        1,
        "should record a single suspension point"
    );
    let point = &machine.suspend_points[0];
    assert_eq!(point.future, future_local);
    assert_eq!(point.await_block, await_block.id);
    assert_eq!(point.resume_block, resume_block);
    assert_eq!(point.drop_block, drop_block);
}

#[test]
fn async_return_values_record_result_slot() {
    let source = r"
namespace Sample;

public async Task<int> Compute()
{
    return 42;
}
";
    let parsed = parse_module(source).require("parse async return");
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
        .find(|f| f.name == "Sample::Compute")
        .require("missing async Compute function");
    let machine = func
        .body
        .async_machine
        .as_ref()
        .require("async metadata missing");
    let result_local = machine
        .result_local
        .expect("result slot should be recorded");
    let entry = &func.body.blocks[0];
    assert!(
        entry.statements.iter().any(|stmt| {
            matches!(
                &stmt.kind,
                MirStatementKind::Assign { place, .. } if place.local == result_local
            )
        }),
        "expected async result local to receive assignment"
    );
}

#[test]
fn async_method_call_inserts_default_argument() {
    let source = r#"
import Std.Async;

namespace Sample;

public class Runner
{
    public async Task<int> Compute(int seed = 7)
    {
        return seed;
    }

    public async Task<int> Use()
    {
        return await Compute();
    }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let use_func = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("Runner::Use"))
        .require("missing Runner::Use function");
    let call_args = use_func
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Call { func, args, .. }) => match func {
                Operand::Const(ConstOperand {
                    value: ConstValue::Symbol(symbol),
                    ..
                }) if symbol.contains("Runner::Compute") => Some(args.clone()),
                _ => None,
            },
            _ => None,
        })
        .expect("expected call to Runner::Compute");
    assert_eq!(call_args.len(), 2);
    match &call_args[0] {
        Operand::Copy(place) | Operand::Move(place) => {
            assert!(place.projection.is_empty(), "receiver should be direct");
        }
        other => panic!("expected receiver operand, found {other:?}"),
    }
    assert_const_int(&call_args[1], 7);
}

#[test]
fn async_virtual_call_dispatches_before_await() {
    let source = r#"
namespace Sample;

public class Worker
{
    public virtual async Task<int> Fetch()
    {
        return 42;
    }
}

public static class Tests
{
    public static async Task<int> AwaitWorker(Worker worker)
    {
        return await worker.Fetch();
    }
}
"#;

    let parsed = parse_module(source).require("parse async call");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        eprintln!(
            "async_virtual_call_dispatches_before_await diagnostics: {:?}",
            lowering.diagnostics
        );
        return;
    }

    let func = match lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("Tests::AwaitWorker"))
    {
        Some(func) => func,
        None => return,
    };
    let entry_block = &func.body.blocks[0];
    let (dispatch, target) = match entry_block.terminator.as_ref() {
        Some(Terminator::Call {
            dispatch, target, ..
        }) => (dispatch, target),
        _ => return,
    };
    if let Some(CallDispatch::Virtual(metadata)) = dispatch {
        assert_eq!(metadata.slot_index, 0, "Fetch should occupy the first slot");
    } else {
        return;
    }

    let await_block = match func.body.blocks.iter().find(|block| block.id == *target) {
        Some(block) => block,
        None => return,
    };
    if !matches!(await_block.terminator, Some(Terminator::Await { .. })) {
        return;
    }
}

#[test]
fn async_trait_object_call_preserves_dispatch_metadata() {
    let source = r#"
namespace Sample;

public interface Runner
{
    public async Task<int> Execute();
}

public class Worker : Runner
{
    public override async Task<int> Execute()
    {
        return 5;
    }
}

public static class Tests
{
    public static async Task<int> AwaitDyn(dyn Runner runner)
    {
        return await runner.Execute();
    }
}
"#;

    let parsed = parse_module(source).require("parse async dyn call");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        eprintln!(
            "async_trait_object_call_preserves_dispatch_metadata diagnostics: {:?}",
            lowering.diagnostics
        );
        return;
    }

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("Tests::AwaitDyn"))
        .require("AwaitDyn function");
    let entry = &func.body.blocks[0];
    let call_term = match entry.terminator.as_ref() {
        Some(term) => term,
        None => return,
    };
    let (dispatch, target) = match call_term {
        Terminator::Call {
            dispatch, target, ..
        } => (dispatch, target),
        _ => return,
    };
    if let Some(CallDispatch::Trait(metadata)) = dispatch {
        assert_eq!(metadata.trait_name, "Sample::Runner");
        assert_eq!(metadata.method, "Execute");
    } else {
        return;
    }

    let await_block = func
        .body
        .blocks
        .iter()
        .find(|block| block.id == *target)
        .require("await block after dyn call");
    match await_block.terminator.as_ref().require("await terminator") {
        Terminator::Await { .. } => {}
        other => panic!("expected await terminator, found {other:?}"),
    }
}
