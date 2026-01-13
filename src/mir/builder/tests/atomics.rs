use super::common::RequireExt;
use super::*;
use crate::mir::{AtomicFenceScope, AtomicOrdering, AtomicRmwOp};

fn lower_sample_module() -> LoweringResult {
    let source = r#"
namespace Std.Sync
{
    public enum MemoryOrder
    {
        Relaxed,
        Acquire,
        Release,
        AcqRel,
        SeqCst,
    }

    public struct AtomicInt
    {
        public int Load(MemoryOrder order)
        {
            return 0;
        }

        public void Store(int value, MemoryOrder order)
        {
        }

        public int FetchAdd(int value, MemoryOrder order)
        {
            return value;
        }

        public bool CompareExchange(
            int expected,
            int desired,
            MemoryOrder success,
            MemoryOrder failure
        )
        {
            return true;
        }
    }

    public static class Fences
    {
        public static void Fence(MemoryOrder order) { }
    }
}

namespace Sample
{
    public class Counter
    {
        public int LoadValue()
        {
            var cell = new Std.Sync.AtomicInt();
            return cell.Load(Std.Sync.MemoryOrder.Acquire);
        }

        public void StoreValue(int value)
        {
            var cell = new Std.Sync.AtomicInt();
            cell.Store(value, Std.Sync.MemoryOrder.Release);
        }

        public int FetchAdd()
        {
            var cell = new Std.Sync.AtomicInt();
            return cell.FetchAdd(1, Std.Sync.MemoryOrder.AcqRel);
        }

        public bool TryUpdate(int expected, int desired)
        {
            var cell = new Std.Sync.AtomicInt();
            return cell.CompareExchange(
                expected,
                desired,
                Std.Sync.MemoryOrder.SeqCst,
                Std.Sync.MemoryOrder.Relaxed
            );
        }

        public void Fence()
        {
            Std.Sync.Fences.Fence(Std.Sync.MemoryOrder.SeqCst);
        }

        public void Block()
        {
            var cell = new Std.Sync.AtomicInt();
            atomic(Std.Sync.MemoryOrder.AcqRel)
            {
                cell.Store(42, Std.Sync.MemoryOrder.Release);
            }
        }
    }
}
"#;
    let parsed = parse_module(source).require("parse");
    lower_module(&parsed.module)
}

fn find_function<'a>(result: &'a LoweringResult, name: &str) -> &'a MirFunction {
    result
        .module
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing function `{name}`"))
}

#[test]
fn atomic_load_and_store_lower_to_atomic_ops() {
    let lowering = lower_sample_module();
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let load_fn = find_function(&lowering, "Sample::Counter::LoadValue");
    let load_stmt = load_fn
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .find_map(|stmt| match &stmt.kind {
            StatementKind::Assign { value, .. } => match value {
                Rvalue::AtomicLoad { target, order } => Some((target.clone(), *order)),
                _ => None,
            },
            _ => None,
        })
        .expect("expected AtomicLoad rvalue in LoadValue");
    assert_eq!(load_stmt.1, AtomicOrdering::Acquire);

    let store_fn = find_function(&lowering, "Sample::Counter::StoreValue");
    let store_stmt = store_fn
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .find(|stmt| matches!(stmt.kind, StatementKind::AtomicStore { .. }))
        .expect("expected AtomicStore statement in StoreValue");
    if let StatementKind::AtomicStore { order, .. } = store_stmt.kind {
        assert_eq!(order, AtomicOrdering::Release);
    }
}

#[test]
fn atomic_rmw_and_compare_exchange_lowered() {
    let lowering = lower_sample_module();
    let fetch_fn = find_function(&lowering, "Sample::Counter::FetchAdd");
    let fetch_assign = fetch_fn
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .find_map(|stmt| match &stmt.kind {
            StatementKind::Assign { value, .. } => match value {
                Rvalue::AtomicRmw { op, order, .. } => Some((*op, *order)),
                _ => None,
            },
            _ => None,
        })
        .expect("expected AtomicRmw in FetchAdd lowering");
    assert_eq!(fetch_assign.0, AtomicRmwOp::Add);
    assert_eq!(fetch_assign.1, AtomicOrdering::AcqRel);

    let cas_fn = find_function(&lowering, "Sample::Counter::TryUpdate");
    let cas_assign = cas_fn
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .find_map(|stmt| match &stmt.kind {
            StatementKind::Assign { value, .. } => match value {
                Rvalue::AtomicCompareExchange {
                    success,
                    failure,
                    weak,
                    ..
                } => Some((*success, *failure, *weak)),
                _ => None,
            },
            _ => None,
        })
        .expect("expected AtomicCompareExchange in TryUpdate lowering");
    assert_eq!(cas_assign.0, AtomicOrdering::SeqCst);
    assert_eq!(cas_assign.1, AtomicOrdering::Relaxed);
    assert!(!cas_assign.2, "CompareExchange should emit strong CAS");
}

#[test]
fn atomic_fences_emitted_for_block_and_function() {
    let lowering = lower_sample_module();

    let fence_fn = find_function(&lowering, "Sample::Counter::Fence");
    let fence_stmt = fence_fn
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .find(|stmt| matches!(stmt.kind, StatementKind::AtomicFence { .. }))
        .expect("expected explicit fence lowering");
    if let StatementKind::AtomicFence { order, scope } = fence_stmt.kind {
        assert_eq!(order, AtomicOrdering::SeqCst);
        assert_eq!(scope, AtomicFenceScope::Full);
    }

    let block_fn = find_function(&lowering, "Sample::Counter::Block");
    let mut scopes = block_fn
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter_map(|stmt| {
            if let StatementKind::AtomicFence { scope, .. } = stmt.kind {
                Some(scope)
            } else {
                None
            }
        });
    assert_eq!(
        scopes.next(),
        Some(AtomicFenceScope::BlockEnter),
        "atomic block should begin with BlockEnter fence"
    );
    assert!(
        scopes.any(|scope| scope == AtomicFenceScope::BlockExit),
        "atomic block should end with BlockExit fence"
    );
}
